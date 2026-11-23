use std::collections::HashSet;

use crate::course::ast::{CourseChapterRaw, CourseDocumentRaw, CourseNode};
use crate::course::ast_text::{sanitize_heading_id, slugify_identifier};
use crate::course::section_extractor::{
    extract_sections, infer_dominant_language, infer_target_path_from_paragraph,
};
use crate::course::shell_analyzer::{
    analyze_shell_block, block_starts_with_root_reset_cd, extract_loopback_port_from_text,
    is_shell_language, parse_fence_attributes, ExplicitCommandMode, FenceAttributes,
};
use crate::course::ParseMarkdownOptions;
use crate::domain::course::{
    ChapterKind, ChapterThreeStepFlow, CommandBlock, MarkdownNode, ParsedChapter, ParsedCourse,
    ReferenceCodeBlock,
};
use crate::domain::ids::{ChapterId, CommandBlockId, RelWorkspacePath};
use crate::domain::pty::PtyRole;

const NON_REFERENCE_LANGUAGES: &[&str] = &["text", "txt", "console", "output", "plaintext"];

struct ChapterIdAllocator {
    seen: HashSet<String>,
}

impl ChapterIdAllocator {
    fn new() -> Self {
        Self {
            seen: HashSet::new(),
        }
    }

    fn allocate(
        &mut self,
        ordinal_1_based: usize,
        raw_id_attr: Option<&str>,
        title: &str,
    ) -> ChapterId {
        let base = if let Some(explicit) = raw_id_attr.and_then(sanitize_heading_id) {
            explicit
        } else {
            let slug = slugify_identifier(title, 48);
            if slug.is_empty() {
                format!("ch-{ordinal_1_based:02}")
            } else {
                format!("ch-{ordinal_1_based:02}-{slug}")
            }
        };

        if self.seen.insert(base.clone()) {
            return ChapterId(base);
        }

        let mut suffix = 2usize;
        loop {
            let candidate = format!("{base}-{suffix}");
            if self.seen.insert(candidate.clone()) {
                return ChapterId(candidate);
            }
            suffix += 1;
        }
    }
}

struct ExtractedCommand {
    block: CommandBlock,
    explicit_secondary: Option<bool>,
    is_daemon: bool,
    is_test_runner: bool,
    is_pure_cd: bool,
}

struct ChapterLowerState<'a> {
    chapter_id: &'a ChapterId,
    current_cwd: Option<RelWorkspacePath>,
    next_cmd_ordinal: usize,
    next_ref_index: u32,
    commands: Vec<ExtractedCommand>,
    reference_blocks: Vec<ReferenceCodeBlock>,
    chapter_loopback_port: Option<u16>,
}

pub(crate) fn extract_course(
    raw: CourseDocumentRaw,
    options: &ParseMarkdownOptions,
) -> ParsedCourse {
    let title = raw
        .title
        .filter(|t| !t.trim().is_empty())
        .or_else(|| {
            options
                .default_title
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "Untitled Course".to_owned());

    let course_id = options
        .course_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            let slug = slugify_identifier(&title, 64);
            if slug.is_empty() {
                "untitled-course".to_owned()
            } else {
                slug
            }
        });

    let mut all_code_languages: Vec<&str> = Vec::new();
    collect_code_languages(&raw.intro_nodes, &mut all_code_languages);
    for ch in &raw.chapters {
        collect_code_languages(&ch.nodes, &mut all_code_languages);
    }

    let language = options
        .language_hint
        .or_else(|| infer_dominant_language(&all_code_languages));

    let (intro_raw_nodes, raw_chapters) = if raw.chapters.is_empty() {
        let synthetic = CourseChapterRaw {
            raw_id_attr: None,
            title: title.clone(),
            nodes: raw.intro_nodes,
        };
        (Vec::new(), vec![synthetic])
    } else {
        (raw.intro_nodes, raw.chapters)
    };

    let intro_nodes = lower_plain_nodes(&intro_raw_nodes);

    let mut id_allocator = ChapterIdAllocator::new();
    let mut chapters = Vec::with_capacity(raw_chapters.len());

    for (index, raw_chapter) in raw_chapters.into_iter().enumerate() {
        let chapter_id = id_allocator.allocate(
            index + 1,
            raw_chapter.raw_id_attr.as_deref(),
            &raw_chapter.title,
        );
        chapters.push(extract_chapter(index as u32, chapter_id, raw_chapter));
    }

    ParsedCourse {
        course_id,
        title,
        language,
        intro_nodes,
        chapters,
    }
}

fn collect_code_languages<'a>(nodes: &'a [CourseNode], out: &mut Vec<&'a str>) {
    for node in nodes {
        match node {
            CourseNode::CodeBlock { language, .. } => {
                if let Some(lang) = language.as_deref() {
                    out.push(lang);
                }
            }
            CourseNode::Paragraph { children }
            | CourseNode::Heading { children, .. }
            | CourseNode::Strong { children }
            | CourseNode::Emphasis { children }
            | CourseNode::BlockQuote { children }
            | CourseNode::Link { children, .. }
            | CourseNode::Superscript { children }
            | CourseNode::Subscript { children }
            | CourseNode::Details { children, .. } => collect_code_languages(children, out),
            CourseNode::List { items, .. } => {
                for item in items {
                    collect_code_languages(&item.nodes, out);
                }
            }
            CourseNode::Table { headers, rows } => {
                for cell in headers {
                    collect_code_languages(cell, out);
                }
                for row in rows {
                    for cell in row {
                        collect_code_languages(cell, out);
                    }
                }
            }
            CourseNode::Text { .. }
            | CourseNode::InlineCode { .. }
            | CourseNode::Image { .. }
            | CourseNode::LineBreak
            | CourseNode::ThematicBreak
            | CourseNode::Kbd { .. } => {}
        }
    }
}

fn lower_plain_nodes(nodes: &[CourseNode]) -> Vec<MarkdownNode> {
    nodes.iter().map(lower_plain_node).collect()
}

fn lower_plain_node(node: &CourseNode) -> MarkdownNode {
    match node {
        CourseNode::Paragraph { children } => MarkdownNode::Paragraph {
            children: lower_plain_nodes(children),
        },
        CourseNode::Heading {
            level,
            id,
            children,
        } => MarkdownNode::Heading {
            level: *level,
            id: id.clone(),
            children: lower_plain_nodes(children),
        },
        CourseNode::Text { value } => MarkdownNode::Text {
            value: value.clone(),
        },
        CourseNode::Strong { children } => MarkdownNode::Strong {
            children: lower_plain_nodes(children),
        },
        CourseNode::Emphasis { children } => MarkdownNode::Emphasis {
            children: lower_plain_nodes(children),
        },
        CourseNode::InlineCode { value } => MarkdownNode::InlineCode {
            value: value.clone(),
        },
        CourseNode::CodeBlock { language, code, .. } => MarkdownNode::CodeBlock {
            language: language.clone(),
            code: code.clone(),
            target_path: None,
            command_block_id: None,
        },
        CourseNode::List {
            ordered,
            start,
            items,
        } => MarkdownNode::List {
            ordered: *ordered,
            start: *start,
            items: items
                .iter()
                .map(|item| lower_plain_nodes(&item.nodes))
                .collect(),
        },
        CourseNode::BlockQuote { children } => MarkdownNode::BlockQuote {
            children: lower_plain_nodes(children),
        },
        CourseNode::Table { headers, rows } => MarkdownNode::Table {
            headers: headers.iter().map(|c| lower_plain_nodes(c)).collect(),
            rows: rows
                .iter()
                .map(|r| r.iter().map(|c| lower_plain_nodes(c)).collect())
                .collect(),
        },
        CourseNode::Link {
            href,
            title,
            children,
        } => MarkdownNode::Link {
            href: href.clone(),
            title: title.clone(),
            children: lower_plain_nodes(children),
        },
        CourseNode::Image { src, alt, title } => MarkdownNode::Image {
            src: src.clone(),
            alt: alt.clone(),
            title: title.clone(),
        },
        CourseNode::LineBreak => MarkdownNode::LineBreak,
        CourseNode::ThematicBreak => MarkdownNode::ThematicBreak,
        CourseNode::Kbd { label } => MarkdownNode::Kbd {
            label: label.clone(),
        },
        CourseNode::Superscript { children } => MarkdownNode::Superscript {
            children: lower_plain_nodes(children),
        },
        CourseNode::Subscript { children } => MarkdownNode::Subscript {
            children: lower_plain_nodes(children),
        },
        CourseNode::Details { summary, children } => MarkdownNode::Details {
            summary: summary.clone(),
            children: lower_plain_nodes(children),
        },
    }
}

fn extract_chapter(index: u32, chapter_id: ChapterId, raw: CourseChapterRaw) -> ParsedChapter {
    let mut state = ChapterLowerState {
        chapter_id: &chapter_id,
        current_cwd: None,
        next_cmd_ordinal: 1,
        next_ref_index: 0,
        commands: Vec::new(),
        reference_blocks: Vec::new(),
        chapter_loopback_port: None,
    };

    let lowered_nodes = lower_chapter_nodes(&raw.nodes, &mut state);

    let fallback_port = state.chapter_loopback_port;
    let mut extracted_commands = state.commands;
    for cmd in &mut extracted_commands {
        if let PtyRole::Daemon { port_hint, .. } = &mut cmd.block.role {
            if port_hint.is_none() {
                *port_hint = fallback_port;
            }
        }
    }

    classify_primary_and_secondary(&mut extracted_commands);

    let sections = extract_sections(&raw.nodes, lowered_nodes);
    let is_exercise = extracted_commands.iter().any(|c| !c.is_pure_cd)
        || !sections.self_checks.is_empty()
        || !state.reference_blocks.is_empty();
    let kind = if is_exercise {
        ChapterKind::Exercise
    } else {
        ChapterKind::Reading
    };

    let mut command_blocks = Vec::with_capacity(extracted_commands.len());
    let mut step1_primary_commands = Vec::new();
    let mut secondary_commands = Vec::new();
    for extracted in extracted_commands {
        if extracted.block.is_secondary {
            secondary_commands.push(extracted.block.id.clone());
        } else {
            step1_primary_commands.push(extracted.block.id.clone());
        }
        command_blocks.push(extracted.block);
    }

    let self_check_items: Vec<String> = sections
        .self_checks
        .iter()
        .map(|item| item.title.clone())
        .collect();

    let three_step = ChapterThreeStepFlow {
        step1_primary_commands,
        step2_reference_blocks: state.reference_blocks,
        step3_self_checks: sections.self_checks,
        secondary_commands,
        reflection_nodes: sections.reflection_nodes,
    };

    ParsedChapter {
        chapter_id,
        index,
        kind,
        title: raw.title,
        goal: sections.goal,
        default_cwd: None,
        entry_file: None,
        nodes: sections.folded_nodes,
        command_blocks,
        self_check_items,
        three_step,
    }
}

fn lower_chapter_nodes(
    nodes: &[CourseNode],
    state: &mut ChapterLowerState<'_>,
) -> Vec<MarkdownNode> {
    let mut out = Vec::with_capacity(nodes.len());
    for (idx, node) in nodes.iter().enumerate() {
        let prev = idx.checked_sub(1).and_then(|p| nodes.get(p));
        out.push(lower_chapter_node(node, prev, state));
    }
    out
}

fn lower_chapter_node(
    node: &CourseNode,
    prev_sibling: Option<&CourseNode>,
    state: &mut ChapterLowerState<'_>,
) -> MarkdownNode {
    match node {
        CourseNode::CodeBlock {
            info_string,
            language,
            code,
        } => {
            if state.chapter_loopback_port.is_none() {
                state.chapter_loopback_port = extract_loopback_port_from_text(code);
            }
            let attrs = parse_fence_attributes(info_string);

            if is_shell_language(language.as_deref()) {
                let extracted = build_command_block(code, &attrs, state);
                let cmd_id = extracted.block.id.clone();
                state.commands.push(extracted);
                MarkdownNode::CodeBlock {
                    language: language.clone(),
                    code: code.clone(),
                    target_path: None,
                    command_block_id: Some(cmd_id),
                }
            } else {
                let is_plain_output = language.as_deref().is_none_or(|lang| {
                    NON_REFERENCE_LANGUAGES
                        .iter()
                        .any(|plain| lang.eq_ignore_ascii_case(plain))
                });
                let resolved_target = if !is_plain_output || attrs.target_path.is_some() {
                    let target = attrs.target_path.or_else(|| {
                        if let Some(CourseNode::Paragraph { children }) = prev_sibling {
                            infer_target_path_from_paragraph(children, state.current_cwd.as_ref())
                        } else {
                            None
                        }
                    });
                    state.reference_blocks.push(ReferenceCodeBlock {
                        code_index: state.next_ref_index,
                        language: language.clone(),
                        target_path: target.clone(),
                        code: code.clone(),
                    });
                    state.next_ref_index += 1;
                    target
                } else {
                    None
                };

                MarkdownNode::CodeBlock {
                    language: language.clone(),
                    code: code.clone(),
                    target_path: resolved_target,
                    command_block_id: None,
                }
            }
        }
        CourseNode::Paragraph { children } => MarkdownNode::Paragraph {
            children: lower_chapter_nodes(children, state),
        },
        CourseNode::Heading {
            level,
            id,
            children,
        } => MarkdownNode::Heading {
            level: *level,
            id: id.clone(),
            children: lower_chapter_nodes(children, state),
        },
        CourseNode::Strong { children } => MarkdownNode::Strong {
            children: lower_chapter_nodes(children, state),
        },
        CourseNode::Emphasis { children } => MarkdownNode::Emphasis {
            children: lower_chapter_nodes(children, state),
        },
        CourseNode::BlockQuote { children } => MarkdownNode::BlockQuote {
            children: lower_chapter_nodes(children, state),
        },
        CourseNode::Link {
            href,
            title,
            children,
        } => MarkdownNode::Link {
            href: href.clone(),
            title: title.clone(),
            children: lower_chapter_nodes(children, state),
        },
        CourseNode::Superscript { children } => MarkdownNode::Superscript {
            children: lower_chapter_nodes(children, state),
        },
        CourseNode::Subscript { children } => MarkdownNode::Subscript {
            children: lower_chapter_nodes(children, state),
        },
        CourseNode::Details { summary, children } => MarkdownNode::Details {
            summary: summary.clone(),
            children: lower_chapter_nodes(children, state),
        },
        CourseNode::List {
            ordered,
            start,
            items,
        } => MarkdownNode::List {
            ordered: *ordered,
            start: *start,
            items: items
                .iter()
                .map(|item| lower_chapter_nodes(&item.nodes, state))
                .collect(),
        },
        CourseNode::Table { headers, rows } => MarkdownNode::Table {
            headers: headers
                .iter()
                .map(|cell| lower_chapter_nodes(cell, state))
                .collect(),
            rows: rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| lower_chapter_nodes(cell, state))
                        .collect()
                })
                .collect(),
        },
        CourseNode::Text { value } => MarkdownNode::Text {
            value: value.clone(),
        },
        CourseNode::InlineCode { value } => MarkdownNode::InlineCode {
            value: value.clone(),
        },
        CourseNode::Image { src, alt, title } => MarkdownNode::Image {
            src: src.clone(),
            alt: alt.clone(),
            title: title.clone(),
        },
        CourseNode::LineBreak => MarkdownNode::LineBreak,
        CourseNode::ThematicBreak => MarkdownNode::ThematicBreak,
        CourseNode::Kbd { label } => MarkdownNode::Kbd {
            label: label.clone(),
        },
    }
}

fn build_command_block(
    code: &str,
    attrs: &FenceAttributes,
    state: &mut ChapterLowerState<'_>,
) -> ExtractedCommand {
    let block_cwd = match attrs.cwd.clone() {
        Some(explicit) => Some(explicit),
        None if block_starts_with_root_reset_cd(code) => None,
        None => state.current_cwd.clone(),
    };

    let analysis = analyze_shell_block(code, block_cwd.as_ref());
    state.current_cwd = analysis.post_cd_cwd.clone();

    let is_daemon = match attrs.mode {
        Some(ExplicitCommandMode::Daemon) => true,
        Some(ExplicitCommandMode::Oneshot) => false,
        None => analysis.matched_daemon.is_some(),
    };

    let is_test_runner = !is_daemon && analysis.is_test_runner;

    let role = if is_daemon {
        let port_hint = attrs.port.or(analysis.command_port);
        let label = attrs
            .label
            .clone()
            .or_else(|| {
                analysis.matched_daemon.as_ref().map(|(seg, rule)| {
                    if seg.chars().count() <= 64 {
                        seg.clone()
                    } else {
                        rule.default_label.to_owned()
                    }
                })
            })
            .unwrap_or_else(|| {
                analysis
                    .first_segment
                    .filter(|s| s.chars().count() <= 64)
                    .unwrap_or_else(|| "daemon".to_owned())
            });

        let service_key = attrs
            .service_key
            .clone()
            .or_else(|| {
                analysis
                    .post_cd_cwd
                    .as_ref()
                    .or(block_cwd.as_ref())
                    .map(|p| slugify_identifier(p.as_str(), 40))
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| {
                analysis
                    .matched_daemon
                    .map(|(_, rule)| rule.default_service_key.to_owned())
            })
            .unwrap_or_else(|| "service".to_owned());

        PtyRole::Daemon {
            service_key,
            label,
            port_hint,
        }
    } else {
        PtyRole::Oneshot { slot_index: 0 }
    };

    let id = CommandBlockId(format!(
        "{}-cmd-{:02}",
        state.chapter_id.0, state.next_cmd_ordinal
    ));
    state.next_cmd_ordinal += 1;

    ExtractedCommand {
        block: CommandBlock {
            id,
            chapter_id: state.chapter_id.clone(),
            command: code.to_owned(),
            cwd: block_cwd,
            role,
            is_secondary: false,
            label: attrs.label.clone(),
        },
        explicit_secondary: attrs.secondary,
        is_daemon,
        is_test_runner,
        is_pure_cd: analysis.is_pure_cd,
    }
}

fn classify_primary_and_secondary(commands: &mut [ExtractedCommand]) {
    for cmd in commands.iter_mut() {
        cmd.block.is_secondary = match cmd.explicit_secondary {
            Some(explicit) => explicit,
            None if cmd.is_daemon || cmd.is_test_runner => false,
            None => true,
        };
    }

    if commands.iter().all(|cmd| cmd.block.is_secondary) {
        if let Some(first_candidate) = commands
            .iter_mut()
            .find(|cmd| !cmd.is_pure_cd && cmd.explicit_secondary != Some(true))
        {
            first_candidate.block.is_secondary = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::markdown_parser::parse_raw_markdown;

    #[test]
    fn explicit_secondary_daemon_still_allows_promoting_another_primary_command() {
        let md = "\
## Chapter 1

```bash secondary=true
go run .
```

```bash
curl -s http://127.0.0.1:8080/healthz
```
";
        let course = extract_course(parse_raw_markdown(md), &ParseMarkdownOptions::default());
        let cmds = &course.chapters[0].command_blocks;
        assert!(cmds[0].is_secondary);
        assert!(!cmds[1].is_secondary);
        assert_eq!(
            course.chapters[0].three_step.step1_primary_commands.len(),
            1
        );
    }

    #[test]
    fn task_lists_inside_details_hints_do_not_leak_into_step3_self_checks() {
        let md = "\
## Chapter 1

<details><summary>Hint</summary>

- [ ] Internal hint checklist item

</details>

- [ ] Top-level verification task
";
        let course = extract_course(parse_raw_markdown(md), &ParseMarkdownOptions::default());
        assert_eq!(
            course.chapters[0].self_check_items,
            vec!["Top-level verification task".to_owned()]
        );
    }

    #[test]
    fn root_reset_cd_clears_inherited_cwd_and_updates_post_block_cwd() {
        let md = "\
## 练习 7：迁移到 GoFrame

```bash
cd task-api
go test ./...
```

```bash
cd /abs/path && gf init task-api-gf && cd task-api-gf && go run main.go
```

```bash
curl -s http://127.0.0.1:8000/tasks
```
";
        let course = extract_course(parse_raw_markdown(md), &ParseMarkdownOptions::default());
        let cmds = &course.chapters[0].command_blocks;
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].cwd, None);
        assert_eq!(cmds[1].cwd, None);
        assert_eq!(
            cmds[1].role,
            PtyRole::Daemon {
                service_key: "task-api-gf".to_owned(),
                label: "go run main.go".to_owned(),
                port_hint: Some(8000),
            }
        );
        assert_eq!(
            cmds[2].cwd.as_ref().map(RelWorkspacePath::as_str),
            Some("task-api-gf")
        );
    }

    #[test]
    fn chapters_classify_reading_vs_exercise() {
        let md = "\
## 如何使用本手册

纯阅读说明，不含可执行命令、参考代码或验收清单。

## 仅目录切换说明

```bash
cd task-api
```

## 练习 1：最小服务

```bash
go run .
```
";
        let course = extract_course(parse_raw_markdown(md), &ParseMarkdownOptions::default());
        assert_eq!(course.chapters.len(), 3);
        assert_eq!(course.chapters[0].kind, ChapterKind::Reading);
        assert_eq!(course.chapters[1].kind, ChapterKind::Reading);
        assert_eq!(course.chapters[2].kind, ChapterKind::Exercise);
    }
}

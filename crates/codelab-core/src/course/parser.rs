use std::collections::{HashMap, HashSet};

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::course::ChapterReferences;
use crate::course::chapter_id::{
    ManifestChapterHint, extract_numbered_exercise_id, match_manifest_chapter, resolve_chapter_id,
    split_heading_and_anchor,
};
use crate::course::model::{
    Chapter, ChapterKind, ChapterPresentation, CheckKind, CommandBlock, ContentBlock,
    CourseCheckSpec, DaemonRequirement, ThreeStepProjection,
};
use crate::course::sanitize::sanitize_html;
use crate::course::shell::{analyze_command_block, is_command_shell, parse_fence_info};
use crate::model::CourseLanguage;

#[derive(Debug, Clone, Default)]
pub(crate) struct ChapterChecksOverlay {
    pub requires_daemon: Option<DaemonRequirement>,
    pub checks: Vec<CourseCheckSpec>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ParseOverlay {
    pub has_pack_metadata: bool,
    pub chapter_hints: Vec<ManifestChapterHint>,
    pub checks_by_chapter: HashMap<String, ChapterChecksOverlay>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedMarkdownDocument {
    pub title: Option<String>,
    pub intro_html: Option<String>,
    pub inferred_language: Option<CourseLanguage>,
    pub chapters: Vec<Chapter>,
    pub references: ChapterReferences,
}

#[derive(Debug)]
enum SectionItem<'a> {
    Heading {
        text: String,
        events: Vec<Event<'a>>,
    },
    ShellCommand {
        info: String,
        code: String,
    },
    CodeBlock {
        file_attr: Option<String>,
        events: Vec<Event<'a>>,
    },
    Prose {
        plain_text: String,
        list_items: Vec<String>,
        nested_shell_blocks: Vec<(String, String)>,
        events: Vec<Event<'a>>,
    },
}

struct RawSection<'a> {
    heading_text: String,
    parsed_id: Option<String>,
    items: Vec<SectionItem<'a>>,
    references: Vec<(String, String)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChapterZone {
    Main,
    Verification,
    Reflection,
}

pub(crate) fn parse_markdown_course(
    markdown: &str,
    overlay: &ParseOverlay,
) -> ParsedMarkdownDocument {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let all_events: Vec<Event<'_>> = Parser::new_ext(markdown, options).collect();

    let mut doc_title: Option<String> = None;
    let mut intro_events: Vec<Event<'_>> = Vec::new();
    let mut raw_sections: Vec<RawSection<'_>> = Vec::new();
    let mut current_section: Option<RawSection<'_>> = None;
    let mut lang_votes: HashMap<CourseLanguage, usize> = HashMap::new();

    let mut idx = 0;
    while idx < all_events.len() {
        match &all_events[idx] {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) if doc_title.is_none() && current_section.is_none() => {
                let (block, next_idx) = collect_balanced(&all_events, idx);
                let text = extract_plain_text(&block);
                let cleaned = split_heading_and_anchor(&text, None).clean_title;
                if !cleaned.is_empty() {
                    doc_title = Some(cleaned);
                }
                idx = next_idx;
            }
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                id,
                ..
            }) => {
                if let Some(sec) = current_section.take() {
                    raw_sections.push(sec);
                }
                let parsed_id = id.as_ref().map(|s| s.to_string());
                let (block, next_idx) = collect_balanced(&all_events, idx);
                let heading_text = extract_plain_text(&block);
                current_section = Some(RawSection {
                    heading_text,
                    parsed_id,
                    items: Vec::new(),
                    references: Vec::new(),
                });
                idx = next_idx;
            }
            _ => {
                let (block, next_idx) = collect_balanced(&all_events, idx);
                if let Some(sec) = current_section.as_mut() {
                    sec.references.extend(file_fences(&block));
                    if let Some(item) = classify_section_item(block, &mut lang_votes) {
                        sec.items.push(item);
                    }
                } else {
                    record_language_votes_from_events(&block, &mut lang_votes);
                    intro_events.extend(block);
                }
                idx = next_idx;
            }
        }
    }

    if let Some(sec) = current_section.take() {
        raw_sections.push(sec);
    }

    // Promote pre-H2 content into an opening guide chapter if there are no H2 chapters at all
    // OR if pre-H2 itself contains executable shell Command Blocks that must remain interactive.
    let intro_has_shell = events_contain_shell_fence(&intro_events);
    let intro_references = file_fences(&intro_events);
    let intro_html = if !intro_events.is_empty()
        && (raw_sections.is_empty() || intro_has_shell || !intro_references.is_empty())
    {
        let mut items = Vec::new();
        let mut cursor = 0;
        while cursor < intro_events.len() {
            let (block, next_cursor) = collect_balanced(&intro_events, cursor);
            if let Some(item) = classify_section_item(block, &mut lang_votes) {
                items.push(item);
            }
            cursor = next_cursor;
        }
        raw_sections.insert(
            0,
            RawSection {
                heading_text: doc_title.clone().unwrap_or_else(|| "Overview".to_string()),
                parsed_id: None,
                items,
                references: intro_references,
            },
        );
        None
    } else {
        render_sanitized_events(intro_events)
    };

    let mut used_ids = HashSet::new();
    let mut chapters = Vec::with_capacity(raw_sections.len());

    let mut references = ChapterReferences::new();
    for mut raw in raw_sections {
        let files = std::mem::take(&mut raw.references);
        let mut chapter = build_chapter(raw, overlay, &mut used_ids);
        chapter.has_reference_hint = !files.is_empty();
        if chapter.has_reference_hint {
            references.insert(chapter.chapter_id.clone(), files);
        }
        chapters.push(chapter);
    }

    let inferred_language = lang_votes
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(lang, _)| lang);

    ParsedMarkdownDocument {
        title: doc_title,
        intro_html,
        inferred_language,
        chapters,
        references,
    }
}

fn file_fences(events: &[Event<'_>]) -> Vec<(String, String)> {
    events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = event else {
                return None;
            };
            let path = parse_fence_info(info).attrs.remove("file")?;
            let code = events[index + 1..]
                .iter()
                .take_while(|event| !matches!(event, Event::End(TagEnd::CodeBlock)))
                .filter_map(|event| match event {
                    Event::Text(text) => Some(text.as_ref()),
                    _ => None,
                })
                .collect();
            Some((path, code))
        })
        .collect()
}

fn is_shell_command_fence(info: &str) -> bool {
    let meta = parse_fence_info(info);
    is_command_shell(&meta.lang) && !meta.attrs.contains_key("file")
}

fn events_contain_shell_fence(events: &[Event<'_>]) -> bool {
    events.iter().any(|ev| {
        if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = ev {
            is_shell_command_fence(info)
        } else {
            false
        }
    })
}

fn collect_balanced<'a>(events: &[Event<'a>], start: usize) -> (Vec<Event<'a>>, usize) {
    match &events[start] {
        Event::Start(_) => {
            let mut depth = 0usize;
            let mut idx = start;
            let mut out = Vec::new();
            while idx < events.len() {
                let ev = events[idx].clone();
                match &ev {
                    Event::Start(_) => depth += 1,
                    Event::End(_) => depth = depth.saturating_sub(1),
                    _ => {}
                }
                out.push(ev);
                idx += 1;
                if depth == 0 {
                    break;
                }
            }
            (out, idx)
        }
        single => (vec![single.clone()], start + 1),
    }
}

fn classify_section_item<'a>(
    events: Vec<Event<'a>>,
    lang_votes: &mut HashMap<CourseLanguage, usize>,
) -> Option<SectionItem<'a>> {
    if events.is_empty() {
        return None;
    }

    match &events[0] {
        Event::Start(Tag::Heading { .. }) => {
            let text = extract_plain_text(&events);
            Some(SectionItem::Heading { text, events })
        }
        Event::Start(Tag::CodeBlock(kind)) => {
            let info = match kind {
                CodeBlockKind::Fenced(info) => info.to_string(),
                CodeBlockKind::Indented => String::new(),
            };
            let meta = parse_fence_info(&info);
            let code = extract_code_text(&events);
            if is_command_shell(&meta.lang) && !meta.attrs.contains_key("file") {
                Some(SectionItem::ShellCommand { info, code })
            } else {
                vote_language(&meta.lang, lang_votes);
                let file_attr = meta
                    .attrs
                    .get("file")
                    .or_else(|| meta.attrs.get("path"))
                    .cloned();
                Some(SectionItem::CodeBlock { file_attr, events })
            }
        }
        _ => {
            record_language_votes_from_events(&events, lang_votes);
            let (stripped_events, nested_shell_blocks) = strip_nested_shell_blocks(events);
            let plain_text = extract_plain_text(&stripped_events);
            let list_items = extract_top_list_items(&stripped_events);
            Some(SectionItem::Prose {
                plain_text,
                list_items,
                nested_shell_blocks,
                events: stripped_events,
            })
        }
    }
}

struct ChapterAccumulator<'a> {
    chapter_id: String,
    default_cwd: Option<String>,
    entry_file: Option<String>,
    zone: ChapterZone,
    commands: Vec<CommandBlock>,
    blocks: Vec<ContentBlock>,
    pending_linear_prose: Vec<Event<'a>>,
    summary_events: Vec<Event<'a>>,
    summary_sealed: bool,
    step1_intro_events: Vec<Event<'a>>,
    step2_events: Vec<Event<'a>>,
    step3_intro_events: Vec<Event<'a>>,
    md_verification_items: Vec<String>,
    reflection_events: Vec<Event<'a>>,
    chapter_context_cwd: Option<String>,
    cmd_index: usize,
    has_non_shell_code: bool,
    has_verification_section: bool,
}

impl<'a> ChapterAccumulator<'a> {
    fn new(chapter_id: String, default_cwd: Option<String>, entry_file: Option<String>) -> Self {
        Self {
            chapter_id,
            default_cwd,
            entry_file,
            zone: ChapterZone::Main,
            commands: Vec::new(),
            blocks: Vec::new(),
            pending_linear_prose: Vec::new(),
            summary_events: Vec::new(),
            summary_sealed: false,
            step1_intro_events: Vec::new(),
            step2_events: Vec::new(),
            step3_intro_events: Vec::new(),
            md_verification_items: Vec::new(),
            reflection_events: Vec::new(),
            chapter_context_cwd: None,
            cmd_index: 0,
            has_non_shell_code: false,
            has_verification_section: false,
        }
    }

    fn flush_linear_prose(&mut self) {
        if !self.pending_linear_prose.is_empty() {
            let taken = std::mem::take(&mut self.pending_linear_prose);
            if let Some(html) = render_sanitized_events(taken) {
                self.blocks.push(ContentBlock::Prose { html });
            }
        }
    }

    fn push_shell_command(&mut self, info: &str, code: &str) -> Option<String> {
        let meta = parse_fence_info(info);
        self.cmd_index += 1;
        let cmd_id = format!("{}:cmd-{}", self.chapter_id, self.cmd_index);
        let cmd = analyze_command_block(
            cmd_id,
            &meta,
            code,
            self.default_cwd.as_deref(),
            &mut self.chapter_context_cwd,
        )?;
        let id = cmd.id.clone();
        self.flush_linear_prose();
        self.blocks.push(ContentBlock::Command {
            command_id: id.clone(),
        });
        self.commands.push(cmd);
        Some(id)
    }

    fn preview_is_secondary_shell(&self, info: &str, code: &str) -> Option<bool> {
        let meta = parse_fence_info(info);
        let mut scratch_cwd = self.chapter_context_cwd.clone();
        let cmd = analyze_command_block(
            String::new(),
            &meta,
            code,
            self.default_cwd.as_deref(),
            &mut scratch_cwd,
        )?;
        Some(cmd.is_secondary)
    }
}

fn build_chapter(
    raw: RawSection<'_>,
    overlay: &ParseOverlay,
    used_ids: &mut HashSet<String>,
) -> Chapter {
    let resolved_heading = split_heading_and_anchor(&raw.heading_text, raw.parsed_id.as_deref());
    let manifest_hint = match_manifest_chapter(
        &overlay.chapter_hints,
        &resolved_heading.clean_title,
        resolved_heading.explicit_anchor.as_deref(),
    );

    let chapter_id = resolve_chapter_id(
        manifest_hint,
        resolved_heading.explicit_anchor.as_deref(),
        &resolved_heading.clean_title,
        used_ids,
    );

    let title = manifest_hint
        .and_then(|h| h.title.as_deref())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| resolved_heading.clean_title.clone());

    let default_cwd = manifest_hint.and_then(|h| h.default_cwd.clone());
    let entry_file = manifest_hint.and_then(|h| h.entry_file.clone());
    let checks_overlay = overlay.checks_by_chapter.get(&chapter_id);

    let mut acc = ChapterAccumulator::new(chapter_id.clone(), default_cwd.clone(), entry_file);
    let mut items_iter = raw.items.into_iter().peekable();

    while let Some(item) = items_iter.next() {
        let next_shell_secondary = match items_iter.peek() {
            Some(SectionItem::ShellCommand { info, code }) => {
                acc.preview_is_secondary_shell(info, code)
            }
            _ => None,
        };

        match item {
            SectionItem::Heading { text, events } => {
                if is_reflection_heading(&text) {
                    acc.zone = ChapterZone::Reflection;
                    continue;
                }
                if is_verification_heading(&text) {
                    acc.zone = ChapterZone::Verification;
                    acc.has_verification_section = true;
                    continue;
                }
                if acc.zone != ChapterZone::Main {
                    acc.zone = ChapterZone::Main;
                }
                acc.summary_sealed = true;
                acc.pending_linear_prose.extend(events.clone());
                acc.step2_events.extend(events);
            }
            SectionItem::ShellCommand { info, code } => {
                acc.summary_sealed = true;
                let _ = acc.push_shell_command(&info, &code);
            }
            SectionItem::CodeBlock { file_attr, events } => {
                acc.summary_sealed = true;
                acc.has_non_shell_code = true;
                if acc.entry_file.is_none()
                    && let Some(path) = file_attr
                {
                    acc.entry_file = Some(path);
                }
                match acc.zone {
                    ChapterZone::Reflection => acc.reflection_events.extend(events),
                    ChapterZone::Verification => {
                        acc.pending_linear_prose.extend(events.clone());
                        acc.step3_intro_events.extend(events);
                    }
                    ChapterZone::Main => {
                        acc.pending_linear_prose.extend(events.clone());
                        acc.step2_events.extend(events);
                    }
                }
            }
            SectionItem::Prose {
                plain_text,
                list_items,
                nested_shell_blocks,
                events,
            } => {
                if let Some(inline_rest) = strip_reflection_lead_in(&plain_text) {
                    acc.zone = ChapterZone::Reflection;
                    if !inline_rest.is_empty() {
                        acc.reflection_events.extend(events);
                    }
                    for (info, code) in nested_shell_blocks {
                        let _ = acc.push_shell_command(&info, &code);
                    }
                    continue;
                }

                if let Some(inline_rest) = strip_verification_lead_in(&plain_text) {
                    acc.zone = ChapterZone::Verification;
                    acc.has_verification_section = true;
                    acc.pending_linear_prose.extend(events.clone());
                    if !inline_rest.is_empty() {
                        acc.md_verification_items.push(inline_rest.to_string());
                        acc.step3_intro_events.extend(events);
                    }
                    for (info, code) in nested_shell_blocks {
                        let _ = acc.push_shell_command(&info, &code);
                    }
                    continue;
                }

                match acc.zone {
                    ChapterZone::Reflection => acc.reflection_events.extend(events),
                    ChapterZone::Verification => {
                        acc.pending_linear_prose.extend(events.clone());
                        if !list_items.is_empty() {
                            acc.md_verification_items.extend(list_items);
                        } else if !plain_text.trim().is_empty() {
                            acc.step3_intro_events.extend(events);
                        }
                    }
                    ChapterZone::Main => {
                        acc.pending_linear_prose.extend(events.clone());
                        if !acc.summary_sealed
                            && list_items.is_empty()
                            && is_paragraph_block(&events)
                            && acc.summary_events.is_empty()
                        {
                            acc.summary_events.extend(events);
                        } else if is_command_lead_in_paragraph(&plain_text, &events)
                            && let Some(is_secondary) = next_shell_secondary
                        {
                            acc.summary_sealed = true;
                            if !is_secondary {
                                acc.step1_intro_events.extend(events);
                            }
                            // Secondary command lead-ins stay in linear blocks and are not orphaned in Step 1/2.
                        } else {
                            acc.summary_sealed = true;
                            acc.step2_events.extend(events);
                        }
                    }
                }

                for (info, code) in nested_shell_blocks {
                    let _ = acc.push_shell_command(&info, &code);
                }
            }
        }
    }

    acc.flush_linear_prose();

    let has_numbered_pattern =
        extract_numbered_exercise_id(&resolved_heading.clean_title).is_some();
    let has_overlay_checks = checks_overlay.is_some_and(|ov| !ov.checks.is_empty());

    let kind = if let Some(explicit_kind) = manifest_hint.and_then(|h| h.kind) {
        explicit_kind
    } else if has_overlay_checks || has_numbered_pattern || acc.has_verification_section {
        ChapterKind::Exercise
    } else {
        ChapterKind::Guide
    };

    let summary_html = render_sanitized_events(acc.summary_events);
    let reflections_html = render_sanitized_events(acc.reflection_events);

    let structured_signal_count = usize::from(!acc.commands.is_empty())
        + usize::from(acc.has_non_shell_code || acc.entry_file.is_some())
        + usize::from(has_overlay_checks || acc.has_verification_section);

    let use_three_step = kind == ChapterKind::Exercise
        && (overlay.has_pack_metadata
            || has_overlay_checks
            || acc.has_verification_section
            || structured_signal_count >= 2);

    let (presentation, three_step) = if use_three_step {
        (
            ChapterPresentation::ThreeStep,
            Some(build_three_step_projection(
                &acc.commands,
                acc.step1_intro_events,
                acc.step2_events,
                acc.step3_intro_events,
                acc.md_verification_items,
                acc.entry_file.clone(),
                checks_overlay,
                reflections_html.clone(),
            )),
        )
    } else {
        (ChapterPresentation::LinearFallback, None)
    };

    Chapter {
        chapter_id,
        title,
        kind,
        presentation,
        has_reference_hint: false,
        default_cwd,
        entry_file: acc.entry_file,
        summary_html,
        blocks: acc.blocks,
        commands: acc.commands,
        three_step,
        reflections_html,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_three_step_projection<'a>(
    commands: &[CommandBlock],
    step1_intro_events: Vec<Event<'a>>,
    mut step2_events: Vec<Event<'a>>,
    step3_intro_events: Vec<Event<'a>>,
    md_verification_items: Vec<String>,
    entry_file: Option<String>,
    checks_overlay: Option<&ChapterChecksOverlay>,
    reflections_html: Option<String>,
) -> ThreeStepProjection {
    let mut step1_command_ids = Vec::new();
    let mut secondary_command_ids = Vec::new();

    for cmd in commands {
        if cmd.is_secondary {
            secondary_command_ids.push(cmd.id.clone());
        } else {
            step1_command_ids.push(cmd.id.clone());
        }
    }

    let step3_checks = if let Some(ov) = checks_overlay
        && !ov.checks.is_empty()
    {
        ov.checks.clone()
    } else {
        md_verification_items
            .into_iter()
            .enumerate()
            .map(|(i, item_title)| CourseCheckSpec {
                id: format!("md-v{}", i + 1),
                title: item_title,
                kind: CheckKind::SelfCheck,
                required: false,
                cwd: None,
                command: None,
                method: None,
                url: None,
                headers: Default::default(),
                body: None,
                expect: Default::default(),
                timeout_sec: None,
                expect_exit_code: None,
                forbid_output: Vec::new(),
                require_tests: Vec::new(),
            })
            .collect()
    };

    let step1_intro_html = if step1_command_ids.is_empty() {
        if !step1_intro_events.is_empty() {
            step2_events.extend(step1_intro_events);
        }
        None
    } else {
        render_sanitized_events(step1_intro_events)
    };

    ThreeStepProjection {
        step1_intro_html,
        step1_command_ids,
        step2_entry_file: entry_file,
        step2_html: render_sanitized_events(step2_events),
        step3_intro_html: render_sanitized_events(step3_intro_events),
        step3_checks,
        requires_daemon: checks_overlay.and_then(|ov| ov.requires_daemon.clone()),
        secondary_command_ids,
        reflections_html,
    }
}

fn render_sanitized_events(events: Vec<Event<'_>>) -> Option<String> {
    if events.is_empty() {
        return None;
    }
    let normalized = events.into_iter().map(|ev| match ev {
        Event::TaskListMarker(true) => Event::Html(
            "<span class=\"md-task md-task--done\" aria-hidden=\"true\">✓</span> ".into(),
        ),
        Event::TaskListMarker(false) => {
            Event::Html("<span class=\"md-task\" aria-hidden=\"true\">○</span> ".into())
        }
        other => other,
    });
    let mut raw_html = String::new();
    pulldown_cmark::html::push_html(&mut raw_html, normalized);
    let cleaned = sanitize_html(&raw_html);
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn extract_plain_text(events: &[Event<'_>]) -> String {
    let mut out = String::new();
    for ev in events {
        match ev {
            Event::Text(text) | Event::Code(text) => out.push_str(text),
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn extract_code_text(events: &[Event<'_>]) -> String {
    let mut out = String::new();
    for ev in events {
        if let Event::Text(text) = ev {
            out.push_str(text);
        }
    }
    out
}

fn extract_top_list_items(events: &[Event<'_>]) -> Vec<String> {
    let mut items = Vec::new();
    let mut list_depth = 0usize;
    let mut in_item = false;
    let mut buf = String::new();

    for ev in events {
        match ev {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Start(Tag::Item) if list_depth == 1 => {
                in_item = true;
                buf.clear();
            }
            Event::End(TagEnd::Item) if list_depth == 1 && in_item => {
                in_item = false;
                let trimmed = buf.trim();
                if !trimmed.is_empty() {
                    items.push(trimmed.to_string());
                }
            }
            Event::Text(text) | Event::Code(text) if in_item => buf.push_str(text),
            Event::SoftBreak | Event::HardBreak if in_item => buf.push(' '),
            _ => {}
        }
    }
    items
}

fn strip_nested_shell_blocks<'a>(
    events: Vec<Event<'a>>,
) -> (Vec<Event<'a>>, Vec<(String, String)>) {
    let mut kept = Vec::with_capacity(events.len());
    let mut extracted = Vec::new();
    let mut active_shell_info: Option<String> = None;
    let mut active_code = String::new();

    for ev in events {
        match ev {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info)))
                if is_shell_command_fence(info) =>
            {
                active_shell_info = Some(info.to_string());
                active_code.clear();
            }
            Event::End(TagEnd::CodeBlock) if active_shell_info.is_some() => {
                if let Some(info) = active_shell_info.take() {
                    extracted.push((info, std::mem::take(&mut active_code)));
                }
            }
            Event::Text(ref text) if active_shell_info.is_some() => {
                active_code.push_str(text);
            }
            other => {
                if active_shell_info.is_none() {
                    kept.push(other);
                }
            }
        }
    }

    (kept, extracted)
}

fn is_paragraph_block(events: &[Event<'_>]) -> bool {
    matches!(events.first(), Some(Event::Start(Tag::Paragraph)))
}

fn is_command_lead_in_paragraph(plain_text: &str, events: &[Event<'_>]) -> bool {
    if !is_paragraph_block(events) {
        return false;
    }
    let trimmed = plain_text.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= 48
        && (trimmed.ends_with('：') || trimmed.ends_with(':'))
}

fn is_verification_heading(title: &str) -> bool {
    let lower = title.trim().to_ascii_lowercase();
    title.trim().starts_with("验收")
        || lower.starts_with("verification")
        || lower.starts_with("acceptance")
        || lower == "checks"
        || lower == "checklist"
}

fn is_reflection_heading(title: &str) -> bool {
    let lower = title.trim().to_ascii_lowercase();
    title.trim().starts_with("思考")
        || title.trim().starts_with("课后思考")
        || lower.starts_with("reflection")
        || lower.starts_with("think")
        || lower == "questions"
}

fn strip_verification_lead_in(plain_text: &str) -> Option<&str> {
    let trimmed = plain_text.trim();
    for prefix in [
        "验收：",
        "验收:",
        "验收清单：",
        "验收清单:",
        "Verification:",
        "Acceptance:",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some(rest.trim());
        }
    }
    None
}

fn strip_reflection_lead_in(plain_text: &str) -> Option<&str> {
    let trimmed = plain_text.trim();
    for prefix in [
        "思考：",
        "思考:",
        "课后思考：",
        "课后思考:",
        "Reflection:",
        "Reflections:",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some(rest.trim());
        }
    }
    None
}

fn record_language_votes_from_events(
    events: &[Event<'_>],
    votes: &mut HashMap<CourseLanguage, usize>,
) {
    for ev in events {
        if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = ev {
            let meta = parse_fence_info(info);
            vote_language(&meta.lang, votes);
        }
    }
}

fn vote_language(lang: &str, votes: &mut HashMap<CourseLanguage, usize>) {
    if let Some(detected) = CourseLanguage::from_token(lang) {
        *votes.entry(detected).or_insert(0) += 1;
    }
}

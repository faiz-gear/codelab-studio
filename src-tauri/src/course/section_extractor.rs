use std::collections::{BTreeMap, HashSet};

use crate::course::ast::{CourseListItem, CourseNode};
use crate::course::ast_text::{
    collect_list_item_lead_text, collect_plain_text, normalize_rel_workspace_path,
    normalize_whitespace,
};
use crate::domain::course::{MarkdownNode, SelfCheckItem};
use crate::domain::ids::RelWorkspacePath;
use crate::domain::runtime::Language;

const FILE_INTRO_ZH_KEYWORDS: &[&str] = &[
    "创建", "修改", "编辑", "新建", "建立", "写入", "打开", "更新", "保存", "编写", "新增", "实现",
    "文件", "例如",
];

const FILE_INTRO_EN_KEYWORDS: &[&str] = &[
    "create",
    "edit",
    "update",
    "modify",
    "write",
    "save",
    "open",
    "add",
    "implement",
    "file",
    "example",
];

const PLAUSIBLE_FILE_EXTENSIONS: &[&str] = &[
    ".go",
    ".sql",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".mjs",
    ".cjs",
    ".py",
    ".java",
    ".kt",
    ".rs",
    ".json",
    ".yaml",
    ".yml",
    ".toml",
    ".mod",
    ".md",
    ".html",
    ".css",
    ".scss",
    ".sh",
    ".env",
    ".xml",
    ".gradle",
    ".properties",
    ".proto",
    ".graphql",
];

const PLAUSIBLE_EXACT_FILENAMES: &[&str] = &[
    "Dockerfile",
    "Makefile",
    "go.mod",
    "go.sum",
    "Cargo.toml",
    "package.json",
    "pom.xml",
    "build.gradle",
    "pyproject.toml",
];

const ACCEPTANCE_HEADINGS: &[&str] = &[
    "验收",
    "验收清单",
    "验收标准",
    "交付验收",
    "自动验收",
    "自查",
    "自查清单",
    "verification",
    "acceptance",
    "acceptance criteria",
    "checklist",
    "self-check",
    "self check",
];

const ACCEPTANCE_PREFIXES: &[&str] = &[
    "验收：",
    "验收:",
    "验证：",
    "验证:",
    "自查：",
    "自查:",
    "acceptance:",
    "verification:",
    "checklist:",
    "self-check:",
];

const ACCEPTANCE_SUFFIXES: &[&str] = &["验收：", "验收:", "验证：", "验证:"];

const REFLECTION_HEADINGS: &[&str] = &[
    "思考",
    "思考题",
    "课后思考",
    "反思",
    "reflection",
    "reflections",
    "reflection questions",
    "questions",
    "review questions",
];

const REFLECTION_PREFIXES: &[(&str, &str)] = &[
    ("课后思考：", "课后思考"),
    ("课后思考:", "课后思考"),
    ("思考题：", "思考题"),
    ("思考题:", "思考题"),
    ("思考：", "思考"),
    ("思考:", "思考"),
    ("reflections:", "Reflections"),
    ("reflection:", "Reflection"),
    ("questions:", "Questions"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionKind {
    Normal,
    Acceptance,
    Reflection,
}

#[derive(Debug, Clone)]
enum SectionSpan {
    Single(usize, SectionKind),
    AcceptanceHeading {
        start: usize,
        end: usize,
    },
    ReflectionHeading {
        summary: String,
        start: usize,
        end: usize,
    },
    ReflectionLead {
        summary: String,
        start: usize,
        end: usize,
    },
}

pub(crate) struct ExtractedSections {
    pub goal: Option<String>,
    pub self_checks: Vec<SelfCheckItem>,
    pub folded_nodes: Vec<MarkdownNode>,
    pub reflection_nodes: Vec<MarkdownNode>,
}

pub(crate) fn extract_sections(
    raw_nodes: &[CourseNode],
    lowered_nodes: Vec<MarkdownNode>,
) -> ExtractedSections {
    debug_assert_eq!(raw_nodes.len(), lowered_nodes.len());

    let spans = partition_sections(raw_nodes);
    let goal = extract_goal_from_spans(raw_nodes, &spans);
    let self_checks = extract_self_checks_from_spans(raw_nodes, &spans);
    let (folded_nodes, reflection_nodes) =
        fold_reflections_from_spans(raw_nodes, lowered_nodes, &spans);

    ExtractedSections {
        goal,
        self_checks,
        folded_nodes,
        reflection_nodes,
    }
}

fn partition_sections(nodes: &[CourseNode]) -> Vec<SectionSpan> {
    let mut spans = Vec::new();
    let mut idx = 0usize;

    while idx < nodes.len() {
        match &nodes[idx] {
            CourseNode::Heading {
                level, children, ..
            } if *level >= 3 => {
                let heading_text = normalize_whitespace(&collect_plain_text(children));
                if is_reflection_heading(&heading_text) {
                    let end = heading_section_end(nodes, idx, *level, false);
                    spans.push(SectionSpan::ReflectionHeading {
                        summary: heading_text,
                        start: idx,
                        end,
                    });
                    idx = end;
                    continue;
                }
                if is_acceptance_heading(&heading_text) {
                    let end = heading_section_end(nodes, idx, *level, true);
                    spans.push(SectionSpan::AcceptanceHeading { start: idx, end });
                    idx = end;
                    continue;
                }
                spans.push(SectionSpan::Single(idx, SectionKind::Normal));
            }
            CourseNode::Paragraph { children } => {
                let text = normalize_whitespace(&collect_plain_text(children));
                if let Some(summary) = match_reflection_lead_paragraph(&text) {
                    let end = if matches!(nodes.get(idx + 1), Some(CourseNode::List { .. })) {
                        idx + 2
                    } else {
                        idx + 1
                    };
                    spans.push(SectionSpan::ReflectionLead {
                        summary,
                        start: idx,
                        end,
                    });
                    idx = end;
                    continue;
                }
                if match_acceptance_lead_paragraph(&text).is_some() {
                    spans.push(SectionSpan::Single(idx, SectionKind::Acceptance));
                } else {
                    spans.push(SectionSpan::Single(idx, SectionKind::Normal));
                }
            }
            CourseNode::Details { summary, .. } if is_reflection_heading(summary) => {
                spans.push(SectionSpan::Single(idx, SectionKind::Reflection));
            }
            _ => {
                spans.push(SectionSpan::Single(idx, SectionKind::Normal));
            }
        }
        idx += 1;
    }

    spans
}

fn heading_section_end(
    nodes: &[CourseNode],
    heading_idx: usize,
    level: u8,
    stop_on_reflection: bool,
) -> usize {
    let mut scan = heading_idx + 1;
    while scan < nodes.len() {
        match &nodes[scan] {
            CourseNode::Heading {
                level: next_lvl,
                children,
                ..
            } => {
                if *next_lvl <= level {
                    break;
                }
                if stop_on_reflection {
                    let text = normalize_whitespace(&collect_plain_text(children));
                    if is_reflection_heading(&text) {
                        break;
                    }
                }
            }
            CourseNode::Paragraph { children } if stop_on_reflection => {
                let text = normalize_whitespace(&collect_plain_text(children));
                if match_reflection_lead_paragraph(&text).is_some() {
                    break;
                }
            }
            _ => {}
        }
        scan += 1;
    }
    scan
}

fn extract_goal_from_spans(nodes: &[CourseNode], spans: &[SectionSpan]) -> Option<String> {
    for span in spans {
        if let SectionSpan::Single(idx, SectionKind::Normal) = span {
            if let CourseNode::Paragraph { children } = &nodes[*idx] {
                let text = normalize_whitespace(&collect_plain_text(children));
                if let Some(sentence) = extract_first_sentence(&text) {
                    return Some(sentence);
                }
            }
        }
    }
    None
}

fn extract_self_checks_from_spans(
    nodes: &[CourseNode],
    spans: &[SectionSpan],
) -> Vec<SelfCheckItem> {
    let mut collected: Vec<(String, bool)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for span in spans {
        match span {
            SectionSpan::AcceptanceHeading { start, end } => {
                for scan in (*start + 1)..*end {
                    match &nodes[scan] {
                        CourseNode::Paragraph { children } => {
                            let text = normalize_whitespace(&collect_plain_text(children));
                            let followed_by_list = scan + 1 < *end
                                && matches!(nodes.get(scan + 1), Some(CourseNode::List { .. }));
                            let is_colon_lead = text.ends_with('：') || text.ends_with(':');
                            if !text.is_empty() && !(followed_by_list && is_colon_lead) {
                                push_unique_check(&mut collected, &mut seen, text, false);
                            }
                        }
                        CourseNode::List { items, .. } => {
                            collect_list_checks(items, false, &mut collected, &mut seen);
                        }
                        _ => {}
                    }
                }
            }
            SectionSpan::Single(idx, SectionKind::Acceptance) => {
                if let CourseNode::Paragraph { children } = &nodes[*idx] {
                    let text = normalize_whitespace(&collect_plain_text(children));
                    if let Some(Some(rem)) = match_acceptance_lead_paragraph(&text) {
                        push_unique_check(&mut collected, &mut seen, rem, false);
                    }
                    if let Some(CourseNode::List { items, .. }) = nodes.get(*idx + 1) {
                        collect_list_checks(items, false, &mut collected, &mut seen);
                    }
                }
            }
            SectionSpan::Single(idx, SectionKind::Normal) => {
                if let CourseNode::List { items, .. } = &nodes[*idx] {
                    collect_list_checks(items, true, &mut collected, &mut seen);
                }
            }
            SectionSpan::Single(_, SectionKind::Reflection)
            | SectionSpan::ReflectionHeading { .. }
            | SectionSpan::ReflectionLead { .. } => {}
        }
    }

    collected
        .into_iter()
        .enumerate()
        .map(|(i, (title, checked_by_default))| SelfCheckItem {
            id: format!("s{}", i + 1),
            title,
            checked_by_default,
        })
        .collect()
}

fn collect_list_checks(
    items: &[CourseListItem],
    only_task_items: bool,
    collected: &mut Vec<(String, bool)>,
    seen: &mut HashSet<String>,
) {
    for item in items {
        if only_task_items && item.task_checked.is_none() {
            continue;
        }
        let title = normalize_whitespace(&collect_list_item_lead_text(&item.nodes));
        push_unique_check(collected, seen, title, item.task_checked.unwrap_or(false));
    }
}

fn push_unique_check(
    collected: &mut Vec<(String, bool)>,
    seen: &mut HashSet<String>,
    title: String,
    checked: bool,
) {
    if !title.is_empty() && seen.insert(title.clone()) {
        collected.push((title, checked));
    }
}

fn fold_reflections_from_spans(
    raw_nodes: &[CourseNode],
    lowered_nodes: Vec<MarkdownNode>,
    spans: &[SectionSpan],
) -> (Vec<MarkdownNode>, Vec<MarkdownNode>) {
    let mut slots: Vec<Option<MarkdownNode>> = lowered_nodes.into_iter().map(Some).collect();
    let mut folded = Vec::with_capacity(slots.len());
    let mut reflection_nodes = Vec::new();

    for span in spans {
        match span {
            SectionSpan::ReflectionHeading {
                summary,
                start,
                end,
            } => {
                let mut children = Vec::new();
                for slot in slots.iter_mut().take(*end).skip(*start + 1) {
                    if let Some(node) = slot.take() {
                        children.push(node);
                    }
                }
                reflection_nodes.extend(children.clone());
                folded.push(MarkdownNode::Details {
                    summary: summary.clone(),
                    children,
                });
            }
            SectionSpan::ReflectionLead {
                summary,
                start,
                end,
            } => {
                let mut children = Vec::new();
                for slot in slots.iter_mut().take(*end).skip(*start) {
                    if let Some(node) = slot.take() {
                        children.push(node);
                    }
                }
                reflection_nodes.extend(children.clone());
                folded.push(MarkdownNode::Details {
                    summary: summary.clone(),
                    children,
                });
            }
            SectionSpan::AcceptanceHeading { start, end } => {
                for slot in slots.iter_mut().take(*end).skip(*start) {
                    if let Some(node) = slot.take() {
                        folded.push(node);
                    }
                }
            }
            SectionSpan::Single(idx, SectionKind::Reflection) => {
                if let Some(node) = slots[*idx].take() {
                    if let MarkdownNode::Details { ref children, .. } = node {
                        reflection_nodes.extend(children.clone());
                    }
                    folded.push(node);
                }
            }
            SectionSpan::Single(idx, _) => {
                if let Some(node) = slots[*idx].take() {
                    if matches!(raw_nodes.get(*idx), Some(CourseNode::Details { summary, .. }) if is_reflection_heading(summary))
                    {
                        if let MarkdownNode::Details { ref children, .. } = node {
                            reflection_nodes.extend(children.clone());
                        }
                    }
                    folded.push(node);
                }
            }
        }
    }

    (folded, reflection_nodes)
}

pub(crate) fn infer_target_path_from_paragraph(
    children: &[CourseNode],
    current_cwd: Option<&RelWorkspacePath>,
) -> Option<RelWorkspacePath> {
    let plain = collect_plain_text(children);
    if !paragraph_introduces_file(&plain) {
        return None;
    }

    let mut inline_codes = Vec::new();
    collect_inline_codes(children, &mut inline_codes);

    for raw_code in inline_codes {
        let trimmed = raw_code.trim();
        let candidate = trimmed.strip_prefix("./").unwrap_or(trimmed);
        if !is_plausible_file_path(candidate) {
            continue;
        }
        let resolved = if !candidate.contains('/') {
            if let Some(dir) = current_cwd {
                format!("{}/{}", dir.as_str(), candidate)
            } else {
                candidate.to_owned()
            }
        } else {
            candidate.to_owned()
        };
        if let Some(rel) = normalize_rel_workspace_path(&resolved) {
            return Some(rel);
        }
    }

    None
}

fn paragraph_introduces_file(plain: &str) -> bool {
    if FILE_INTRO_ZH_KEYWORDS
        .iter()
        .any(|keyword| plain.contains(keyword))
    {
        return true;
    }
    let lower = plain.to_ascii_lowercase();
    FILE_INTRO_EN_KEYWORDS
        .iter()
        .any(|keyword| contains_ascii_word(&lower, keyword))
}

fn contains_ascii_word(haystack_lower: &str, needle_lower: &str) -> bool {
    let bytes = haystack_lower.as_bytes();
    let mut start = 0usize;
    while let Some(rel) = haystack_lower[start..].find(needle_lower) {
        let abs = start + rel;
        let end = abs + needle_lower.len();
        let left_ok = abs == 0 || !bytes[abs - 1].is_ascii_alphanumeric();
        let right_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if left_ok && right_ok {
            return true;
        }
        start = abs + needle_lower.len();
    }
    false
}

fn collect_inline_codes<'a>(nodes: &'a [CourseNode], out: &mut Vec<&'a str>) {
    for node in nodes {
        match node {
            CourseNode::InlineCode { value } => out.push(value.as_str()),
            CourseNode::Paragraph { children }
            | CourseNode::Heading { children, .. }
            | CourseNode::Strong { children }
            | CourseNode::Emphasis { children }
            | CourseNode::BlockQuote { children }
            | CourseNode::Link { children, .. }
            | CourseNode::Superscript { children }
            | CourseNode::Subscript { children }
            | CourseNode::Details { children, .. } => collect_inline_codes(children, out),
            CourseNode::List { items, .. } => {
                for item in items {
                    collect_inline_codes(&item.nodes, out);
                }
            }
            CourseNode::Table { headers, rows } => {
                for cell in headers {
                    collect_inline_codes(cell, out);
                }
                for row in rows {
                    for cell in row {
                        collect_inline_codes(cell, out);
                    }
                }
            }
            CourseNode::Text { .. }
            | CourseNode::CodeBlock { .. }
            | CourseNode::Image { .. }
            | CourseNode::LineBreak
            | CourseNode::ThematicBreak
            | CourseNode::Kbd { .. } => {}
        }
    }
}

fn is_plausible_file_path(candidate: &str) -> bool {
    if candidate.chars().any(char::is_whitespace)
        || normalize_rel_workspace_path(candidate).is_none()
    {
        return false;
    }

    let file_name = candidate.rsplit('/').next().unwrap_or(candidate);
    if PLAUSIBLE_EXACT_FILENAMES.contains(&file_name) {
        return true;
    }

    let lower = file_name.to_ascii_lowercase();
    PLAUSIBLE_FILE_EXTENSIONS
        .iter()
        .any(|ext| lower.len() > ext.len() && lower.ends_with(ext))
}

fn extract_first_sentence(text: &str) -> Option<String> {
    let normalized = normalize_whitespace(text);
    if normalized.is_empty() {
        return None;
    }

    let mut prev: Option<char> = None;
    for (idx, ch) in normalized.char_indices() {
        if matches!(ch, '。' | '！' | '？') {
            let end = idx + ch.len_utf8();
            return Some(normalized[..end].trim().to_owned());
        }

        if matches!(ch, '.' | '!' | '?') {
            let end = idx + ch.len_utf8();
            let next = normalized[end..].chars().next();
            let boundary_after = next.is_none() || next.is_some_and(char::is_whitespace);
            let standalone_dot = ch == '.' && prev.is_none_or(|p| p.is_whitespace() || p == '.');
            if boundary_after && !standalone_dot {
                return Some(normalized[..end].trim().to_owned());
            }
        }

        prev = Some(ch);
    }

    Some(normalized)
}

fn matches_section_heading(heading_text: &str, allowed: &[&str]) -> bool {
    let cleaned = heading_text.trim().trim_end_matches([':', '：', '?', '？']);
    let lower = cleaned.to_ascii_lowercase();
    allowed.iter().any(|candidate| {
        if lower == *candidate {
            return true;
        }
        if let Some(rest) = lower.strip_prefix(candidate) {
            let trimmed_rest = rest.trim();
            trimmed_rest.starts_with('(')
                || trimmed_rest.starts_with('（')
                || trimmed_rest.starts_with(':')
                || trimmed_rest.starts_with('：')
        } else {
            false
        }
    })
}

fn is_acceptance_heading(heading_text: &str) -> bool {
    matches_section_heading(heading_text, ACCEPTANCE_HEADINGS)
}

fn is_reflection_heading(heading_text: &str) -> bool {
    matches_section_heading(heading_text, REFLECTION_HEADINGS)
}

fn starts_with_ignore_ascii_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn strip_lead_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if starts_with_ignore_ascii_case(text, prefix) {
        return Some(&text[prefix.len()..]);
    }
    for marker in ["**", "__"] {
        if let Some(after_open) = text.strip_prefix(marker) {
            if starts_with_ignore_ascii_case(after_open, prefix) {
                let after_prefix = &after_open[prefix.len()..];
                if let Some(after_close) = after_prefix.strip_prefix(marker) {
                    return Some(after_close);
                }
            }
        }
    }
    None
}

fn match_acceptance_lead_paragraph(text: &str) -> Option<Option<String>> {
    for prefix in ACCEPTANCE_PREFIXES {
        if let Some(remainder) = strip_lead_prefix(text, prefix) {
            let trimmed = remainder.trim();
            return Some(if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            });
        }
    }

    if ACCEPTANCE_SUFFIXES
        .iter()
        .any(|suffix| text.ends_with(suffix))
    {
        return Some(None);
    }

    None
}

fn match_reflection_lead_paragraph(text: &str) -> Option<String> {
    for (prefix, summary) in REFLECTION_PREFIXES {
        if strip_lead_prefix(text, prefix).is_some() {
            return Some((*summary).to_owned());
        }
    }
    None
}

pub(crate) fn infer_dominant_language(code_languages: &[&str]) -> Option<Language> {
    let mut counts: BTreeMap<Language, usize> = BTreeMap::new();
    for raw in code_languages {
        let lang = match raw.to_ascii_lowercase().as_str() {
            "go" | "golang" => Some(Language::Go),
            "ts" | "tsx" | "typescript" => Some(Language::Ts),
            "js" | "jsx" | "javascript" | "mjs" | "cjs" => Some(Language::Js),
            "py" | "python" | "python3" => Some(Language::Python),
            "java" => Some(Language::Java),
            _ => None,
        };
        if let Some(mapped) = lang {
            *counts.entry(mapped).or_insert(0) += 1;
        }
    }

    let mut best: Option<(Language, usize)> = None;
    let mut tied = false;

    for (lang, count) in counts {
        match best {
            None => best = Some((lang, count)),
            Some((_, best_count)) if count > best_count => {
                best = Some((lang, count));
                tied = false;
            }
            Some((_, best_count)) if count == best_count => {
                tied = true;
            }
            Some(_) => {}
        }
    }

    if tied {
        None
    } else {
        best.map(|(lang, _)| lang)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn technical_headings_are_not_misclassified_as_acceptance_or_reflection() {
        assert!(!is_reflection_heading("Go Reflection"));
        assert!(!is_reflection_heading("Java Reflection API"));
        assert!(!is_acceptance_heading("验证健康检查接口"));

        assert!(is_reflection_heading("课后思考"));
        assert!(is_reflection_heading("Reflection"));
        assert!(is_reflection_heading("Reflection (2)"));
        assert!(is_acceptance_heading("验收清单"));
        assert!(is_acceptance_heading("Verification"));
    }

    #[test]
    fn commonmark_unparsed_cjk_bold_lead_paragraphs_are_recognized() {
        assert_eq!(
            match_acceptance_lead_paragraph("**验收：**不看代码即可写出请求。"),
            Some(Some("不看代码即可写出请求。".to_owned()))
        );
        assert_eq!(
            match_reflection_lead_paragraph("**思考：**为什么不能合并错误？"),
            Some("思考".to_owned())
        );
    }

    #[test]
    fn plausible_file_path_allows_double_dots_in_filename_and_blocks_traversal() {
        assert!(is_plausible_file_path("src/v1..v2.ts"));
        assert!(!is_plausible_file_path("../secret.ts"));
        assert!(!is_plausible_file_path("C:main.go"));
    }

    #[test]
    fn dominant_language_returns_none_on_tie() {
        assert_eq!(infer_dominant_language(&["go", "ts"]), None);
        assert_eq!(
            infer_dominant_language(&["go", "go", "ts"]),
            Some(Language::Go)
        );
    }
}

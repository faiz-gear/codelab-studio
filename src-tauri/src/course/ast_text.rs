use crate::course::ast::CourseNode;
use crate::domain::ids::RelWorkspacePath;

pub(crate) fn collect_plain_text(nodes: &[CourseNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        append_node_plain_text(node, &mut out);
    }
    out
}

pub(crate) fn collect_list_item_lead_text(nodes: &[CourseNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        if matches!(node, CourseNode::List { .. }) {
            continue;
        }
        append_node_plain_text(node, &mut out);
    }
    out
}

fn append_joined_groups(groups: &[Vec<CourseNode>], out: &mut String) {
    for group in groups {
        if !out.is_empty() && !out.ends_with(char::is_whitespace) {
            out.push(' ');
        }
        for child in group {
            append_node_plain_text(child, out);
        }
    }
}

pub(crate) fn append_node_plain_text(node: &CourseNode, out: &mut String) {
    match node {
        CourseNode::Text { value } | CourseNode::InlineCode { value } => out.push_str(value),
        CourseNode::Kbd { label } => out.push_str(label),
        CourseNode::Image { alt, .. } => out.push_str(alt),
        CourseNode::LineBreak => {
            if !out.ends_with(char::is_whitespace) && !out.is_empty() {
                out.push(' ');
            }
        }
        CourseNode::ThematicBreak => {}
        CourseNode::CodeBlock { code, .. } => out.push_str(code),
        CourseNode::Paragraph { children }
        | CourseNode::Heading { children, .. }
        | CourseNode::Strong { children }
        | CourseNode::Emphasis { children }
        | CourseNode::BlockQuote { children }
        | CourseNode::Link { children, .. }
        | CourseNode::Superscript { children }
        | CourseNode::Subscript { children } => {
            for child in children {
                append_node_plain_text(child, out);
            }
        }
        CourseNode::Details { summary, children } => {
            if !summary.is_empty() {
                out.push_str(summary);
                if !children.is_empty() {
                    out.push(' ');
                }
            }
            for child in children {
                append_node_plain_text(child, out);
            }
        }
        CourseNode::List { items, .. } => {
            for item in items {
                if !out.is_empty() && !out.ends_with(char::is_whitespace) {
                    out.push(' ');
                }
                for child in &item.nodes {
                    append_node_plain_text(child, out);
                }
            }
        }
        CourseNode::Table { headers, rows } => {
            append_joined_groups(headers, out);
            for row in rows {
                append_joined_groups(row, out);
            }
        }
    }
}

pub(crate) fn extract_fence_language(info_string: &str) -> Option<String> {
    let trimmed = info_string.trim();
    if trimmed.is_empty() {
        return None;
    }

    let first_raw = if let Some(rest) = trimmed.strip_prefix('{') {
        rest.trim_start()
            .split(|c: char| c.is_whitespace() || c == '}' || c == ',')
            .next()
            .unwrap_or("")
    } else {
        trimmed
            .split(|c: char| c.is_whitespace() || c == '{')
            .next()
            .unwrap_or("")
    };

    let candidate = first_raw.trim().trim_start_matches('.');
    if candidate.is_empty() || candidate.contains('=') {
        None
    } else {
        Some(candidate.to_owned())
    }
}

pub(crate) fn slugify_with_options(
    input: &str,
    max_chars: usize,
    lowercase_ascii: bool,
    allow_underscore: bool,
) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    let mut count = 0usize;

    for ch in input.chars() {
        if ch.is_alphanumeric() || (allow_underscore && ch == '_') {
            let normalized = if lowercase_ascii && ch.is_ascii() {
                ch.to_ascii_lowercase()
            } else {
                ch
            };
            if pending_dash && !out.is_empty() {
                if count + 1 >= max_chars {
                    break;
                }
                out.push('-');
                count += 1;
            }
            pending_dash = false;
            if count >= max_chars {
                break;
            }
            out.push(normalized);
            count += 1;
        } else {
            pending_dash = true;
        }
    }

    out.trim_matches('-').to_owned()
}

pub(crate) fn sanitize_heading_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_start_matches('#').trim();
    let cleaned = slugify_with_options(trimmed, 64, false, true);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

pub(crate) fn slugify_identifier(input: &str, max_chars: usize) -> String {
    slugify_with_options(input, max_chars, true, false)
}

pub(crate) fn normalize_rel_workspace_path(raw: &str) -> Option<RelWorkspacePath> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.contains(':') || trimmed.chars().any(char::is_control) {
        return None;
    }
    let mut candidate = trimmed;
    while let Some(rest) = candidate.strip_prefix("./") {
        candidate = rest;
    }
    RelWorkspacePath::new(candidate).ok()
}

pub(crate) fn normalize_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_ws = true;

    for ch in input.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                out.push(' ');
                prev_ws = true;
            }
        } else {
            out.push(ch);
            prev_ws = false;
        }
    }

    out.trim_end().to_owned()
}

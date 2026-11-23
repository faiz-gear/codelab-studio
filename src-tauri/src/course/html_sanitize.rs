use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use ammonia::Builder;

use crate::course::ast_text::normalize_rel_workspace_path;

static ALLOWLIST_SANITIZER: OnceLock<Builder<'static>> = OnceLock::new();
static TEXT_ONLY_SANITIZER: OnceLock<Builder<'static>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AllowedTag {
    Details,
    Summary,
    Kbd,
    Sup,
    Sub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SuppressedTag {
    Script,
    Style,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HtmlToken {
    Open(AllowedTag),
    Close(AllowedTag),
    LineBreak,
    OpenSuppressed(SuppressedTag),
    CloseSuppressed(SuppressedTag),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SingleTagAction {
    Token(HtmlToken),
    Ignore,
    FallbackToSanitizer,
}

fn allowlist_sanitizer() -> &'static Builder<'static> {
    ALLOWLIST_SANITIZER.get_or_init(|| {
        let mut builder = Builder::empty();
        builder
            .tags(HashSet::from([
                "br", "kbd", "sup", "sub", "details", "summary",
            ]))
            .generic_attributes(HashSet::new())
            .tag_attributes(HashMap::new())
            .clean_content_tags(HashSet::from(["script", "style"]))
            .link_rel(None)
            .strip_comments(true);
        builder
    })
}

fn text_only_sanitizer() -> &'static Builder<'static> {
    TEXT_ONLY_SANITIZER.get_or_init(|| {
        let mut builder = Builder::empty();
        builder
            .tags(HashSet::new())
            .generic_attributes(HashSet::new())
            .tag_attributes(HashMap::new())
            .clean_content_tags(HashSet::from(["script", "style"]))
            .link_rel(None)
            .strip_comments(true);
        builder
    })
}

pub(crate) fn sanitize_to_plain_text(raw: &str) -> String {
    let cleaned = text_only_sanitizer().clean(raw).to_string();
    decode_html_entities(&cleaned)
}

fn is_disallowed_url_char(ch: char) -> bool {
    ch.is_control()
        || matches!(
            ch,
            '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
        )
}

pub(crate) fn sanitize_link_href(raw: &str) -> Option<String> {
    let trimmed = raw.trim_matches(|c: char| c.is_whitespace() || is_disallowed_url_char(c));
    if trimmed.is_empty() || trimmed.chars().any(is_disallowed_url_char) {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || trimmed.starts_with('#')
    {
        Some(trimmed.to_owned())
    } else {
        None
    }
}

pub(crate) fn sanitize_image_src(raw: &str) -> Option<String> {
    let trimmed = raw.trim_matches(|c: char| c.is_whitespace() || is_disallowed_url_char(c));
    if trimmed.is_empty() || trimmed.chars().any(is_disallowed_url_char) {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Some(trimmed.to_owned());
    }
    if has_uri_scheme(&lower) {
        return None;
    }

    normalize_rel_workspace_path(trimmed).map(|rel| rel.into_inner())
}

fn has_uri_scheme(lower: &str) -> bool {
    let Some(colon_idx) = lower.find(':') else {
        return false;
    };
    let prefix = &lower[..colon_idx];
    let mut bytes = prefix.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

pub(crate) fn classify_inline_html_tag(raw: &str) -> SingleTagAction {
    let trimmed = raw.trim();
    let Some(inner) = trimmed.strip_prefix('<').and_then(|s| s.strip_suffix('>')) else {
        return SingleTagAction::FallbackToSanitizer;
    };
    if find_unquoted_byte(inner, |b| b == b'<' || b == b'>').is_some() {
        return SingleTagAction::FallbackToSanitizer;
    }

    let inner = inner.trim();
    if inner.is_empty() || inner.starts_with('!') || inner.starts_with('?') {
        return SingleTagAction::FallbackToSanitizer;
    }

    let (is_close, rest) = if let Some(after_slash) = inner.strip_prefix('/') {
        (true, after_slash.trim_start())
    } else {
        (false, inner)
    };

    let name_len = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric())
        .count();
    if name_len == 0 {
        return SingleTagAction::FallbackToSanitizer;
    }
    let name = &rest[..name_len];
    let after_name = &rest[name_len..];
    if after_name
        .chars()
        .next()
        .is_some_and(|ch| !ch.is_ascii_whitespace() && ch != '/')
    {
        return SingleTagAction::FallbackToSanitizer;
    }

    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "script" => {
            let token = if is_close {
                HtmlToken::CloseSuppressed(SuppressedTag::Script)
            } else {
                HtmlToken::OpenSuppressed(SuppressedTag::Script)
            };
            return SingleTagAction::Token(token);
        }
        "style" => {
            let token = if is_close {
                HtmlToken::CloseSuppressed(SuppressedTag::Style)
            } else {
                HtmlToken::OpenSuppressed(SuppressedTag::Style)
            };
            return SingleTagAction::Token(token);
        }
        "br" => return SingleTagAction::Token(HtmlToken::LineBreak),
        _ => {}
    }

    let allowed = match lower.as_str() {
        "kbd" => Some(AllowedTag::Kbd),
        "sup" => Some(AllowedTag::Sup),
        "sub" => Some(AllowedTag::Sub),
        "details" => Some(AllowedTag::Details),
        "summary" => Some(AllowedTag::Summary),
        _ => None,
    };

    let is_self_closing = !is_close && inner.trim_end().ends_with('/');
    match allowed {
        Some(tag) if is_close => SingleTagAction::Token(HtmlToken::Close(tag)),
        Some(_) if is_self_closing => SingleTagAction::Ignore,
        Some(tag) => SingleTagAction::Token(HtmlToken::Open(tag)),
        None => SingleTagAction::FallbackToSanitizer,
    }
}

pub(crate) fn sanitize_html_block_to_tokens(
    raw: &str,
    mut open_details_depth: usize,
) -> Vec<HtmlToken> {
    let mut out = Vec::new();
    let mut remaining = raw;

    while !remaining.is_empty() {
        let scan = scan_details_boundaries(remaining, open_details_depth > 0);
        if let Some((start, end)) = scan.first_unmatched_close {
            let before = &remaining[..start];
            if !before.trim().is_empty() {
                let start_len = out.len();
                let before_scan = scan_details_boundaries(before, false);
                emit_sanitized_chunk(before, before_scan.closing_details_count, &mut out);
                open_details_depth = update_details_depth(open_details_depth, &out[start_len..]);
            }
            out.push(HtmlToken::Close(AllowedTag::Details));
            open_details_depth = open_details_depth.saturating_sub(1);
            remaining = &remaining[end..];
            continue;
        }

        emit_sanitized_chunk(remaining, scan.closing_details_count, &mut out);
        break;
    }

    out
}

fn update_details_depth(initial: usize, tokens: &[HtmlToken]) -> usize {
    let mut depth = initial;
    for tok in tokens {
        match tok {
            HtmlToken::Open(AllowedTag::Details) => depth += 1,
            HtmlToken::Close(AllowedTag::Details) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

fn emit_sanitized_chunk(raw: &str, raw_closing_details: usize, out: &mut Vec<HtmlToken>) {
    let mut cleaned = allowlist_sanitizer().clean(raw).to_string();
    if cleaned.is_empty() {
        return;
    }

    let cleaned_opens = cleaned.matches("<details>").count();
    let synthetic_closes = cleaned_opens.saturating_sub(raw_closing_details);
    for _ in 0..synthetic_closes {
        let trimmed_len = cleaned.trim_end().len();
        if let Some(prefix) = cleaned[..trimmed_len].strip_suffix("</details>") {
            let new_len = prefix.len();
            cleaned.truncate(new_len);
        }
    }

    tokenize_sanitized_html(&cleaned, out);
}

fn tokenize_sanitized_html(cleaned: &str, out: &mut Vec<HtmlToken>) {
    let mut cursor = 0usize;

    while cursor < cleaned.len() {
        let rest = &cleaned[cursor..];
        let Some(lt_offset) = rest.find('<') else {
            let text = decode_html_entities(rest);
            if !text.is_empty() {
                out.push(HtmlToken::Text(text));
            }
            break;
        };

        if lt_offset > 0 {
            let text = decode_html_entities(&rest[..lt_offset]);
            if !text.is_empty() {
                out.push(HtmlToken::Text(text));
            }
        }

        let tag_start = cursor + lt_offset;
        let after_lt = &cleaned[tag_start..];
        let Some(gt_offset) = after_lt.find('>') else {
            let text = decode_html_entities(after_lt);
            if !text.is_empty() {
                out.push(HtmlToken::Text(text));
            }
            break;
        };

        let tag_token = &after_lt[..=gt_offset];
        cursor = tag_start + gt_offset + 1;

        match tag_token {
            "<details>" => out.push(HtmlToken::Open(AllowedTag::Details)),
            "</details>" => out.push(HtmlToken::Close(AllowedTag::Details)),
            "<summary>" => out.push(HtmlToken::Open(AllowedTag::Summary)),
            "</summary>" => out.push(HtmlToken::Close(AllowedTag::Summary)),
            "<br>" => out.push(HtmlToken::LineBreak),
            "<kbd>" => out.push(HtmlToken::Open(AllowedTag::Kbd)),
            "</kbd>" => out.push(HtmlToken::Close(AllowedTag::Kbd)),
            "<sup>" => out.push(HtmlToken::Open(AllowedTag::Sup)),
            "</sup>" => out.push(HtmlToken::Close(AllowedTag::Sup)),
            "<sub>" => out.push(HtmlToken::Open(AllowedTag::Sub)),
            "</sub>" => out.push(HtmlToken::Close(AllowedTag::Sub)),
            _ => {}
        }
    }
}

struct DetailsScanResult {
    first_unmatched_close: Option<(usize, usize)>,
    closing_details_count: usize,
}

fn scan_details_boundaries(raw: &str, stop_at_unmatched_close: bool) -> DetailsScanResult {
    let bytes = raw.as_bytes();
    let mut i = 0usize;
    let mut local_depth = 0usize;
    let mut closing_details_count = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }

        let slice = &raw[i..];

        if let Some(after_comment) = slice.strip_prefix("<!--") {
            if let Some(end_rel) = after_comment.find("-->") {
                i += 4 + end_rel + 3;
                continue;
            }
            break;
        }

        let Some(tag_end_rel) = find_unquoted_byte(&slice[1..], |b| b == b'>') else {
            break;
        };
        let tag_end = i + 1 + tag_end_rel + 1;
        let tag_inner = raw[i + 1..tag_end - 1].trim();

        let (is_close, name_start) = if let Some(after_slash) = tag_inner.strip_prefix('/') {
            (true, after_slash.trim_start())
        } else {
            (false, tag_inner)
        };

        let name_len = name_start
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric())
            .count();
        if name_len == 0 {
            i = tag_end;
            continue;
        }
        let name = &name_start[..name_len];
        let after_name = &name_start[name_len..];
        let valid_boundary = after_name
            .chars()
            .next()
            .is_none_or(|ch| ch.is_ascii_whitespace() || ch == '/');
        if !valid_boundary {
            i = tag_end;
            continue;
        }

        if !is_close && (name.eq_ignore_ascii_case("script") || name.eq_ignore_ascii_case("style"))
        {
            i = skip_until_closing_raw_tag(raw, tag_end, name);
            continue;
        }

        if name.eq_ignore_ascii_case("details") {
            if is_close {
                closing_details_count += 1;
                if stop_at_unmatched_close && local_depth == 0 {
                    return DetailsScanResult {
                        first_unmatched_close: Some((i, tag_end)),
                        closing_details_count,
                    };
                }
                local_depth = local_depth.saturating_sub(1);
            } else {
                local_depth += 1;
            }
        }

        i = tag_end;
    }

    DetailsScanResult {
        first_unmatched_close: None,
        closing_details_count,
    }
}

fn skip_until_closing_raw_tag(raw: &str, start: usize, tag_name: &str) -> usize {
    let bytes = raw.as_bytes();
    let mut cursor = start;
    while cursor + 2 + tag_name.len() < bytes.len() {
        if bytes[cursor] == b'<' && bytes[cursor + 1] == b'/' {
            let rest = &raw[cursor + 2..];
            let trimmed = rest.trim_start();
            let offset_ws = rest.len() - trimmed.len();
            if trimmed.len() >= tag_name.len()
                && trimmed[..tag_name.len()].eq_ignore_ascii_case(tag_name)
            {
                let after = &trimmed[tag_name.len()..];
                if after
                    .chars()
                    .next()
                    .is_some_and(|c| c == '>' || c.is_ascii_whitespace() || c == '/')
                {
                    if let Some(gt_rel) = after.find('>') {
                        return cursor + 2 + offset_ws + tag_name.len() + gt_rel + 1;
                    }
                    return raw.len();
                }
            }
        }
        cursor += 1;
    }
    raw.len()
}

fn find_unquoted_byte(slice: &str, predicate: impl Fn(u8) -> bool) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (idx, b) in slice.bytes().enumerate() {
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
            continue;
        }
        if b == b'\'' || b == b'"' {
            quote = Some(b);
            continue;
        }
        if predicate(b) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_owned();
    }

    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;

    while cursor < input.len() {
        let rest = &input[cursor..];
        let Some(amp_rel) = rest.find('&') else {
            out.push_str(rest);
            break;
        };

        out.push_str(&rest[..amp_rel]);
        let entity_start = cursor + amp_rel;
        let after_amp = &input[entity_start + 1..];

        if let Some(semi_rel) = after_amp.find(';').filter(|&pos| pos > 0 && pos <= 10) {
            let name = &after_amp[..semi_rel];
            if let Some(decoded) = decode_single_entity(name) {
                out.push(decoded);
                cursor = entity_start + 1 + semi_rel + 1;
                continue;
            }
        }

        out.push('&');
        cursor = entity_start + 1;
    }

    out
}

fn decode_single_entity(name: &str) -> Option<char> {
    match name {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => {
            let num = name.strip_prefix('#')?;
            let code_point =
                if let Some(hex) = num.strip_prefix('x').or_else(|| num.strip_prefix('X')) {
                    u32::from_str_radix(hex, 16).ok()?
                } else {
                    num.parse::<u32>().ok()?
                };
            if code_point == 0 {
                return None;
            }
            char::from_u32(code_point)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_html_entities_handles_named_numeric_and_prevents_double_unescape() {
        assert_eq!(
            decode_html_entities("a &lt; b &amp;&amp; c &gt; d"),
            "a < b && c > d"
        );
        assert_eq!(
            decode_html_entities("it&apos;s &#39;ok&#x27; &nbsp;"),
            "it's 'ok'  "
        );
        assert_eq!(
            decode_html_entities("&amp;lt;script&amp;gt;"),
            "&lt;script&gt;"
        );
    }

    #[test]
    fn script_and_style_never_self_close() {
        assert_eq!(
            classify_inline_html_tag("<script/>"),
            SingleTagAction::Token(HtmlToken::OpenSuppressed(SuppressedTag::Script))
        );
        assert_eq!(
            classify_inline_html_tag("<script src=https://evil.invalid/>"),
            SingleTagAction::Token(HtmlToken::OpenSuppressed(SuppressedTag::Script))
        );
        assert_eq!(
            classify_inline_html_tag("<style/>"),
            SingleTagAction::Token(HtmlToken::OpenSuppressed(SuppressedTag::Style))
        );
    }

    #[test]
    fn image_src_rejects_non_ascii_whitespace_uri_and_windows_drive_prefix() {
        assert_eq!(
            sanitize_image_src("\u{00A0}data:image/svg+xml,<svg onload=alert(1)>"),
            None
        );
        assert_eq!(sanitize_image_src("\u{200B}javascript:alert(1)"), None);
        assert_eq!(sanitize_image_src("C:evil.png"), None);
        assert_eq!(
            sanitize_image_src("./assets/arch.png"),
            Some("assets/arch.png".to_owned())
        );
        assert_eq!(
            sanitize_image_src("https://example.com/a.png"),
            Some("https://example.com/a.png".to_owned())
        );
    }

    #[test]
    fn details_scanner_ignores_comments_and_quoted_attributes_in_linear_time() {
        let with_comment = "<details><summary>Hint</summary>\n<!-- </details> -->\nBody";
        let tokens = sanitize_html_block_to_tokens(with_comment, 0);
        assert!(!tokens.contains(&HtmlToken::Close(AllowedTag::Details)));

        let with_attr = r#"<div title="</details>">visible</div>"#;
        let tokens = sanitize_html_block_to_tokens(with_attr, 1);
        assert!(!tokens.contains(&HtmlToken::Close(AllowedTag::Details)));
        assert!(tokens
            .iter()
            .any(|tok| matches!(tok, HtmlToken::Text(t) if t.contains("visible"))));

        let adversarial = "<script ".repeat(4000);
        let tokens = sanitize_html_block_to_tokens(&adversarial, 1);
        assert!(tokens.is_empty());

        let many_closes = "</details>".repeat(5000);
        let tokens = sanitize_html_block_to_tokens(&many_closes, 5000);
        assert_eq!(tokens.len(), 5000);
    }
}

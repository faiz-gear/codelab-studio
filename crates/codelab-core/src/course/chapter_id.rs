use std::collections::HashSet;

use icu_normalizer::ComposingNormalizer;

use crate::course::model::ChapterKind;

#[derive(Debug, Clone)]
pub(crate) struct ManifestChapterHint {
    pub chapter_id: String,
    pub title: Option<String>,
    pub kind: Option<ChapterKind>,
    pub heading_prefix: Option<String>,
    pub heading_exact: Option<String>,
    pub anchor: Option<String>,
    pub default_cwd: Option<String>,
    pub entry_file: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedHeading {
    pub clean_title: String,
    pub explicit_anchor: Option<String>,
}

pub(crate) fn split_heading_and_anchor(raw_text: &str, parsed_id: Option<&str>) -> ResolvedHeading {
    if let Some(id) = parsed_id.map(str::trim).filter(|id| is_valid_anchor(id)) {
        return ResolvedHeading {
            clean_title: raw_text.trim().to_string(),
            explicit_anchor: Some(id.to_string()),
        };
    }

    let trimmed = raw_text.trim();
    if let Some(open_idx) = trimmed.rfind("{#")
        && trimmed.ends_with('}')
    {
        let candidate = &trimmed[open_idx + 2..trimmed.len() - 1];
        if is_valid_anchor(candidate) {
            let clean_title = trimmed[..open_idx].trim_end().to_string();
            return ResolvedHeading {
                clean_title,
                explicit_anchor: Some(candidate.to_string()),
            };
        }
    }

    ResolvedHeading {
        clean_title: trimmed.to_string(),
        explicit_anchor: None,
    }
}

pub(crate) fn match_manifest_chapter<'a>(
    hints: &'a [ManifestChapterHint],
    title: &str,
    explicit_anchor: Option<&str>,
) -> Option<&'a ManifestChapterHint> {
    if let Some(anchor) = explicit_anchor
        && let Some(found) = hints
            .iter()
            .find(|h| h.anchor.as_deref() == Some(anchor) || h.chapter_id == anchor)
    {
        return Some(found);
    }

    if let Some(found) = hints
        .iter()
        .find(|h| h.heading_exact.as_deref() == Some(title))
    {
        return Some(found);
    }

    if let Some(found) = hints.iter().find(|h| {
        h.heading_prefix
            .as_deref()
            .is_some_and(|prefix| !prefix.is_empty() && title.starts_with(prefix))
    }) {
        return Some(found);
    }

    hints.iter().find(|h| h.title.as_deref() == Some(title))
}

pub(crate) fn extract_numbered_exercise_id(title: &str) -> Option<String> {
    let normalized = nfkc_normalize(title);
    let trimmed = normalized.trim();

    if let Some(rest) = trimmed
        .strip_prefix("练习")
        .or_else(|| trimmed.strip_prefix("实战"))
    {
        return parse_leading_exercise_number(rest);
    }

    let lower = trimmed.to_ascii_lowercase();
    for prefix in ["exercise", "lab", "task"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let first = rest.chars().next();
            if first
                .is_some_and(|c| c.is_whitespace() || c.is_ascii_digit() || c == '#' || c == '-')
            {
                let rest_trimmed = rest.trim_start_matches([' ', '\t', '#', '-']);
                return parse_leading_exercise_number(rest_trimmed);
            }
        }
    }

    None
}

fn parse_leading_exercise_number(input: &str) -> Option<String> {
    let trimmed = input.trim_start();
    let digit_end = trimmed
        .char_indices()
        .take_while(|&(_, ch)| ch.is_ascii_digit())
        .last()
        .map(|(idx, ch)| idx + ch.len_utf8())?;

    let digits = &trimmed[..digit_end];
    let after = &trimmed[digit_end..];
    if let Some(next_char) = after.chars().next()
        && next_char.is_alphanumeric()
    {
        return None;
    }

    let number: u32 = digits.parse().ok()?;
    Some(format!("ex-{number}"))
}

pub(crate) fn unicode_nfkc_slug(input: &str) -> String {
    let normalized = nfkc_normalize(input).to_lowercase();
    let mut slug = String::with_capacity(normalized.len());
    let mut pending_dash = false;

    for ch in normalized.chars() {
        if ch.is_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch);
            pending_dash = false;
        } else {
            pending_dash = true;
        }
    }

    if slug.is_empty() {
        "chapter".to_string()
    } else {
        slug
    }
}

pub(crate) fn resolve_chapter_id(
    manifest_hint: Option<&ManifestChapterHint>,
    explicit_anchor: Option<&str>,
    clean_title: &str,
    used_ids: &mut HashSet<String>,
) -> String {
    let base = if let Some(hint) = manifest_hint
        && !hint.chapter_id.trim().is_empty()
    {
        hint.chapter_id.trim().to_string()
    } else if let Some(anchor) = explicit_anchor.map(str::trim).filter(|a| !a.is_empty()) {
        anchor.to_string()
    } else if let Some(ex_id) = extract_numbered_exercise_id(clean_title) {
        ex_id
    } else {
        unicode_nfkc_slug(clean_title)
    };

    deduplicate_id(base, used_ids)
}

fn deduplicate_id(base: String, used_ids: &mut HashSet<String>) -> String {
    if used_ids.insert(base.clone()) {
        return base;
    }
    let mut suffix = 2usize;
    loop {
        let candidate = format!("{base}-{suffix}");
        if used_ids.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn is_valid_anchor(candidate: &str) -> bool {
    let mut chars = candidate.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_alphanumeric() {
        return false;
    }
    chars.all(|ch| ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

fn nfkc_normalize(input: &str) -> String {
    let normalizer = ComposingNormalizer::new_nfkc();
    normalizer.normalize(input).into_owned()
}

use std::collections::{HashMap, HashSet};

use crate::course::shell_analyzer::block_starts_with_root_reset_or_init;
use crate::domain::course::{
    ChapterKind, CheckSpec, ChecksSpec, CourseChapterRef, CourseManifest, MarkdownNode,
    PackValidationIssue, ParsedChapter, ParsedCourse,
};
use crate::domain::ids::{ChapterId, CommandBlockId, RelWorkspacePath};

pub(crate) fn apply_manifest_overlay(
    course: &mut ParsedCourse,
    manifest: &CourseManifest,
    validation_issues: &mut Vec<PackValidationIssue>,
) {
    course.course_id = manifest.course_id.clone();
    course.title = manifest.title.clone();
    course.language = Some(manifest.language);

    if manifest.chapters.is_empty() {
        return;
    }

    let mut matched_indices: HashSet<usize> = HashSet::new();
    let mut seen_ref_ids: HashSet<&str> = HashSet::new();

    for chapter_ref in &manifest.chapters {
        if !seen_ref_ids.insert(chapter_ref.chapter_id.0.as_str()) {
            validation_issues.push(PackValidationIssue {
                file: "codelab.json".to_owned(),
                message: format!("duplicate chapterId '{}'", chapter_ref.chapter_id.0),
            });
            continue;
        }

        let Some(idx) =
            find_matching_chapter_index(&course.chapters, &matched_indices, chapter_ref)
        else {
            validation_issues.push(PackValidationIssue {
                file: "codelab.json".to_owned(),
                message: format!("unmatched chapter ref '{}'", chapter_ref.chapter_id.0),
            });
            continue;
        };

        matched_indices.insert(idx);
        rebind_chapter_from_manifest(&mut course.chapters[idx], chapter_ref);
    }

    for (idx, chapter) in course.chapters.iter_mut().enumerate() {
        if !matched_indices.contains(&idx) {
            chapter.kind = ChapterKind::Reading;
        }
    }
}

pub(crate) fn validate_checks_against_course(
    course: &ParsedCourse,
    checks: &ChecksSpec,
    checks_file_name: &str,
    issues: &mut Vec<PackValidationIssue>,
) {
    let known_chapters: HashSet<&ChapterId> =
        course.chapters.iter().map(|ch| &ch.chapter_id).collect();

    match checks {
        ChecksSpec::MultiChapter(multi) => {
            for chapter in &multi.chapters {
                validate_chapter_checks(
                    &known_chapters,
                    &chapter.chapter_id,
                    &chapter.checks,
                    checks_file_name,
                    issues,
                );
            }
        }
        ChecksSpec::SingleChapter(single) => {
            validate_chapter_checks(
                &known_chapters,
                &single.chapter_id,
                &single.checks,
                checks_file_name,
                issues,
            );
        }
    }
}

pub(crate) fn checks_for_chapter<'a>(
    checks: Option<&'a ChecksSpec>,
    chapter_id: &ChapterId,
) -> &'a [CheckSpec] {
    let Some(spec) = checks else {
        return &[];
    };
    match spec {
        ChecksSpec::MultiChapter(multi) => multi
            .chapters
            .iter()
            .find(|ch| &ch.chapter_id == chapter_id)
            .map(|ch| ch.checks.as_slice())
            .unwrap_or(&[]),
        ChecksSpec::SingleChapter(single) => {
            if &single.chapter_id == chapter_id {
                single.checks.as_slice()
            } else {
                &[]
            }
        }
    }
}

fn find_matching_chapter_index(
    chapters: &[ParsedChapter],
    matched: &HashSet<usize>,
    chapter_ref: &CourseChapterRef,
) -> Option<usize> {
    let unmatched = || {
        chapters
            .iter()
            .enumerate()
            .filter(|(idx, _)| !matched.contains(idx))
    };

    if let Some((idx, _)) = unmatched().find(|(_, ch)| ch.chapter_id == chapter_ref.chapter_id) {
        return Some(idx);
    }

    if let Some(spec) = chapter_ref.heading_match.as_ref() {
        if let Some(anchor) = non_empty_trimmed(spec.anchor.as_deref()) {
            if let Some((idx, _)) = unmatched().find(|(_, ch)| ch.chapter_id.0 == anchor) {
                return Some(idx);
            }
        }

        if let Some(exact) = non_empty_trimmed(spec.heading_exact.as_deref()) {
            if let Some((idx, _)) = unmatched().find(|(_, ch)| ch.title.trim() == exact) {
                return Some(idx);
            }
        }

        if let Some(prefix) = non_empty_trimmed(spec.heading_prefix.as_deref()) {
            if let Some((idx, _)) = unmatched().find(|(_, ch)| ch.title.trim().starts_with(prefix))
            {
                return Some(idx);
            }
        }
    }

    let ref_title = chapter_ref.title.trim();
    if !ref_title.is_empty() {
        if let Some((idx, _)) = unmatched().find(|(_, ch)| ch.title.trim() == ref_title) {
            return Some(idx);
        }
    }

    None
}

fn non_empty_trimmed(raw: Option<&str>) -> Option<&str> {
    raw.map(str::trim).filter(|s| !s.is_empty())
}

fn rebind_chapter_from_manifest(chapter: &mut ParsedChapter, chapter_ref: &CourseChapterRef) {
    chapter.chapter_id = chapter_ref.chapter_id.clone();

    let mut id_map: HashMap<CommandBlockId, CommandBlockId> =
        HashMap::with_capacity(chapter.command_blocks.len());
    for (ord, cmd) in chapter.command_blocks.iter_mut().enumerate() {
        let old_id = cmd.id.clone();
        let new_id = CommandBlockId(format!("{}-cmd-{:02}", chapter.chapter_id.0, ord + 1));
        cmd.chapter_id = chapter.chapter_id.clone();
        cmd.id = new_id.clone();
        id_map.insert(old_id, new_id);
    }

    for id in &mut chapter.three_step.step1_primary_commands {
        if let Some(new_id) = id_map.get(id) {
            *id = new_id.clone();
        }
    }
    for id in &mut chapter.three_step.secondary_commands {
        if let Some(new_id) = id_map.get(id) {
            *id = new_id.clone();
        }
    }

    rewrite_command_ids_in_nodes(&mut chapter.nodes, &id_map);
    rewrite_command_ids_in_nodes(&mut chapter.three_step.reflection_nodes, &id_map);

    chapter.kind = chapter_ref.kind.unwrap_or(ChapterKind::Exercise);
    chapter.default_cwd = chapter_ref.default_cwd.clone();
    chapter.entry_file = chapter_ref.entry_file.clone();

    if let Some(default_cwd) = chapter_ref.default_cwd.as_ref() {
        for cmd in &mut chapter.command_blocks {
            if cmd.cwd.is_none() && !block_starts_with_root_reset_or_init(&cmd.command) {
                cmd.cwd = Some(default_cwd.clone());
            }
        }

        for ref_block in &mut chapter.three_step.step2_reference_blocks {
            if let Some(rel) = ref_block.target_path.as_ref() {
                if let Some(prefixed) = prefix_with_default_cwd(rel, default_cwd) {
                    ref_block.target_path = Some(prefixed);
                }
            }
        }

        apply_default_cwd_to_reference_nodes(&mut chapter.nodes, default_cwd);
        apply_default_cwd_to_reference_nodes(&mut chapter.three_step.reflection_nodes, default_cwd);
    }
}

fn prefix_with_default_cwd(
    rel: &RelWorkspacePath,
    default_cwd: &RelWorkspacePath,
) -> Option<RelWorkspacePath> {
    let prefix = format!("{}/", default_cwd.as_str());
    if rel.as_str().starts_with(&prefix) || rel.as_str() == default_cwd.as_str() {
        return None;
    }
    RelWorkspacePath::new(format!("{prefix}{}", rel.as_str())).ok()
}

fn visit_code_blocks_mut<F>(nodes: &mut [MarkdownNode], visitor: &mut F)
where
    F: FnMut(&mut Option<RelWorkspacePath>, &mut Option<CommandBlockId>),
{
    for node in nodes {
        match node {
            MarkdownNode::CodeBlock {
                target_path,
                command_block_id,
                ..
            } => visitor(target_path, command_block_id),
            MarkdownNode::Paragraph { children }
            | MarkdownNode::Heading { children, .. }
            | MarkdownNode::Strong { children }
            | MarkdownNode::Emphasis { children }
            | MarkdownNode::BlockQuote { children }
            | MarkdownNode::Link { children, .. }
            | MarkdownNode::Superscript { children }
            | MarkdownNode::Subscript { children }
            | MarkdownNode::Details { children, .. } => visit_code_blocks_mut(children, visitor),
            MarkdownNode::List { items, .. } => {
                for item in items {
                    visit_code_blocks_mut(item, visitor);
                }
            }
            MarkdownNode::Table { headers, rows } => {
                for cell in headers {
                    visit_code_blocks_mut(cell, visitor);
                }
                for row in rows {
                    for cell in row {
                        visit_code_blocks_mut(cell, visitor);
                    }
                }
            }
            MarkdownNode::Text { .. }
            | MarkdownNode::InlineCode { .. }
            | MarkdownNode::Image { .. }
            | MarkdownNode::LineBreak
            | MarkdownNode::ThematicBreak
            | MarkdownNode::Kbd { .. } => {}
        }
    }
}

fn rewrite_command_ids_in_nodes(
    nodes: &mut [MarkdownNode],
    id_map: &HashMap<CommandBlockId, CommandBlockId>,
) {
    visit_code_blocks_mut(nodes, &mut |_, cmd_id| {
        if let Some(id) = cmd_id.as_mut() {
            if let Some(new_id) = id_map.get(id) {
                *id = new_id.clone();
            }
        }
    });
}

fn apply_default_cwd_to_reference_nodes(
    nodes: &mut [MarkdownNode],
    default_cwd: &RelWorkspacePath,
) {
    visit_code_blocks_mut(nodes, &mut |target_path, cmd_id| {
        if cmd_id.is_none() {
            if let Some(rel) = target_path.as_mut() {
                if let Some(prefixed) = prefix_with_default_cwd(rel, default_cwd) {
                    *rel = prefixed;
                }
            }
        }
    });
}

fn validate_chapter_checks(
    known_chapters: &HashSet<&ChapterId>,
    chapter_id: &ChapterId,
    checks: &[CheckSpec],
    checks_file_name: &str,
    issues: &mut Vec<PackValidationIssue>,
) {
    if !known_chapters.contains(chapter_id) {
        issues.push(PackValidationIssue {
            file: checks_file_name.to_owned(),
            message: format!("checks reference unknown chapterId '{}'", chapter_id.0),
        });
    }

    for check in checks {
        if let CheckSpec::HttpAssert { id, request, .. } = check {
            if !is_loopback_http_url(&request.url) {
                issues.push(PackValidationIssue {
                    file: checks_file_name.to_owned(),
                    message: format!(
                        "http_assert '{}' URL '{}' must target loopback (127.0.0.1, localhost, or [::1])",
                        id, request.url
                    ),
                });
            }
        }
    }
}

fn is_loopback_http_url(raw: &str) -> bool {
    let trimmed = raw.trim();
    let Some(rest) = trimmed
        .get(..7)
        .filter(|prefix| prefix.eq_ignore_ascii_case("http://"))
        .map(|_| &trimmed[7..])
    else {
        return false;
    };

    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() || authority.contains('@') {
        return false;
    }

    if let Some(after_ipv6) = authority.strip_prefix("[::1]") {
        return is_empty_or_valid_port(after_ipv6);
    }

    let (host, port_part) = match authority.split_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (authority, None),
    };
    let is_loopback = host == "127.0.0.1" || host.eq_ignore_ascii_case("localhost");
    if !is_loopback {
        return false;
    }

    match port_part {
        None => true,
        Some(port) => {
            !port.is_empty()
                && port.bytes().all(|b| b.is_ascii_digit())
                && port.parse::<u16>().is_ok_and(|value| value > 0)
        }
    }
}

fn is_empty_or_valid_port(rest: &str) -> bool {
    if rest.is_empty() {
        return true;
    }
    let Some(port) = rest.strip_prefix(':') else {
        return false;
    };
    !port.is_empty()
        && port.bytes().all(|b| b.is_ascii_digit())
        && port.parse::<u16>().is_ok_and(|value| value > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_http_url_guard_allows_loopback_and_blocks_ssrf_variants() {
        for allowed in [
            "http://127.0.0.1:8080/healthz",
            "http://127.0.0.1/healthz",
            "http://localhost:8000/tasks",
            "http://LOCALHOST:3000/api?x=1",
            "http://[::1]:8080/healthz",
            "http://[::1]/",
        ] {
            assert!(
                is_loopback_http_url(allowed),
                "expected loopback URL '{allowed}' to be allowed"
            );
        }

        for blocked in [
            "https://127.0.0.1:8080/healthz",
            "http://169.254.169.254/latest/meta-data",
            "http://localhost.evil.com/healthz",
            "http://127.0.0.1.evil.com/healthz",
            "http://127.0.0.1@evil.com/healthz",
            "http://[::1]@evil.com/healthz",
            "http://127.0.0.1:0/healthz",
            "http://[::1]evil/healthz",
            "file:///etc/passwd",
            "",
        ] {
            assert!(
                !is_loopback_http_url(blocked),
                "expected URL '{blocked}' to be rejected"
            );
        }
    }
}

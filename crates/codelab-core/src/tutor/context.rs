use std::collections::HashSet;

use crate::course::{Chapter, ChapterKind, CheckKind, ContentBlock, LoadedCourse};
use crate::execution::verification::model::{ChapterProgress, CheckStatus, CompletionState};
use crate::workspace::{
    MAX_WORKSPACE_TEXT_BYTES, Workspace, WorkspaceFileContents, WorkspacePaths,
};

use super::backend::compose_prompt_parts;
use super::model::{
    ContextChip, ContextDimension, ContextQuote, ContextQuoteSource, ContextSnapshot,
    ContextSnapshotInput, MAX_CONTEXT_INPUT_TOKENS, TeachingStyle,
};
use super::policy::inspect_workspace_path;

const MAX_TERMINAL_ERROR_LINES: usize = 200;
const ACTIVE_FILE_CONTEXT_RADIUS: usize = 40;
const ACTIVE_FILE_DEFAULT_HEAD_LINES: usize = 120;
const TRUNCATION_MARKER: &str = "\n… [truncated for 6k token budget]";
const EARLIER_OMITTED_MARKER: &str = "… [earlier terminal error lines omitted]";

/// Conservative deterministic token estimator calibrated for mixed CJK + source code:
/// - Non-ASCII (including CJK) characters: ~1 token per char (never under-estimates Chinese)
/// - ASCII characters: ~1 token per 4 chars (rounded up)
pub(crate) fn estimate_tokens(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    let mut ascii_chars: u32 = 0;
    let mut non_ascii_chars: u32 = 0;
    for ch in text.chars() {
        if ch.is_ascii() {
            ascii_chars = ascii_chars.saturating_add(1);
        } else {
            non_ascii_chars = non_ascii_chars.saturating_add(1);
        }
    }
    non_ascii_chars.saturating_add(ascii_chars.div_ceil(4))
}

struct ChapterSection {
    title: String,
    chapter_id: String,
    body: String,
    truncated: bool,
    dropped: bool,
}

impl ChapterSection {
    fn render(&self) -> Option<String> {
        if self.dropped {
            return None;
        }
        let header = format!("### Current Chapter ({}: {})", self.chapter_id, self.title);
        if self.body.trim().is_empty() {
            Some(header)
        } else {
            Some(format!("{header}\n{}", self.body))
        }
    }

    fn trim_by(&mut self, excess_tokens: u32) {
        let body_tokens = estimate_tokens(&self.body);
        if body_tokens > excess_tokens + 24 {
            let target_tokens = body_tokens.saturating_sub(excess_tokens + 16);
            self.body = truncate_text_to_tokens(&self.body, target_tokens);
        } else {
            self.body.clear();
        }
        self.truncated = true;
    }

    fn drop_section(&mut self) {
        self.body.clear();
        self.dropped = true;
        self.truncated = true;
    }
}

struct ActiveFileSection {
    path: String,
    lines: Vec<(usize, String)>,
    focus_start: usize,
    focus_end: usize,
    truncated: bool,
    dropped: bool,
}

impl ActiveFileSection {
    fn line_range_detail(&self) -> Option<String> {
        let first = self.lines.first()?.0;
        let last = self.lines.last()?.0;
        Some(format!("L{first}-L{last}"))
    }

    fn body(&self) -> String {
        let mut out = String::new();
        for (idx, (line_no, line)) in self.lines.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            out.push_str(&format!("{line_no:>4} | {line}"));
        }
        if self.truncated && !out.is_empty() {
            out.push_str(TRUNCATION_MARKER);
        }
        out
    }

    fn render(&self) -> Option<String> {
        if self.dropped || self.lines.is_empty() {
            return None;
        }
        let range = self.line_range_detail().unwrap_or_default();
        Some(format!(
            "### Active File Snippet ({}, {})\n{}",
            self.path,
            range,
            self.body()
        ))
    }

    fn drop_farthest_line(&mut self) -> bool {
        if self.lines.len() <= 1 {
            return false;
        }
        let first_no = self.lines.first().unwrap().0;
        let last_no = self.lines.last().unwrap().0;
        let dist_top = self.focus_start.saturating_sub(first_no);
        let dist_bottom = last_no.saturating_sub(self.focus_end);
        if dist_top > dist_bottom {
            self.lines.remove(0);
        } else {
            self.lines.pop();
        }
        self.truncated = true;
        true
    }

    fn trim_single_line(&mut self, excess_tokens: u32) {
        if let Some((_, line)) = self.lines.first_mut() {
            let current = estimate_tokens(line);
            if current > excess_tokens + 16 {
                *line = truncate_text_to_tokens(line, current.saturating_sub(excess_tokens + 16));
                self.truncated = true;
            }
        }
    }

    fn drop_section(&mut self) {
        self.lines.clear();
        self.dropped = true;
        self.truncated = true;
    }
}

struct VerificationSection {
    chapter_id: String,
    label: String,
    detail: String,
    priority_lines: Vec<String>,
    /// Wire error codes for LLM prompt only (never surfaced verbatim in UI chip preview).
    llm_error_lines: Vec<String>,
    passing_lines: Vec<String>,
    raw_lines: Vec<String>,
    truncated: bool,
    dropped: bool,
}

impl VerificationSection {
    fn render_body(&self, include_error_codes: bool) -> String {
        let mut all = Vec::new();
        all.extend(self.priority_lines.iter().cloned());
        if include_error_codes {
            all.extend(self.llm_error_lines.iter().cloned());
        }
        all.extend(self.passing_lines.iter().cloned());
        all.extend(self.raw_lines.iter().cloned());
        let mut text = all.join("\n");
        if self.truncated && !text.is_empty() {
            text.push_str(TRUNCATION_MARKER);
        }
        text
    }

    fn render_for_prompt(&self) -> Option<String> {
        self.render_inner(true)
    }

    fn render_for_preview(&self) -> Option<String> {
        self.render_inner(false)
    }

    fn render_inner(&self, include_error_codes: bool) -> Option<String> {
        if self.dropped {
            return None;
        }
        let body = self.render_body(include_error_codes);
        if body.trim().is_empty() {
            return None;
        }
        Some(format!(
            "### Verification Status (chapter: {}, state: {})\n{}",
            self.chapter_id, self.detail, body
        ))
    }

    /// Drops one lowest-priority verification line at a time:
    /// raw output -> passing checks -> remaining details (keeping at least 1 summary line).
    fn drop_lowest_priority_line(&mut self) -> bool {
        if self.raw_lines.pop().is_some()
            || self.passing_lines.pop().is_some()
            || (self.priority_lines.len() > 1 && self.priority_lines.pop().is_some())
        {
            self.truncated = true;
            true
        } else {
            false
        }
    }

    fn drop_section(&mut self) {
        self.priority_lines.clear();
        self.llm_error_lines.clear();
        self.passing_lines.clear();
        self.raw_lines.clear();
        self.dropped = true;
        self.truncated = true;
    }
}

struct TerminalErrorsSection {
    lines: Vec<String>,
    truncated: bool,
}

impl TerminalErrorsSection {
    fn body(&self) -> String {
        if self.lines.is_empty() {
            return String::new();
        }
        if self.truncated {
            format!("{EARLIER_OMITTED_MARKER}\n{}", self.lines.join("\n"))
        } else {
            self.lines.join("\n")
        }
    }

    fn render(&self) -> Option<String> {
        if self.lines.is_empty() {
            return None;
        }
        Some(format!(
            "### Terminal Error Tail (latest {} lines)\n{}",
            self.lines.len(),
            self.body()
        ))
    }

    fn drop_oldest_line(&mut self) -> bool {
        if self.lines.len() <= 1 {
            return false;
        }
        self.lines.remove(0);
        self.truncated = true;
        true
    }
}

struct DraftSnapshot {
    quote: Option<ContextQuote>,
    chapter: Option<ChapterSection>,
    active_file: Option<ActiveFileSection>,
    verification: Option<VerificationSection>,
    terminal_errors: Option<TerminalErrorsSection>,
}

impl DraftSnapshot {
    fn render_preamble(&self) -> String {
        let mut sections = Vec::new();
        if let Some(quote) = &self.quote {
            sections.push(render_quote_section(quote));
        }
        if let Some(term) = self.terminal_errors.as_ref().and_then(|s| s.render()) {
            sections.push(term);
        }
        if let Some(ver) = self
            .verification
            .as_ref()
            .and_then(|s| s.render_for_prompt())
        {
            sections.push(ver);
        }
        if let Some(file) = self.active_file.as_ref().and_then(|s| s.render()) {
            sections.push(file);
        }
        if let Some(chapter) = self.chapter.as_ref().and_then(|s| s.render()) {
            sections.push(chapter);
        }
        if sections.is_empty() {
            String::new()
        } else {
            format!("[Context Snapshot]\n\n{}", sections.join("\n\n"))
        }
    }

    fn total_tokens(&self, style: TeachingStyle, prompt: &str) -> u32 {
        let preamble = self.render_preamble();
        let composed = compose_prompt_parts(
            style,
            if preamble.is_empty() {
                None
            } else {
                Some(preamble.as_str())
            },
            prompt,
        );
        estimate_tokens(&composed)
    }

    fn excess_tokens(&self, style: TeachingStyle, prompt: &str) -> u32 {
        self.total_tokens(style, prompt)
            .saturating_sub(MAX_CONTEXT_INPUT_TOKENS)
    }

    fn any_truncated(&self) -> bool {
        self.chapter.as_ref().is_some_and(|s| s.truncated)
            || self.active_file.as_ref().is_some_and(|s| s.truncated)
            || self.verification.as_ref().is_some_and(|s| s.truncated)
            || self.terminal_errors.as_ref().is_some_and(|s| s.truncated)
    }
}

fn validate_context_path(
    workspace_files: &WorkspacePaths,
    raw_path: &str,
    sensitive_filtered: &mut bool,
) -> Option<String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let check = inspect_workspace_path(workspace_files, trimmed);
    if check.sensitive {
        *sensitive_filtered = true;
        return None;
    }
    if !check.inside_workspace {
        return None;
    }
    Some(
        check
            .workspace_relative_path
            .unwrap_or_else(|| trimmed.to_owned()),
    )
}

pub(crate) fn build_context_snapshot(
    course: &LoadedCourse,
    workspace: &Workspace,
    recent_errors: &[String],
    input: &ContextSnapshotInput,
    default_style: TeachingStyle,
) -> ContextSnapshot {
    let excluded: HashSet<ContextDimension> = input.excluded_dimensions.iter().copied().collect();
    let style = input.teaching_style.unwrap_or(default_style);
    let prompt_text = input.prompt.as_deref().unwrap_or("");
    let mut sensitive_filtered = false;

    // 1. Validate quote (filter out sensitive or outside-workspace file quotes).
    let quote = input.quote.clone().and_then(|mut q| {
        q.text = q.text.trim().to_owned();
        if q.text.is_empty() {
            return None;
        }
        if let Some(raw_path) = q
            .file_path
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            let rel = validate_context_path(&workspace.files, raw_path, &mut sensitive_filtered)?;
            q.file_path = Some(rel);
        }
        Some(q)
    });

    // 2. Resolve active chapter.
    let chapter = resolve_chapter(course, input.chapter_id.as_deref());
    let resolved_chapter_id = chapter.map(|c| c.chapter_id.clone());

    let chapter_section = if !excluded.contains(&ContextDimension::Chapter) {
        chapter.map(build_chapter_section)
    } else {
        None
    };

    // 3. Build ActiveFile section with strict canonical containment & sensitive-file filtering.
    let active_file_section = if !excluded.contains(&ContextDimension::ActiveFile) {
        input
            .active_file_path
            .as_deref()
            .and_then(|raw| validate_context_path(&workspace.files, raw, &mut sensitive_filtered))
            .and_then(|rel| build_active_file_section(workspace, &rel, input, quote.as_ref()))
    } else {
        None
    };

    // 4. Build TerminalErrors section (up to latest 200 lines).
    let terminal_section = if !excluded.contains(&ContextDimension::TerminalErrors) {
        build_terminal_section(recent_errors)
    } else {
        None
    };

    // 5. Build Verification section for the active chapter.
    let verification_section = if !excluded.contains(&ContextDimension::Verification) {
        chapter.and_then(|ch| {
            let progress = workspace.progress_chapters();
            build_verification_section(ch, progress.get(&ch.chapter_id))
        })
    } else {
        None
    };

    let mut draft = DraftSnapshot {
        quote,
        chapter: chapter_section,
        active_file: active_file_section,
        verification: verification_section,
        terminal_errors: terminal_section,
    };

    // 6. Enforce strict 6,000 input token budget in bottom-up priority order:
    //    Lowest priority trimmed first: Chapter -> ActiveFile -> Verification -> older TerminalErrors.
    //    User selection (quote) & latest terminal error tail are NEVER truncated.
    enforce_token_budget(&mut draft, style, prompt_text);

    let rendered_preamble = draft.render_preamble();
    let estimated_tokens = draft.total_tokens(style, prompt_text);
    let truncated = draft.any_truncated();

    let mut chips = Vec::new();
    if let Some(ch) = &draft.chapter {
        let preview = ch.render().unwrap_or_default();
        chips.push(ContextChip {
            dimension: ContextDimension::Chapter,
            label: ch.title.clone(),
            detail: Some(ch.chapter_id.clone()),
            estimated_tokens: estimate_tokens(&preview),
            preview,
            truncated: ch.truncated,
        });
    }
    if let Some(file) = &draft.active_file {
        let preview = file.render().unwrap_or_default();
        chips.push(ContextChip {
            dimension: ContextDimension::ActiveFile,
            label: file.path.clone(),
            detail: file.line_range_detail(),
            estimated_tokens: estimate_tokens(&preview),
            preview,
            truncated: file.truncated,
        });
    }
    if let Some(term) = &draft.terminal_errors {
        let preview = term.render().unwrap_or_default();
        chips.push(ContextChip {
            dimension: ContextDimension::TerminalErrors,
            label: term.lines.len().to_string(),
            detail: None,
            estimated_tokens: estimate_tokens(&preview),
            preview,
            truncated: term.truncated,
        });
    }
    if let Some(ver) = &draft.verification {
        let preview = ver.render_for_preview().unwrap_or_default();
        let prompt_ver = ver.render_for_prompt().unwrap_or_default();
        chips.push(ContextChip {
            dimension: ContextDimension::Verification,
            label: ver.label.clone(),
            detail: Some(ver.detail.clone()),
            estimated_tokens: estimate_tokens(&prompt_ver),
            preview,
            truncated: ver.truncated,
        });
    }

    let mut excluded_sorted: Vec<ContextDimension> = excluded.into_iter().collect();
    excluded_sorted.sort();

    ContextSnapshot {
        workspace_id: input.workspace_id,
        chapter_id: resolved_chapter_id,
        quote: draft.quote,
        chips,
        excluded_dimensions: excluded_sorted,
        estimated_tokens,
        max_tokens: MAX_CONTEXT_INPUT_TOKENS,
        truncated,
        sensitive_filtered,
        rendered_preamble,
    }
}

fn enforce_token_budget(draft: &mut DraftSnapshot, style: TeachingStyle, prompt: &str) {
    // Priority 5 (lowest): Trim Chapter body first.
    let excess = draft.excess_tokens(style, prompt);
    if excess == 0 {
        return;
    }
    if let Some(chapter) = draft.chapter.as_mut() {
        chapter.trim_by(excess);
    }
    if draft.excess_tokens(style, prompt) > 0
        && let Some(chapter) = draft.chapter.as_mut()
    {
        chapter.drop_section();
    }

    // Priority 4: Trim ActiveFile lines farthest from selection/cursor first.
    while draft.excess_tokens(style, prompt) > 0 {
        if !draft
            .active_file
            .as_mut()
            .is_some_and(ActiveFileSection::drop_farthest_line)
        {
            break;
        }
    }
    let excess = draft.excess_tokens(style, prompt);
    if excess > 0
        && let Some(file) = draft.active_file.as_mut()
    {
        file.trim_single_line(excess);
    }
    if draft.excess_tokens(style, prompt) > 0
        && let Some(file) = draft.active_file.as_mut()
    {
        file.drop_section();
    }

    // Priority 3: Trim Verification (raw output -> passing checks -> details).
    while draft.excess_tokens(style, prompt) > 0 {
        if !draft
            .verification
            .as_mut()
            .is_some_and(VerificationSection::drop_lowest_priority_line)
        {
            break;
        }
    }
    if draft.excess_tokens(style, prompt) > 0
        && let Some(ver) = draft.verification.as_mut()
    {
        ver.drop_section();
    }

    // Priority 2: Trim OLDEST TerminalErrors lines from the front ONLY,
    // strictly preserving the newest error tail verbatim!
    while draft.excess_tokens(style, prompt) > 0 {
        if !draft
            .terminal_errors
            .as_mut()
            .is_some_and(TerminalErrorsSection::drop_oldest_line)
        {
            break;
        }
    }
    // Priority 1 (Quote / User Selection) & protected latest terminal error tail are NEVER truncated.
}

fn resolve_chapter<'a>(course: &'a LoadedCourse, chapter_id: Option<&str>) -> Option<&'a Chapter> {
    if let Some(id) = chapter_id.map(str::trim).filter(|s| !s.is_empty())
        && let Some(found) = course.chapters.iter().find(|ch| ch.chapter_id == id)
    {
        return Some(found);
    }
    course
        .chapters
        .iter()
        .find(|ch| ch.kind == ChapterKind::Exercise)
        .or_else(|| course.chapters.first())
}

fn build_chapter_section(chapter: &Chapter) -> ChapterSection {
    let mut parts = Vec::new();
    let kind_label = match chapter.kind {
        ChapterKind::Guide => "guide",
        ChapterKind::Exercise => "exercise",
    };
    parts.push(format!("Kind: {kind_label}"));
    if let Some(summary) = chapter.summary_html.as_deref() {
        let plain = html_to_plain_text(summary);
        if !plain.is_empty() {
            parts.push(plain);
        }
    }
    if let Some(three_step) = &chapter.three_step {
        if let Some(entry) = three_step.step2_entry_file.as_deref() {
            parts.push(format!("Entry file: {entry}"));
        }
        if let Some(step1) = three_step.step1_intro_html.as_deref() {
            let plain = html_to_plain_text(step1);
            if !plain.is_empty() {
                parts.push(plain);
            }
        }
        if let Some(step2) = three_step.step2_html.as_deref() {
            let plain = html_to_plain_text(step2);
            if !plain.is_empty() {
                parts.push(plain);
            }
        }
        if let Some(step3) = three_step.step3_intro_html.as_deref() {
            let plain = html_to_plain_text(step3);
            if !plain.is_empty() {
                parts.push(plain);
            }
        }
        if !three_step.step3_checks.is_empty() {
            let check_titles: Vec<String> = three_step
                .step3_checks
                .iter()
                .map(|c| format!("- {}", c.title))
                .collect();
            parts.push(format!("Checks:\n{}", check_titles.join("\n")));
        }
    } else {
        for block in &chapter.blocks {
            if let ContentBlock::Prose { html } = block {
                let plain = html_to_plain_text(html);
                if !plain.is_empty() {
                    parts.push(plain);
                }
            }
        }
    }
    ChapterSection {
        title: chapter.title.clone(),
        chapter_id: chapter.chapter_id.clone(),
        body: parts.join("\n\n"),
        truncated: false,
        dropped: false,
    }
}

fn build_active_file_section(
    workspace: &Workspace,
    rel_path: &str,
    input: &ContextSnapshotInput,
    quote: Option<&ContextQuote>,
) -> Option<ActiveFileSection> {
    let raw_text = if let Some(draft_text) = input.active_file_text.as_deref() {
        bounded_utf8(draft_text, MAX_WORKSPACE_TEXT_BYTES).to_owned()
    } else {
        let file = workspace.files.read(rel_path).ok()?;
        match file.contents {
            WorkspaceFileContents::Text { text } => text,
            _ => return None,
        }
    };

    let all_lines: Vec<&str> = raw_text.lines().collect();
    if all_lines.is_empty() {
        return None;
    }

    let quote_lines = quote
        .filter(|q| {
            q.source == ContextQuoteSource::Editor && q.file_path.as_deref() == Some(rel_path)
        })
        .and_then(|q| q.start_line.map(|s| (s, q.end_line.unwrap_or(s))));

    let focus = input
        .active_file_start_line
        .map(|s| (s, input.active_file_end_line.unwrap_or(s)))
        .or(quote_lines);

    let total = all_lines.len();
    let (win_start, win_end, focus_start, focus_end) = if let Some((raw_start, raw_end)) = focus {
        let fs = (raw_start.max(1) as usize).min(total);
        let fe = (raw_end.max(raw_start).max(1) as usize).min(total);
        let ws = fs.saturating_sub(ACTIVE_FILE_CONTEXT_RADIUS).max(1);
        let we = (fe + ACTIVE_FILE_CONTEXT_RADIUS).min(total);
        (ws, we, fs, fe)
    } else {
        let we = total.min(ACTIVE_FILE_DEFAULT_HEAD_LINES);
        (1, we, 1, 1)
    };

    let lines = all_lines[(win_start - 1)..win_end]
        .iter()
        .enumerate()
        .map(|(offset, line)| (win_start + offset, (*line).to_owned()))
        .collect();

    // Window extraction around the selection/cursor is NOT a 6k-budget truncation;
    // `truncated` becomes true only if `enforce_token_budget` trims this section.
    Some(ActiveFileSection {
        path: rel_path.to_owned(),
        lines,
        focus_start,
        focus_end,
        truncated: false,
        dropped: false,
    })
}

fn build_terminal_section(recent_errors: &[String]) -> Option<TerminalErrorsSection> {
    let total = recent_errors.len();
    if total == 0 {
        return None;
    }
    let start = total.saturating_sub(MAX_TERMINAL_ERROR_LINES);
    let lines: Vec<String> = recent_errors[start..].to_vec();
    Some(TerminalErrorsSection {
        lines,
        truncated: false,
    })
}

fn build_verification_section(
    chapter: &Chapter,
    progress: Option<&ChapterProgress>,
) -> Option<VerificationSection> {
    let specs = chapter
        .three_step
        .as_ref()
        .map(|s| s.step3_checks.as_slice())
        .unwrap_or_default();
    let results = progress
        .map(|p| p.last_check_results.as_slice())
        .unwrap_or_default();

    if specs.is_empty() && results.is_empty() {
        return None;
    }

    let state_label = match progress.map(|p| p.completion_state) {
        Some(CompletionState::Completed) => "completed",
        Some(CompletionState::InProgress) => "in_progress",
        _ => "not_started",
    };

    let total_checks = if !results.is_empty() {
        results.len()
    } else {
        specs
            .iter()
            .filter(|c| c.kind != CheckKind::SelfCheck)
            .count()
            .max(specs.len())
    };
    let passed_checks = results
        .iter()
        .filter(|r| r.status == CheckStatus::Passed)
        .count();

    let mut priority_lines = Vec::new();
    let mut llm_error_lines = Vec::new();
    let mut passing_lines = Vec::new();
    let mut raw_lines = Vec::new();

    if results.is_empty() {
        for spec in specs {
            passing_lines.push(format!("- [pending] {}", spec.title));
        }
    } else {
        for result in results {
            let title = specs
                .iter()
                .find(|s| s.id == result.check_id)
                .map(|s| s.title.as_str())
                .unwrap_or(result.check_id.as_str());
            let status_str = match result.status {
                CheckStatus::Running => "running",
                CheckStatus::Passed => "passed",
                CheckStatus::Failed => "failed",
                CheckStatus::TimedOut => "timedOut",
                CheckStatus::Skipped => "skipped",
            };
            let summary = format!("- [{status_str}] {title} ({}ms)", result.duration_ms);
            if result.status == CheckStatus::Passed {
                passing_lines.push(summary);
                continue;
            }
            priority_lines.push(summary);
            if let Some(err) = &result.error
                && let Ok(code) = serde_json::to_string(&err.code)
            {
                llm_error_lines.push(format!(
                    "  - errorCode ({title}): {}",
                    code.trim_matches('"')
                ));
            }
            for diff in &result.diffs {
                priority_lines.push(format!(
                    "  - diff ({}): expected `{}` | actual `{}`",
                    diff.label, diff.expected, diff.actual
                ));
            }
            for case in &result.test_cases {
                if case.status != CheckStatus::Passed {
                    priority_lines.push(format!("  - failed test: {}", case.name));
                    if !case.output.trim().is_empty() {
                        for line in case.output.lines().take(12) {
                            priority_lines.push(format!("    {line}"));
                        }
                    }
                }
            }
            if !result.raw_output.trim().is_empty() {
                raw_lines.push(format!("Raw output ({title}):"));
                for line in result.raw_output.lines() {
                    raw_lines.push(line.to_owned());
                }
            }
        }
    }

    Some(VerificationSection {
        chapter_id: chapter.chapter_id.clone(),
        label: format!("{passed_checks}/{total_checks}"),
        detail: state_label.to_owned(),
        priority_lines,
        llm_error_lines,
        passing_lines,
        raw_lines,
        truncated: false,
        dropped: false,
    })
}

fn render_quote_section(quote: &ContextQuote) -> String {
    let source = match quote.source {
        ContextQuoteSource::Manual => "manual",
        ContextQuoteSource::Editor => "editor",
        ContextQuoteSource::Terminal => "terminal",
        ContextQuoteSource::CheckFailure => "check_failure",
        ContextQuoteSource::VerificationPanel => "verification_panel",
        ContextQuoteSource::Reflection => "reflection",
        ContextQuoteSource::PortConflict => "port_conflict",
    };
    let mut meta = vec![format!("source: {source}")];
    if let Some(path) = quote.file_path.as_deref() {
        let range = match (quote.start_line, quote.end_line) {
            (Some(s), Some(e)) if s != e => format!("{path}:{s}-{e}"),
            (Some(s), _) => format!("{path}:{s}"),
            _ => path.to_owned(),
        };
        meta.push(format!("file: {range}"));
    }
    if let Some(label) = quote
        .label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        meta.push(format!("label: {label}"));
    }
    format!("### User Selection ({})\n{}", meta.join(", "), quote.text)
}

fn truncate_text_to_tokens(text: &str, max_tokens: u32) -> String {
    if estimate_tokens(text) <= max_tokens {
        return text.to_owned();
    }
    let marker_tokens = estimate_tokens(TRUNCATION_MARKER);
    let budget = max_tokens.saturating_sub(marker_tokens);
    if budget == 0 {
        return String::new();
    }
    let mut ascii_chars: u32 = 0;
    let mut non_ascii_chars: u32 = 0;
    let mut end_byte = 0;
    for (idx, ch) in text.char_indices() {
        if ch.is_ascii() {
            ascii_chars = ascii_chars.saturating_add(1);
        } else {
            non_ascii_chars = non_ascii_chars.saturating_add(1);
        }
        let next_tokens = non_ascii_chars.saturating_add(ascii_chars.div_ceil(4));
        if next_tokens > budget {
            break;
        }
        end_byte = idx + ch.len_utf8();
    }
    let prefix = text[..end_byte].trim_end();
    if prefix.is_empty() {
        String::new()
    } else {
        format!("{prefix}{TRUNCATION_MARKER}")
    }
}

fn bounded_utf8(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = 0;
    for (idx, ch) in text.char_indices() {
        let next = idx + ch.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }
    &text[..end]
}

fn html_to_plain_text(html: &str) -> String {
    let mut raw = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut tag = String::new();
            for next in chars.by_ref() {
                if next == '>' {
                    break;
                }
                tag.push(next);
            }
            let lower = tag.trim().to_ascii_lowercase();
            let name = lower
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("");
            if matches!(
                name,
                "p" | "div"
                    | "br"
                    | "li"
                    | "ul"
                    | "ol"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "pre"
                    | "blockquote"
                    | "tr"
            ) {
                raw.push('\n');
            }
        } else {
            raw.push(ch);
        }
    }
    let decoded = decode_html_entities(&raw);
    let mut out = Vec::new();
    for line in decoded.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            out.push(trimmed);
        }
    }
    out.join("\n")
}

fn decode_html_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        if let Some(semi) = rest.find(';')
            && semi <= 12
        {
            let entity = &rest[1..semi];
            let replacement = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" | "#39" | "#x27" => Some('\''),
                "nbsp" => Some(' '),
                _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                    u32::from_str_radix(&entity[2..], 16)
                        .ok()
                        .and_then(char::from_u32)
                }
                _ if entity.starts_with('#') => {
                    entity[1..].parse::<u32>().ok().and_then(char::from_u32)
                }
                _ => None,
            };
            if let Some(ch) = replacement {
                out.push(ch);
                rest = &rest[semi + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

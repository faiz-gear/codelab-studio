use crate::course::ast_text::normalize_rel_workspace_path;
use crate::domain::ids::RelWorkspacePath;

const SHELL_LANGUAGES: &[&str] = &["bash", "sh", "zsh", "shell"];

pub(crate) struct DaemonRule {
    pub prefix_tokens: &'static [&'static str],
    pub default_service_key: &'static str,
    pub default_label: &'static str,
}

const DAEMON_RULES: &[DaemonRule] = &[
    DaemonRule {
        prefix_tokens: &["go", "run"],
        default_service_key: "go-server",
        default_label: "go run",
    },
    DaemonRule {
        prefix_tokens: &["gf", "run"],
        default_service_key: "gf-server",
        default_label: "gf run",
    },
    DaemonRule {
        prefix_tokens: &["npm", "run", "dev"],
        default_service_key: "npm-dev",
        default_label: "npm run dev",
    },
    DaemonRule {
        prefix_tokens: &["npm", "start"],
        default_service_key: "npm-start",
        default_label: "npm start",
    },
    DaemonRule {
        prefix_tokens: &["pnpm", "run", "dev"],
        default_service_key: "pnpm-dev",
        default_label: "pnpm run dev",
    },
    DaemonRule {
        prefix_tokens: &["pnpm", "dev"],
        default_service_key: "pnpm-dev",
        default_label: "pnpm dev",
    },
    DaemonRule {
        prefix_tokens: &["yarn", "dev"],
        default_service_key: "yarn-dev",
        default_label: "yarn dev",
    },
    DaemonRule {
        prefix_tokens: &["bun", "run", "dev"],
        default_service_key: "bun-dev",
        default_label: "bun run dev",
    },
    DaemonRule {
        prefix_tokens: &["bun", "dev"],
        default_service_key: "bun-dev",
        default_label: "bun dev",
    },
    DaemonRule {
        prefix_tokens: &["uvicorn"],
        default_service_key: "uvicorn",
        default_label: "uvicorn",
    },
    DaemonRule {
        prefix_tokens: &["flask", "run"],
        default_service_key: "flask",
        default_label: "flask run",
    },
    DaemonRule {
        prefix_tokens: &["python", "-m", "http.server"],
        default_service_key: "python-http",
        default_label: "python -m http.server",
    },
    DaemonRule {
        prefix_tokens: &["python3", "-m", "http.server"],
        default_service_key: "python-http",
        default_label: "python3 -m http.server",
    },
    DaemonRule {
        prefix_tokens: &["mvn", "spring-boot:run"],
        default_service_key: "spring-boot",
        default_label: "mvn spring-boot:run",
    },
    DaemonRule {
        prefix_tokens: &["./gradlew", "bootRun"],
        default_service_key: "spring-boot",
        default_label: "./gradlew bootRun",
    },
    DaemonRule {
        prefix_tokens: &["gradle", "bootRun"],
        default_service_key: "spring-boot",
        default_label: "gradle bootRun",
    },
    DaemonRule {
        prefix_tokens: &["vite"],
        default_service_key: "vite",
        default_label: "vite",
    },
    DaemonRule {
        prefix_tokens: &["next", "dev"],
        default_service_key: "next-dev",
        default_label: "next dev",
    },
];

const TEST_RUNNER_RULES: &[&[&str]] = &[
    &["go", "test"],
    &["bun", "test"],
    &["npm", "test"],
    &["npm", "run", "test"],
    &["pnpm", "test"],
    &["pnpm", "run", "test"],
    &["yarn", "test"],
    &["pytest"],
    &["python", "-m", "pytest"],
    &["python3", "-m", "pytest"],
    &["mvn", "test"],
    &["./gradlew", "test"],
    &["gradle", "test"],
    &["cargo", "test"],
];

const UV_RUN_VALUE_FLAGS: &[&str] = &[
    "-p",
    "--python",
    "--extra",
    "--group",
    "--only-group",
    "--no-group",
    "--with",
    "--with-editable",
    "--with-requirements",
    "--directory",
    "--project",
    "--package",
    "--env-file",
    "--script",
    "--module",
    "-m",
    "-C",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExplicitCommandMode {
    Daemon,
    Oneshot,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FenceAttributes {
    pub cwd: Option<RelWorkspacePath>,
    pub target_path: Option<RelWorkspacePath>,
    pub mode: Option<ExplicitCommandMode>,
    pub port: Option<u16>,
    pub label: Option<String>,
    pub service_key: Option<String>,
    pub secondary: Option<bool>,
}

pub(crate) struct ShellBlockAnalysis {
    pub first_segment: Option<String>,
    pub post_cd_cwd: Option<RelWorkspacePath>,
    pub matched_daemon: Option<(String, &'static DaemonRule)>,
    pub is_test_runner: bool,
    pub is_pure_cd: bool,
    pub command_port: Option<u16>,
}

struct ParsedSegment {
    raw: String,
    words: Vec<String>,
    effective_start: usize,
}

impl ParsedSegment {
    fn effective_words(&self) -> &[String] {
        &self.words[self.effective_start..]
    }
}

pub(crate) fn is_shell_language(language: Option<&str>) -> bool {
    language.is_some_and(|lang| {
        SHELL_LANGUAGES
            .iter()
            .any(|shell| lang.eq_ignore_ascii_case(shell))
    })
}

pub(crate) fn block_starts_with_root_reset_cd(script: &str) -> bool {
    let raw_segments = extract_shell_segments(script);
    let Some(first_raw) = raw_segments.first() else {
        return false;
    };
    let words = tokenize_shell_words(first_raw);
    let effective_start = count_leading_env_assignments(&words);
    let effective = &words[effective_start..];
    if !effective
        .first()
        .is_some_and(|w| w.eq_ignore_ascii_case("cd"))
    {
        return false;
    }

    match extract_cd_target_arg(&effective[1..]) {
        None => true,
        Some(raw_arg) => {
            let trimmed = raw_arg.trim();
            trimmed.starts_with('/')
                || trimmed.starts_with('~')
                || trimmed.starts_with('$')
                || trimmed.starts_with('\\')
                || (trimmed.len() >= 2
                    && trimmed.as_bytes()[0].is_ascii_alphabetic()
                    && trimmed.as_bytes()[1] == b':')
        }
    }
}

pub(crate) fn analyze_shell_block(
    script: &str,
    initial_cwd: Option<&RelWorkspacePath>,
) -> ShellBlockAnalysis {
    let raw_segments = extract_shell_segments(script);
    let parsed: Vec<ParsedSegment> = raw_segments
        .into_iter()
        .map(|raw| {
            let words = tokenize_shell_words(&raw);
            let effective_start = count_leading_env_assignments(&words);
            ParsedSegment {
                raw,
                words,
                effective_start,
            }
        })
        .collect();

    let mut post_cd_cwd = initial_cwd.cloned();
    let mut matched_daemon: Option<(String, &'static DaemonRule)> = None;
    let mut is_test_runner = false;
    let mut is_pure_cd = !parsed.is_empty();
    let mut command_port: Option<u16> = None;

    for seg in &parsed {
        let effective = seg.effective_words();
        let is_cd = effective
            .first()
            .is_some_and(|w| w.eq_ignore_ascii_case("cd"));
        if is_cd {
            let arg = extract_cd_target_arg(&effective[1..]);
            post_cd_cwd = match arg {
                Some(raw_arg) => resolve_relative_cd(post_cd_cwd.as_ref(), raw_arg),
                None => None,
            };
        } else {
            is_pure_cd = false;
        }

        if matched_daemon.is_none() {
            if let Some(rule) = match_daemon_words(effective) {
                matched_daemon = Some((seg.raw.clone(), rule));
            }
        }
        if !is_test_runner && is_test_runner_words(effective) {
            is_test_runner = true;
        }
        if command_port.is_none() {
            command_port = extract_port_from_words(&seg.words);
        }
    }

    ShellBlockAnalysis {
        first_segment: parsed.first().map(|s| s.raw.clone()),
        post_cd_cwd,
        matched_daemon,
        is_test_runner,
        is_pure_cd,
        command_port,
    }
}

pub(crate) fn parse_fence_attributes(info_string: &str) -> FenceAttributes {
    let mut attrs = FenceAttributes::default();
    for token in tokenize_fence_info(info_string) {
        let Some((raw_key, raw_value)) = token.split_once('=') else {
            continue;
        };
        let key = raw_key.trim().to_ascii_lowercase();
        let value = unquote_attr_value(raw_value.trim());
        if value.is_empty() {
            continue;
        }

        match key.as_str() {
            "cwd" => attrs.cwd = normalize_rel_workspace_path(&value),
            "path" | "file" | "target" => {
                attrs.target_path = normalize_rel_workspace_path(&value);
            }
            "mode" => {
                if value.eq_ignore_ascii_case("daemon") {
                    attrs.mode = Some(ExplicitCommandMode::Daemon);
                } else if value.eq_ignore_ascii_case("oneshot") {
                    attrs.mode = Some(ExplicitCommandMode::Oneshot);
                }
            }
            "port" => attrs.port = value.parse::<u16>().ok().filter(|p| *p > 0),
            "label" => {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    attrs.label = Some(trimmed.to_owned());
                }
            }
            "service_key" | "servicekey" => {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    attrs.service_key = Some(trimmed.to_owned());
                }
            }
            "secondary" => {
                if let Some(flag) = parse_bool_attr(&value) {
                    attrs.secondary = Some(flag);
                }
            }
            "primary" => {
                if let Some(flag) = parse_bool_attr(&value) {
                    attrs.secondary = Some(!flag);
                }
            }
            _ => {}
        }
    }
    attrs
}

fn tokenize_fence_info(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if let Some(active_quote) = quote {
            if ch == '\\' && active_quote == '"' {
                if let Some(escaped) = chars.next() {
                    current.push(ch);
                    current.push(escaped);
                }
            } else {
                current.push(ch);
                if ch == active_quote {
                    quote = None;
                }
            }
            continue;
        }

        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                current.push(ch);
            }
            c if matches!(c, '{' | '}' | ',') || c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn unquote_attr_value(raw: &str) -> String {
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        let inner = &raw[1..raw.len() - 1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else {
                out.push(ch);
            }
        }
        return out;
    }
    if raw.len() >= 2 && raw.starts_with('\'') && raw.ends_with('\'') {
        return raw[1..raw.len() - 1].to_owned();
    }
    raw.to_owned()
}

fn parse_bool_attr(value: &str) -> Option<bool> {
    if value.eq_ignore_ascii_case("true") || value == "1" {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") || value == "0" {
        Some(false)
    } else {
        None
    }
}

fn extract_shell_segments(script: &str) -> Vec<String> {
    let mut logical_lines = Vec::new();
    let mut pending_line = String::new();
    let mut active_heredoc: Option<String> = None;

    for raw_line in script.lines() {
        if let Some(delim) = &active_heredoc {
            if raw_line.trim() == delim {
                active_heredoc = None;
            }
            continue;
        }

        if pending_line.is_empty() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
        }

        let trimmed_end = raw_line.trim_end();
        if let Some(without_slash) = trimmed_end.strip_suffix('\\') {
            pending_line.push_str(without_slash);
            pending_line.push(' ');
            continue;
        }

        pending_line.push_str(raw_line);
        let completed = std::mem::take(&mut pending_line);
        let trimmed = completed.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(delim) = detect_heredoc_delimiter(trimmed) {
            active_heredoc = Some(delim);
        }
        logical_lines.push(completed);
    }

    if !pending_line.trim().is_empty() && !pending_line.trim_start().starts_with('#') {
        logical_lines.push(pending_line);
    }

    let mut segments = Vec::new();
    for line in logical_lines {
        split_line_into_segments(line.trim(), &mut segments);
    }
    segments
}

fn detect_heredoc_delimiter(line: &str) -> Option<String> {
    let mut in_single = false;
    let mut in_double = false;
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut prev_was_ws = true;

    while i + 1 < bytes.len() {
        let b = bytes[i];
        if in_single {
            if b == b'\'' {
                in_single = false;
            }
            prev_was_ws = false;
            i += 1;
            continue;
        }
        if in_double {
            if b == b'\\' {
                i += 2;
                prev_was_ws = false;
                continue;
            }
            if b == b'"' {
                in_double = false;
            }
            prev_was_ws = false;
            i += 1;
            continue;
        }

        match b {
            b'\'' => in_single = true,
            b'"' => in_double = true,
            b'#' if prev_was_ws => break,
            b'<' if bytes[i + 1] == b'<' => {
                if i + 2 < bytes.len() && bytes[i + 2] == b'<' {
                    i += 3;
                    prev_was_ws = false;
                    continue;
                }
                let mut rest = line[i + 2..].trim_start();
                if let Some(stripped) = rest.strip_prefix('-') {
                    rest = stripped.trim_start();
                }
                let token_end = rest
                    .find(|c: char| c.is_whitespace() || matches!(c, ';' | '&' | '|' | ')' | '#'))
                    .unwrap_or(rest.len());
                let delim = rest[..token_end].trim_matches(|c| c == '\'' || c == '"');
                if !delim.is_empty() {
                    return Some(delim.to_owned());
                }
            }
            _ => {}
        }
        prev_was_ws = b.is_ascii_whitespace();
        i += 1;
    }
    None
}

fn split_line_into_segments(line: &str, out: &mut Vec<String>) {
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = line.chars().peekable();
    let mut prev_was_ws = true;
    let mut prev_char: Option<char> = None;

    while let Some(ch) = chars.next() {
        if in_single {
            current.push(ch);
            if ch == '\'' {
                in_single = false;
            }
            prev_was_ws = false;
            prev_char = Some(ch);
            continue;
        }
        if in_double {
            current.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    current.push(next);
                    prev_char = Some(next);
                }
            } else {
                if ch == '"' {
                    in_double = false;
                }
                prev_char = Some(ch);
            }
            prev_was_ws = false;
            continue;
        }

        match ch {
            '\'' => {
                in_single = true;
                current.push(ch);
                prev_was_ws = false;
                prev_char = Some(ch);
            }
            '"' => {
                in_double = true;
                current.push(ch);
                prev_was_ws = false;
                prev_char = Some(ch);
            }
            '\\' => {
                current.push(ch);
                if let Some(next) = chars.next() {
                    current.push(next);
                    prev_char = Some(next);
                }
                prev_was_ws = false;
            }
            '#' if prev_was_ws => break,
            ';' => {
                push_trimmed_segment(&mut current, out);
                prev_was_ws = true;
                prev_char = Some(';');
            }
            '|' => {
                if chars.peek() == Some(&'|') {
                    chars.next();
                }
                push_trimmed_segment(&mut current, out);
                prev_was_ws = true;
                prev_char = Some('|');
            }
            '&' => {
                let is_redirection =
                    matches!(prev_char, Some('>') | Some('<')) || chars.peek() == Some(&'>');
                if is_redirection {
                    current.push(ch);
                    prev_was_ws = false;
                    prev_char = Some('&');
                } else {
                    if chars.peek() == Some(&'&') {
                        chars.next();
                    }
                    push_trimmed_segment(&mut current, out);
                    prev_was_ws = true;
                    prev_char = Some('&');
                }
            }
            _ => {
                prev_was_ws = ch.is_whitespace();
                current.push(ch);
                prev_char = Some(ch);
            }
        }
    }

    push_trimmed_segment(&mut current, out);
}

fn push_trimmed_segment(current: &mut String, out: &mut Vec<String>) {
    let seg = current.trim();
    if !seg.is_empty() {
        out.push(seg.to_owned());
    }
    current.clear();
}

fn tokenize_shell_words(segment: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = segment.chars();

    while let Some(ch) = chars.next() {
        if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                current.push(ch);
            }
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
            } else if ch == '\\' {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '\'' => in_single = true,
            '"' => in_double = true,
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            c if c.is_ascii_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

fn count_leading_env_assignments(words: &[String]) -> usize {
    let mut idx = 0usize;
    while idx < words.len() && is_env_assignment(&words[idx]) {
        idx += 1;
    }
    idx
}

fn is_env_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn unwrap_runner_prefix(words: &[String]) -> &[String] {
    if words.is_empty() {
        return words;
    }
    if (words[0].eq_ignore_ascii_case("npx") || words[0].eq_ignore_ascii_case("bunx"))
        && words.len() > 1
    {
        return &words[1..];
    }
    if words[0].eq_ignore_ascii_case("pnpm")
        && words.get(1).is_some_and(|w| w.eq_ignore_ascii_case("exec"))
        && words.len() > 2
    {
        return &words[2..];
    }
    if words[0].eq_ignore_ascii_case("uv")
        && words.get(1).is_some_and(|w| w.eq_ignore_ascii_case("run"))
        && words.len() > 2
    {
        let mut idx = 2usize;
        while idx < words.len() {
            let arg = words[idx].as_str();
            if arg == "--" {
                idx += 1;
                break;
            }
            if !arg.starts_with('-') {
                break;
            }
            if !arg.contains('=') && UV_RUN_VALUE_FLAGS.contains(&arg) {
                idx += 2;
            } else {
                idx += 1;
            }
        }
        return if idx < words.len() {
            &words[idx..]
        } else {
            &[]
        };
    }
    words
}

fn words_match_prefix(words: &[String], prefix: &[&str]) -> bool {
    words.len() >= prefix.len()
        && words
            .iter()
            .zip(prefix.iter())
            .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
}

fn match_daemon_words(effective: &[String]) -> Option<&'static DaemonRule> {
    if let Some(rule) = DAEMON_RULES
        .iter()
        .find(|rule| words_match_prefix(effective, rule.prefix_tokens))
    {
        return Some(rule);
    }

    let unwrapped = unwrap_runner_prefix(effective);
    if unwrapped.len() != effective.len() {
        return DAEMON_RULES
            .iter()
            .find(|rule| words_match_prefix(unwrapped, rule.prefix_tokens));
    }

    None
}

fn is_test_runner_words(effective: &[String]) -> bool {
    if TEST_RUNNER_RULES
        .iter()
        .any(|rule| words_match_prefix(effective, rule))
    {
        return true;
    }

    let unwrapped = unwrap_runner_prefix(effective);
    unwrapped.len() != effective.len()
        && TEST_RUNNER_RULES
            .iter()
            .any(|rule| words_match_prefix(unwrapped, rule))
}

fn extract_cd_target_arg(args: &[String]) -> Option<&str> {
    let mut after_double_dash = false;
    for arg in args {
        if !after_double_dash {
            if arg == "--" {
                after_double_dash = true;
                continue;
            }
            if arg == "-L" || arg == "-P" {
                continue;
            }
        }
        return Some(arg.as_str());
    }
    None
}

fn resolve_relative_cd(
    current: Option<&RelWorkspacePath>,
    raw_arg: &str,
) -> Option<RelWorkspacePath> {
    let trimmed = raw_arg.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('~')
        || trimmed == "-"
        || trimmed.contains(':')
        || trimmed.contains('$')
        || trimmed.contains('`')
        || trimmed.contains('\\')
    {
        return None;
    }

    let mut parts: Vec<&str> = current
        .map(|path| path.as_str().split('/').collect())
        .unwrap_or_default();

    for seg in trimmed.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }

    if parts.is_empty() {
        None
    } else {
        RelWorkspacePath::new(parts.join("/")).ok()
    }
}

fn extract_port_from_words(words: &[String]) -> Option<u16> {
    for (idx, word) in words.iter().enumerate() {
        if (word == "--port" || word == "-p") && idx + 1 < words.len() {
            if let Some(port) = parse_valid_port(&words[idx + 1]) {
                return Some(port);
            }
        }
        if let Some(val) = word
            .strip_prefix("--port=")
            .or_else(|| word.strip_prefix("-p="))
        {
            if let Some(port) = parse_valid_port(val) {
                return Some(port);
            }
        }
        if let Some((env_name, env_val)) = word.split_once('=') {
            let is_port_env = env_name.eq_ignore_ascii_case("port")
                || (env_name.len() >= 5
                    && env_name[env_name.len() - 5..].eq_ignore_ascii_case("_port"));
            if is_port_env {
                if let Some(port) = parse_valid_port(env_val) {
                    return Some(port);
                }
            }
        }
        if word.eq_ignore_ascii_case("http.server") && idx + 1 < words.len() {
            if let Some(port) = parse_valid_port(&words[idx + 1]) {
                return Some(port);
            }
        }
        if let Some(port) = extract_colon_port_from_word(word) {
            return Some(port);
        }
    }
    None
}

fn extract_colon_port_from_word(word: &str) -> Option<u16> {
    if let Some(port) = extract_loopback_port_from_text(word) {
        return Some(port);
    }

    let candidate = word.split_once('=').map_or(word, |(_, rhs)| rhs);
    let after_colon = candidate
        .strip_prefix(':')
        .or_else(|| candidate.strip_prefix("0.0.0.0:"))?;
    let digits_len = after_colon
        .bytes()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if digits_len == 0 {
        return None;
    }
    let trailing = &after_colon[digits_len..];
    if trailing
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric())
    {
        return None;
    }
    parse_valid_port(&after_colon[..digits_len])
}

pub(crate) fn extract_loopback_port_from_text(text: &str) -> Option<u16> {
    let lower = text.to_ascii_lowercase();
    for needle in ["127.0.0.1:", "localhost:"] {
        let mut search_from = 0usize;
        while let Some(rel_idx) = lower[search_from..].find(needle) {
            let digits_start = search_from + rel_idx + needle.len();
            let rest = &lower[digits_start..];
            let digits_len = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
            if digits_len > 0 {
                if let Some(port) = parse_valid_port(&rest[..digits_len]) {
                    return Some(port);
                }
            }
            search_from = digits_start;
        }
    }
    None
}

fn parse_valid_port(raw: &str) -> Option<u16> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    trimmed.parse::<u16>().ok().filter(|port| *port > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_run_value_flags_work_for_both_daemons_and_tests_without_false_positives() {
        let daemon =
            analyze_shell_block("uv run --extra dev uvicorn app.main:app --port 8000", None);
        assert!(daemon.matched_daemon.is_some());
        assert_eq!(daemon.command_port, Some(8000));

        let test = analyze_shell_block("uv run --extra dev pytest -q", None);
        assert!(test.is_test_runner);
        assert!(test.matched_daemon.is_none());

        let echo = analyze_shell_block("uv run python -c \"echo pytest\"", None);
        assert!(!echo.is_test_runner);
    }

    #[test]
    fn heredoc_ignores_comment_lines_and_handles_chained_delimiters() {
        let with_comment = "# cat << EOF\ngo test ./...\n";
        let analysis = analyze_shell_block(with_comment, None);
        assert!(analysis.is_test_runner);

        let chained = "cat <<EOF; go run .\nhello\nEOF\n";
        let chained_analysis = analyze_shell_block(chained, None);
        assert!(chained_analysis.matched_daemon.is_some());
    }

    #[test]
    fn fence_attrs_and_cd_normalize_dot_slash_and_reject_drive_prefix() {
        let attrs = parse_fence_attributes("ts path=./src/index.ts cwd=./web-app");
        assert_eq!(
            attrs.target_path.as_ref().map(RelWorkspacePath::as_str),
            Some("src/index.ts")
        );
        assert_eq!(
            attrs.cwd.as_ref().map(RelWorkspacePath::as_str),
            Some("web-app")
        );

        let drive_attrs = parse_fence_attributes("go path=C:main.go cwd=C:task-api");
        assert_eq!(drive_attrs.target_path, None);
        assert_eq!(drive_attrs.cwd, None);

        let base = RelWorkspacePath::new("task-api").unwrap();
        assert_eq!(resolve_relative_cd(Some(&base), "C:other"), None);
    }

    #[test]
    fn colon_port_extracts_trailing_slash_and_ignores_redirections() {
        let analysis = analyze_shell_block("go run . --addr=:8080/ 2>&1", None);
        assert!(analysis.matched_daemon.is_some());
        assert_eq!(analysis.command_port, Some(8080));
    }

    #[test]
    fn root_reset_cd_detects_abs_tilde_bare_without_breaking_relative_parent_cd() {
        assert!(block_starts_with_root_reset_cd(
            "cd /abs/path && gf init task-api-gf && cd task-api-gf && go run main.go",
        ));
        assert!(block_starts_with_root_reset_cd("cd ~/work && go run ."));
        assert!(block_starts_with_root_reset_cd("cd"));
        assert!(block_starts_with_root_reset_cd("cd C:\\work && go run ."));
        assert!(!block_starts_with_root_reset_cd(
            "cd .. && gf init task-api-gf"
        ));
        assert!(!block_starts_with_root_reset_cd("cd sub && go run ."));
        assert!(!block_starts_with_root_reset_cd("go test ./..."));
    }
}

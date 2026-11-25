use std::collections::BTreeMap;

use crate::course::model::{CommandBlock, CommandMode, ValueOrigin};

#[derive(Debug, Default, Clone)]
pub(crate) struct FenceMeta {
    pub lang: String,
    pub attrs: BTreeMap<String, String>,
}

pub(crate) fn parse_fence_info(info: &str) -> FenceMeta {
    let trimmed = info.trim();
    if trimmed.is_empty() {
        return FenceMeta::default();
    }

    let lang_end = trimmed
        .find(|c: char| c.is_whitespace() || c == ',' || c == '{')
        .unwrap_or(trimmed.len());
    let lang_end = if trimmed.starts_with("file=") {
        0
    } else {
        lang_end
    };
    let lang = trimmed[..lang_end].to_ascii_lowercase();
    let rest = trimmed[lang_end..]
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim();

    let mut attrs = BTreeMap::new();
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b','
        {
            i += 1;
        }
        let key = rest[key_start..i].trim().to_ascii_lowercase();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }
            let value = if bytes[i] == b'"' || bytes[i] == b'\'' {
                let quote = bytes[i];
                i += 1;
                let val_start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                let slice = &rest[val_start..i];
                if i < bytes.len() && bytes[i] == quote {
                    i += 1;
                }
                slice.to_string()
            } else {
                let val_start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b',' {
                    i += 1;
                }
                rest[val_start..i].to_string()
            };
            if !key.is_empty() {
                attrs.insert(key, value);
            }
        } else if !key.is_empty() {
            attrs.insert(key, "true".to_string());
        }
    }

    FenceMeta { lang, attrs }
}

pub(crate) fn is_command_shell(lang: &str) -> bool {
    matches!(lang, "bash" | "sh" | "zsh")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedCwdAttr {
    WorkspaceRoot,
    Subdir(String),
}

fn parse_explicit_cwd(raw: &str) -> Option<ParsedCwdAttr> {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'').trim();
    if trimmed == "." || trimmed == "./" {
        return Some(ParsedCwdAttr::WorkspaceRoot);
    }
    sanitize_relative_cwd(trimmed).map(ParsedCwdAttr::Subdir)
}

pub(crate) fn analyze_command_block(
    id: String,
    meta: &FenceMeta,
    raw_code: &str,
    chapter_default_cwd: Option<&str>,
    chapter_context_cwd: &mut Option<String>,
) -> Option<CommandBlock> {
    let command = raw_code.trim();
    if command.is_empty() {
        return None;
    }

    let explicit_cwd = meta
        .attrs
        .get("cwd")
        .and_then(|raw| parse_explicit_cwd(raw));

    let leading_cd = extract_leading_cd(command);
    let starts_with_mkdir = starts_with_mkdir_command(command);
    let standalone_last_cd = extract_last_standalone_cd(command);
    let resets_cd = contains_non_relative_cd(command);

    let (cwd, cwd_origin) = if let Some(explicit) = explicit_cwd {
        match explicit {
            ParsedCwdAttr::WorkspaceRoot => (None, ValueOrigin::Explicit),
            ParsedCwdAttr::Subdir(dir) => (Some(dir), ValueOrigin::Explicit),
        }
    } else if let Some((dir, is_standalone_line)) = leading_cd {
        if is_standalone_line {
            *chapter_context_cwd = Some(dir.clone());
        }
        (Some(dir), ValueOrigin::Inferred)
    } else if starts_with_mkdir {
        if let Some(dir) = chapter_default_cwd.and_then(sanitize_relative_cwd) {
            (Some(dir), ValueOrigin::ChapterDefault)
        } else {
            (None, ValueOrigin::WorkspaceRoot)
        }
    } else if let Some(dir) = chapter_context_cwd.clone() {
        (Some(dir), ValueOrigin::Inferred)
    } else if let Some(dir) = chapter_default_cwd.and_then(sanitize_relative_cwd) {
        (Some(dir), ValueOrigin::ChapterDefault)
    } else {
        (None, ValueOrigin::WorkspaceRoot)
    };

    // Only blocks without an explicit `cwd` override update chapter-scoped directory carry-over.
    if !meta.attrs.contains_key("cwd") {
        if resets_cd {
            *chapter_context_cwd = None;
        }
        if let Some(dir) = standalone_last_cd {
            *chapter_context_cwd = Some(dir);
        }
    }

    let (mode, mode_origin) = match meta.attrs.get("mode").map(|s| s.to_ascii_lowercase()) {
        Some(ref v) if v == "daemon" => (CommandMode::Daemon, ValueOrigin::Explicit),
        Some(ref v) if v == "oneshot" => (CommandMode::Oneshot, ValueOrigin::Explicit),
        _ => (infer_command_mode(command), ValueOrigin::Inferred),
    };

    let explicit_role = meta.attrs.get("role").map(|s| s.to_ascii_lowercase());
    let is_secondary = match explicit_role.as_deref() {
        Some("debug" | "secondary") => true,
        Some("primary") => false,
        _ => mode == CommandMode::Oneshot && is_heuristic_secondary_debug_command(command),
    };

    let label = meta
        .attrs
        .get("label")
        .or_else(|| meta.attrs.get("title"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let port = meta
        .attrs
        .get("port")
        .and_then(|raw| raw.trim().parse::<u16>().ok());

    Some(CommandBlock {
        id,
        shell: meta.lang.clone(),
        command: command.to_string(),
        cwd,
        cwd_origin,
        mode,
        mode_origin,
        label,
        port,
        is_secondary,
    })
}

pub(crate) fn sanitize_relative_cwd(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'').trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == "./" {
        return None;
    }
    if trimmed.starts_with('/')
        || trimmed.starts_with('~')
        || trimmed.starts_with('$')
        || trimmed.starts_with('-')
        || trimmed.contains(':')
    {
        return None;
    }
    let normalized = trimmed.trim_start_matches("./").trim_end_matches('/');
    if normalized.is_empty() {
        return None;
    }
    if normalized
        .split(['/', '\\'])
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return None;
    }
    Some(normalized.replace('\\', "/"))
}

fn extract_leading_cd(command: &str) -> Option<(String, bool)> {
    for raw_line in command.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let dir = parse_cd_from_statement(line)?;
        let is_standalone = !line.contains("&&") && !line.contains(';') && !line.contains('|');
        return Some((dir, is_standalone));
    }
    None
}

fn starts_with_mkdir_command(command: &str) -> bool {
    for raw_line in command.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        return line.starts_with("mkdir ") || line.starts_with("gf init ");
    }
    false
}

fn extract_last_standalone_cd(command: &str) -> Option<String> {
    let mut last = None;
    for raw_line in command.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.contains("&&")
            && !line.contains(';')
            && !line.contains('|')
            && let Some(dir) = parse_cd_from_statement(line)
        {
            last = Some(dir);
        }
    }
    last
}

fn contains_non_relative_cd(command: &str) -> bool {
    for raw_line in command.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for segment in line.split("&&").flat_map(|s| s.split(';')) {
            let trimmed = segment.trim();
            if let Some(rest) = trimmed.strip_prefix("cd")
                && rest.starts_with(char::is_whitespace)
                && parse_cd_from_statement(trimmed).is_none()
            {
                return true;
            }
        }
    }
    false
}

fn parse_cd_from_statement(stmt: &str) -> Option<String> {
    let rest = stmt.strip_prefix("cd")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let arg_part = rest.trim_start();
    // Stop at && or ; or | or #
    let first_clause = arg_part
        .split("&&")
        .next()
        .unwrap_or(arg_part)
        .split(';')
        .next()
        .unwrap_or(arg_part)
        .split('|')
        .next()
        .unwrap_or(arg_part)
        .split('#')
        .next()
        .unwrap_or(arg_part)
        .trim();

    if first_clause.is_empty() {
        return None;
    }

    let target = if let Some(quoted) = first_clause.strip_prefix('"') {
        quoted.split('"').next()?
    } else if let Some(quoted) = first_clause.strip_prefix('\'') {
        quoted.split('\'').next()?
    } else {
        first_clause.split_whitespace().next()?
    };

    sanitize_relative_cwd(target)
}

fn infer_command_mode(command: &str) -> CommandMode {
    for raw_line in command.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for segment in line.split("&&").flat_map(|s| s.split(';')) {
            let seg = segment.trim();
            if seg.is_empty() {
                continue;
            }
            if is_daemon_segment(seg) {
                return CommandMode::Daemon;
            }
        }
    }
    CommandMode::Oneshot
}

fn is_daemon_segment(seg: &str) -> bool {
    let tokens: Vec<&str> = seg.split_whitespace().collect();
    if tokens.is_empty() {
        return false;
    }

    // Skip leading env assignments like PORT=8080 go run .
    let start = tokens
        .iter()
        .position(|tok| !tok.contains('=') || tok.starts_with('-'))
        .unwrap_or(0);
    let cmd = &tokens[start..];
    if cmd.is_empty() {
        return false;
    }

    let joined = cmd.join(" ").to_ascii_lowercase();

    match cmd[0] {
        "go" => cmd.get(1) == Some(&"run"),
        "gf" => cmd.get(1) == Some(&"run"),
        "air" | "nodemon" | "uvicorn" | "gunicorn" | "vite" => true,
        "npm" | "pnpm" | "yarn" | "bun" => {
            if cmd.contains(&"--watch") || cmd.contains(&"-w") {
                return true;
            }
            matches!(
                (cmd.get(1).copied(), cmd.get(2).copied()),
                (Some("dev" | "start" | "serve"), _)
                    | (Some("run"), Some("dev" | "start" | "serve" | "watch"))
            )
        }
        "npx" | "pnpx" | "bunx" => {
            matches!(
                cmd.get(1).copied(),
                Some("vite" | "nodemon" | "next" | "tsx" | "serve")
            ) && !joined.contains("build")
        }
        "flask" => cmd.get(1) == Some(&"run"),
        "fastapi" => matches!(cmd.get(1).copied(), Some("dev" | "run")),
        "python" | "python3" => {
            joined.contains("-m http.server")
                || joined.contains("runserver")
                || joined.contains("-m uvicorn")
        }
        "node" | "deno" => cmd.contains(&"--watch"),
        "mvn" | "./mvnw" => joined.contains("spring-boot:run"),
        "gradle" | "./gradlew" => joined.contains("bootrun"),
        _ => cmd.contains(&"--watch") || cmd.contains(&"--serve"),
    }
}

fn is_heuristic_secondary_debug_command(command: &str) -> bool {
    let mut non_empty_lines = command
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));

    let Some(first) = non_empty_lines.next() else {
        return false;
    };

    // Treat a block as a secondary/debug command only when every command line is a probing/debug tool.
    std::iter::once(first).chain(non_empty_lines).all(|line| {
        // Continuation lines of a multiline command (e.g. curl \ -H ... -d ...)
        if line.starts_with('-') || line.starts_with('\'') || line.starts_with('"') {
            return true;
        }
        let first_tok = line.split_whitespace().next().unwrap_or("");
        matches!(
            first_tok,
            "curl" | "wget" | "http" | "lsof" | "ps" | "netstat" | "ss" | "sqlite3" | "dig"
        )
    })
}

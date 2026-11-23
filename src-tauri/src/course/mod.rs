mod ast;
mod ast_text;
mod html_sanitize;
mod markdown_parser;
mod section_extractor;
mod shell_analyzer;
mod step_extractor;

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crate::domain::course::ParsedCourse;
use crate::domain::error::{AppError, ErrorCode};
use crate::domain::runtime::Language;

pub const MAX_MARKDOWN_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParseMarkdownOptions {
    pub course_id: Option<String>,
    pub default_title: Option<String>,
    pub language_hint: Option<Language>,
}

pub fn parse_markdown(
    source: &str,
    options: &ParseMarkdownOptions,
) -> Result<ParsedCourse, AppError> {
    if source.len() > MAX_MARKDOWN_BYTES {
        return Err(markdown_limit_error(source.len() as u64));
    }

    let raw = markdown_parser::parse_raw_markdown(source);
    Ok(step_extractor::extract_course(raw, options))
}

pub fn parse_markdown_file(
    path: &Path,
    options: &ParseMarkdownOptions,
) -> Result<ParsedCourse, AppError> {
    let file = File::open(path).map_err(|err| map_io_error(path, err))?;
    let metadata = file.metadata().map_err(|err| map_io_error(path, err))?;

    if !metadata.is_file() {
        return Err(
            AppError::new(ErrorCode::SchemaValidationFailed, "error.invalidInput")
                .with_param("field", path.display().to_string())
                .with_detail("markdown path is not a regular file"),
        );
    }

    if metadata.len() > MAX_MARKDOWN_BYTES as u64 {
        return Err(markdown_limit_error(metadata.len()));
    }

    let capacity = usize::try_from(metadata.len())
        .unwrap_or(0)
        .min(MAX_MARKDOWN_BYTES);
    let mut bytes = Vec::with_capacity(capacity);
    file.take((MAX_MARKDOWN_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|err| map_io_error(path, err))?;

    if bytes.len() > MAX_MARKDOWN_BYTES {
        return Err(markdown_limit_error(bytes.len() as u64));
    }

    let source = String::from_utf8(bytes).map_err(|err| {
        AppError::new(
            ErrorCode::FileTooLargeOrBinary,
            "error.fileTooLargeOrBinary",
        )
        .with_param("path", path.display().to_string())
        .with_detail(format!("markdown file is not valid UTF-8: {err}"))
    })?;

    let mut effective_options = options.clone();
    if effective_options.default_title.is_none() {
        effective_options.default_title = derive_fallback_title(path);
    }

    parse_markdown(&source, &effective_options)
}

fn derive_fallback_title(path: &Path) -> Option<String> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if let Some(stem_str) = stem {
        let is_generic =
            stem_str.eq_ignore_ascii_case("readme") || stem_str.eq_ignore_ascii_case("index");
        if !is_generic {
            return Some(stem_str.to_owned());
        }
    }

    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .or_else(|| stem.map(str::to_owned))
}

fn markdown_limit_error(actual_bytes: u64) -> AppError {
    AppError::new(
        ErrorCode::MarkdownLimitExceeded,
        "error.markdownLimitExceeded",
    )
    .with_param("maxBytes", MAX_MARKDOWN_BYTES.to_string())
    .with_param("actualBytes", actual_bytes.to_string())
    .with_detail(format!(
        "markdown size {actual_bytes} bytes exceeds limit of {MAX_MARKDOWN_BYTES} bytes"
    ))
}

fn map_io_error(path: &Path, err: io::Error) -> AppError {
    let (code, message_key, param_key, param_val) = match err.kind() {
        io::ErrorKind::PermissionDenied => (
            ErrorCode::PathOutsideSandbox,
            "error.unknown",
            "detail",
            format!("{}: {err}", path.display()),
        ),
        io::ErrorKind::NotFound => (
            ErrorCode::SchemaValidationFailed,
            "error.invalidInput",
            "field",
            path.display().to_string(),
        ),
        _ => (
            ErrorCode::FileTooLargeOrBinary,
            "error.unknown",
            "detail",
            format!("{}: {err}", path.display()),
        ),
    };
    AppError::new(code, message_key)
        .with_param(param_key, param_val)
        .with_detail(err.to_string())
        .with_retryable(matches!(
            err.kind(),
            io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
        ))
}

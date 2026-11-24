use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::course::OpenedCourse;
use crate::course::chapter_id::{ManifestChapterHint, unicode_nfkc_slug};
use crate::course::model::{
    ChapterKind, CheckKind, CourseCheckSpec, CourseSource, DaemonRequirement, LoadedCourse,
    MAX_MARKDOWN_BYTES,
};
use crate::course::parser::{ChapterChecksOverlay, ParseOverlay, parse_markdown_course};
use crate::error::{DomainError, ErrorCode};
use crate::model::CourseLanguage;

pub(crate) const SUPPORTED_PACK_SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawCourseManifest {
    pub schema_version: String,
    pub course_id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub language: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    #[serde(default = "default_entry_file")]
    pub entry_file: String,
    #[serde(default)]
    pub checks_file: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub chapters: Vec<RawManifestChapter>,
}

fn default_entry_file() -> String {
    "README.md".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawManifestChapter {
    pub chapter_id: String,
    pub title: Option<String>,
    pub kind: Option<String>,
    #[serde(rename = "match")]
    pub matcher: Option<RawChapterMatch>,
    pub default_cwd: Option<String>,
    pub entry_file: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawChapterMatch {
    pub heading_prefix: Option<String>,
    pub heading_exact: Option<String>,
    pub anchor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChecksFile {
    schema_version: String,
    #[serde(default)]
    chapters: Vec<RawChapterChecks>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChapterChecks {
    chapter_id: String,
    requires_daemon: Option<RawDaemonRequirement>,
    #[serde(default)]
    checks: Vec<RawCheckSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDaemonRequirement {
    label: String,
    port: u16,
    service_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCheckSpec {
    id: String,
    title: String,
    #[serde(rename = "type")]
    check_type: String,
    #[serde(default)]
    required: Option<bool>,
    cwd: Option<String>,
    command: Option<String>,
    request: Option<RawHttpRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawHttpRequest {
    method: Option<String>,
    url: Option<String>,
}

pub(crate) fn load_local_markdown_file(raw_path: &str) -> Result<OpenedCourse, DomainError> {
    let canonical = resolve_existing_path(raw_path)?;
    if !canonical.is_file() {
        return Err(DomainError::new(ErrorCode::CoursePathInvalid)
            .with_detail("path", raw_path.to_string()));
    }

    let markdown = read_bounded_markdown(&canonical)?;
    let overlay = ParseOverlay::default();
    let parsed = parse_markdown_course(&markdown, &overlay);

    let file_stem = canonical
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("course");
    let course_id = format!("local-md-{}", unicode_nfkc_slug(file_stem));
    let title = parsed.title.unwrap_or_else(|| file_stem.to_string());
    let canonical_str = canonical.to_string_lossy().into_owned();
    let total_exercises = parsed
        .chapters
        .iter()
        .filter(|ch| ch.kind == ChapterKind::Exercise)
        .count() as u32;

    let course = LoadedCourse {
        course_id,
        title,
        description: String::new(),
        language: parsed.inferred_language,
        version: None,
        author: None,
        dependencies: Vec::new(),
        source: CourseSource::LocalMarkdown {
            path: canonical_str.clone(),
        },
        entry_markdown_path: Some(canonical_str),
        is_builtin: false,
        intro_html: parsed.intro_html,
        total_exercises,
        chapters: parsed.chapters,
    };

    Ok(OpenedCourse {
        course,
        watched_files: vec![canonical],
    })
}

pub(crate) fn load_local_course_pack(raw_path: &str) -> Result<OpenedCourse, DomainError> {
    let canonical_root = resolve_existing_path(raw_path)?;
    if !canonical_root.is_dir() {
        return Err(DomainError::new(ErrorCode::CoursePathInvalid)
            .with_detail("path", raw_path.to_string()));
    }

    let manifest_path = match resolve_contained_file(&canonical_root, "codelab.json") {
        Ok(path) => path,
        Err(err) if err.code == ErrorCode::CourseNotFound => {
            return Err(DomainError::new(ErrorCode::CourseManifestInvalid)
                .with_detail("reasonCode", "MISSING_MANIFEST"));
        }
        Err(err) => return Err(err),
    };

    let manifest_bytes = fs::read(&manifest_path).map_err(|_| {
        DomainError::new(ErrorCode::CourseManifestInvalid)
            .with_detail("reasonCode", "MANIFEST_READ_FAILED")
    })?;
    let manifest = parse_manifest_bytes(&manifest_bytes)?;

    let entry_path = resolve_contained_file(&canonical_root, &manifest.entry_file)?;
    let markdown = read_bounded_markdown(&entry_path)?;

    let mut watched_files = vec![manifest_path, entry_path.clone()];

    let checks_filename = manifest
        .checks_file
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let checks_bytes = if let Some(rel_checks) = checks_filename {
        let checks_path = resolve_contained_file(&canonical_root, rel_checks)?;
        watched_files.push(checks_path.clone());
        Some(fs::read(&checks_path).map_err(|_| {
            DomainError::new(ErrorCode::CourseChecksInvalid)
                .with_detail("reasonCode", "CHECKS_READ_FAILED")
        })?)
    } else {
        let default_checks = canonical_root.join(".codelab-checks.json");
        if default_checks.exists() || default_checks.is_symlink() {
            let safe_checks = resolve_contained_file(&canonical_root, ".codelab-checks.json")?;
            watched_files.push(safe_checks.clone());
            Some(fs::read(&safe_checks).map_err(|_| {
                DomainError::new(ErrorCode::CourseChecksInvalid)
                    .with_detail("reasonCode", "CHECKS_READ_FAILED")
            })?)
        } else {
            None
        }
    };

    let course = build_course_from_pack_parts(
        manifest,
        &markdown,
        checks_bytes.as_deref(),
        CourseSource::LocalPack {
            path: canonical_root.to_string_lossy().into_owned(),
        },
        Some(entry_path.to_string_lossy().into_owned()),
        false,
    )?;

    Ok(OpenedCourse {
        course,
        watched_files,
    })
}

pub(crate) fn build_course_from_pack_parts(
    manifest: RawCourseManifest,
    markdown: &str,
    checks_bytes: Option<&[u8]>,
    source: CourseSource,
    entry_markdown_path: Option<String>,
    is_builtin: bool,
) -> Result<LoadedCourse, DomainError> {
    if markdown.len() > MAX_MARKDOWN_BYTES {
        return Err(DomainError::new(ErrorCode::CourseMarkdownTooLarge)
            .with_detail("maxMb", "5")
            .with_detail("actualBytes", markdown.len().to_string()));
    }

    let checks_by_chapter = match checks_bytes {
        Some(bytes) => parse_checks_bytes(bytes)?,
        None => HashMap::new(),
    };

    let mut chapter_hints = Vec::with_capacity(manifest.chapters.len());
    for raw_ch in manifest.chapters {
        let chapter_id = raw_ch.chapter_id.trim().to_string();
        if chapter_id.is_empty() {
            return Err(DomainError::new(ErrorCode::CourseManifestInvalid)
                .with_detail("reasonCode", "EMPTY_CHAPTER_ID"));
        }
        let kind = match raw_ch.kind.as_deref().map(str::trim) {
            None | Some("") => None,
            Some("guide") => Some(ChapterKind::Guide),
            Some("exercise") => Some(ChapterKind::Exercise),
            Some(other) => {
                return Err(DomainError::new(ErrorCode::CourseManifestInvalid)
                    .with_detail("reasonCode", "UNSUPPORTED_CHAPTER_KIND")
                    .with_detail("value", other.to_string()));
            }
        };
        chapter_hints.push(ManifestChapterHint {
            chapter_id,
            title: raw_ch.title,
            kind,
            heading_prefix: raw_ch
                .matcher
                .as_ref()
                .and_then(|m| m.heading_prefix.clone()),
            heading_exact: raw_ch
                .matcher
                .as_ref()
                .and_then(|m| m.heading_exact.clone()),
            anchor: raw_ch.matcher.as_ref().and_then(|m| m.anchor.clone()),
            default_cwd: raw_ch.default_cwd,
            entry_file: raw_ch.entry_file,
        });
    }

    let overlay = ParseOverlay {
        has_pack_metadata: true,
        chapter_hints,
        checks_by_chapter,
    };

    let parsed = parse_markdown_course(markdown, &overlay);
    let language = match manifest.language.as_deref() {
        Some(raw_lang) => Some(CourseLanguage::from_token(raw_lang).ok_or_else(|| {
            DomainError::new(ErrorCode::CourseManifestInvalid)
                .with_detail("reasonCode", "UNSUPPORTED_LANGUAGE")
                .with_detail("value", raw_lang.trim().to_string())
        })?),
        None => parsed.inferred_language,
    };

    let total_exercises = parsed
        .chapters
        .iter()
        .filter(|ch| ch.kind == ChapterKind::Exercise)
        .count() as u32;

    Ok(LoadedCourse {
        course_id: manifest.course_id.trim().to_string(),
        title: manifest.title.trim().to_string(),
        description: manifest.description.trim().to_string(),
        language,
        version: manifest.version,
        author: manifest.author,
        dependencies: manifest.dependencies,
        source,
        entry_markdown_path,
        is_builtin,
        intro_html: parsed.intro_html,
        total_exercises,
        chapters: parsed.chapters,
    })
}

pub(crate) fn parse_manifest_bytes(bytes: &[u8]) -> Result<RawCourseManifest, DomainError> {
    let manifest: RawCourseManifest = serde_json::from_slice(bytes).map_err(|_| {
        DomainError::new(ErrorCode::CourseManifestInvalid).with_detail("reasonCode", "INVALID_JSON")
    })?;
    if manifest.schema_version.trim() != SUPPORTED_PACK_SCHEMA_VERSION {
        return Err(DomainError::new(ErrorCode::CourseManifestInvalid)
            .with_detail("reasonCode", "UNSUPPORTED_SCHEMA_VERSION")
            .with_detail("schemaVersion", manifest.schema_version));
    }
    if manifest.course_id.trim().is_empty() || manifest.title.trim().is_empty() {
        return Err(DomainError::new(ErrorCode::CourseManifestInvalid)
            .with_detail("reasonCode", "EMPTY_REQUIRED_FIELD"));
    }
    Ok(manifest)
}

fn parse_checks_bytes(bytes: &[u8]) -> Result<HashMap<String, ChapterChecksOverlay>, DomainError> {
    let raw: RawChecksFile = serde_json::from_slice(bytes).map_err(|_| {
        DomainError::new(ErrorCode::CourseChecksInvalid).with_detail("reasonCode", "INVALID_JSON")
    })?;
    if raw.schema_version.trim() != SUPPORTED_PACK_SCHEMA_VERSION {
        return Err(DomainError::new(ErrorCode::CourseChecksInvalid)
            .with_detail("reasonCode", "UNSUPPORTED_SCHEMA_VERSION")
            .with_detail("schemaVersion", raw.schema_version));
    }

    let mut map = HashMap::new();
    for ch in raw.chapters {
        let chapter_id = ch.chapter_id.trim().to_string();
        if chapter_id.is_empty() {
            return Err(DomainError::new(ErrorCode::CourseChecksInvalid)
                .with_detail("reasonCode", "EMPTY_CHAPTER_ID"));
        }
        let requires_daemon = ch.requires_daemon.map(|d| DaemonRequirement {
            label: d.label,
            port: d.port,
            service_key: d.service_key,
        });

        let mut checks = Vec::with_capacity(ch.checks.len());
        for c in ch.checks {
            let kind = match c.check_type.trim() {
                "http_assert" => CheckKind::HttpAssert,
                "command_test" => CheckKind::CommandTest,
                "test_suite" => CheckKind::TestSuite,
                "self_check" => CheckKind::SelfCheck,
                other => {
                    return Err(DomainError::new(ErrorCode::CourseChecksInvalid)
                        .with_detail("reasonCode", "UNSUPPORTED_CHECK_TYPE")
                        .with_detail("value", other.to_string()));
                }
            };
            let required = c.required.unwrap_or(kind != CheckKind::SelfCheck);
            checks.push(CourseCheckSpec {
                id: c.id,
                title: c.title,
                kind,
                required,
                cwd: c.cwd,
                command: c.command,
                method: c.request.as_ref().and_then(|r| r.method.clone()),
                url: c.request.as_ref().and_then(|r| r.url.clone()),
            });
        }

        map.insert(
            chapter_id,
            ChapterChecksOverlay {
                requires_daemon,
                checks,
            },
        );
    }
    Ok(map)
}

fn resolve_existing_path(raw_path: &str) -> Result<PathBuf, DomainError> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err(DomainError::new(ErrorCode::CoursePathInvalid)
            .with_detail("path", raw_path.to_string()));
    }
    let path = Path::new(trimmed);
    if !path.is_absolute() {
        return Err(
            DomainError::new(ErrorCode::CoursePathInvalid).with_detail("path", trimmed.to_string())
        );
    }
    if !path.exists() {
        return Err(
            DomainError::new(ErrorCode::CourseNotFound).with_detail("source", trimmed.to_string())
        );
    }
    fs::canonicalize(path).map_err(|_| {
        DomainError::new(ErrorCode::CoursePathInvalid).with_detail("path", trimmed.to_string())
    })
}

fn resolve_contained_file(
    canonical_root: &Path,
    relative_file: &str,
) -> Result<PathBuf, DomainError> {
    let rel = Path::new(relative_file.trim());
    if relative_file.trim().is_empty() || rel.is_absolute() {
        return Err(DomainError::new(ErrorCode::CoursePathInvalid)
            .with_detail("path", relative_file.to_string()));
    }
    for comp in rel.components() {
        if matches!(
            comp,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(DomainError::new(ErrorCode::CoursePathInvalid)
                .with_detail("path", relative_file.to_string()));
        }
    }
    let joined = canonical_root.join(rel);
    let is_symlink = joined.is_symlink();
    if !joined.exists() && !is_symlink {
        return Err(DomainError::new(ErrorCode::CourseNotFound)
            .with_detail("source", relative_file.to_string()));
    }
    let canonical_target = fs::canonicalize(&joined).map_err(|_| {
        DomainError::new(ErrorCode::CoursePathInvalid)
            .with_detail("path", relative_file.to_string())
    })?;
    if !canonical_target.starts_with(canonical_root) || !canonical_target.is_file() {
        return Err(DomainError::new(ErrorCode::CoursePathInvalid)
            .with_detail("path", relative_file.to_string()));
    }
    Ok(canonical_target)
}

fn read_bounded_markdown(path: &Path) -> Result<String, DomainError> {
    let meta = fs::metadata(path).map_err(|_| {
        DomainError::new(ErrorCode::CourseMarkdownInvalid)
            .with_detail("path", path.to_string_lossy().into_owned())
    })?;
    if meta.len() > MAX_MARKDOWN_BYTES as u64 {
        return Err(DomainError::new(ErrorCode::CourseMarkdownTooLarge)
            .with_detail("maxMb", "5")
            .with_detail("actualBytes", meta.len().to_string()));
    }
    let bytes = fs::read(path).map_err(|_| {
        DomainError::new(ErrorCode::CourseMarkdownInvalid)
            .with_detail("path", path.to_string_lossy().into_owned())
    })?;
    if bytes.len() > MAX_MARKDOWN_BYTES {
        return Err(DomainError::new(ErrorCode::CourseMarkdownTooLarge)
            .with_detail("maxMb", "5")
            .with_detail("actualBytes", bytes.len().to_string()));
    }
    String::from_utf8(bytes).map_err(|_| {
        DomainError::new(ErrorCode::CourseMarkdownInvalid)
            .with_detail("path", path.to_string_lossy().into_owned())
    })
}

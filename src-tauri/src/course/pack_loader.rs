use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::course::pack_fs::{
    canonicalize_pack_dir, invalid_input_error, join_rel_path, resolve_contained_path,
};
use crate::course::pack_fs::{sandbox_error, schema_error};
use crate::course::pack_overlay::{
    apply_manifest_overlay, checks_for_chapter, validate_checks_against_course,
};
use crate::course::starter_hydrator::resolve_starter_dir;
use crate::course::{
    derive_fallback_title, map_io_error, parse_markdown, read_markdown_file_utf8,
    ParseMarkdownOptions,
};
use crate::domain::course::{
    ChapterKind, CheckSpec, ChecksSpec, CommandSummary, CourseChapterPreview, CourseInspectResult,
    CourseManifest, PackValidationIssue, ParsedCourse,
};
use crate::domain::error::{AppError, ErrorCode};
use crate::domain::ids::RelWorkspacePath;
use crate::domain::runtime::Language;
use crate::domain::workspace::{BuiltinCourseSummary, CourseSourceRef};

pub use crate::course::starter_hydrator::{hydrate_starter_into_workspace, HydrationReport};

pub const MAX_PACK_METADATA_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedCoursePack {
    pub source: CourseSourceRef,
    pub source_key: String,
    pub pack_fingerprint: String,
    pub requires_trust: bool,
    pub pack_root: PathBuf,
    pub entry_file_abs: PathBuf,
    pub starter_dir_abs: Option<PathBuf>,
    pub starter_fingerprints: BTreeMap<RelWorkspacePath, [u8; 32]>,
    pub manifest: Option<CourseManifest>,
    pub checks: Option<ChecksSpec>,
    pub course: ParsedCourse,
    pub validation_issues: Vec<PackValidationIssue>,
}

enum FingerprintSource<'a> {
    Bytes(&'a [u8]),
    File {
        path: PathBuf,
        starter_path: RelWorkspacePath,
    },
}

pub fn resolve_default_builtin_courses_dir(resource_dir: Option<&Path>) -> Option<PathBuf> {
    if let Ok(env_dir) = std::env::var("CODELAB_BUILTIN_COURSES_DIR") {
        let trimmed = env_dir.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }

    if let Some(dir) = resource_dir {
        let candidate = dir.join("builtin-courses");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }

    if let Some(candidate) = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|parent| parent.join("builtin-courses"))
    {
        if candidate.is_dir() {
            return Some(candidate);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(candidate) = exe.parent().map(|dir| dir.join("builtin-courses")) {
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn list_builtin_courses(builtin_dir: Option<&Path>) -> Vec<BuiltinCourseSummary> {
    let Some(dir) = builtin_dir.filter(|p| p.is_dir()) else {
        return Vec::new();
    };

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!(
                "codelab warning: failed to read built-in courses dir '{}': {err}",
                dir.display()
            );
            return Vec::new();
        }
    };

    let mut course_dirs = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                eprintln!("codelab warning: failed to inspect built-in course entry: {err}");
                continue;
            }
        };
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        if let Some(name) = entry.file_name().to_str().map(str::to_owned) {
            course_dirs.push(name);
        }
    }
    course_dirs.sort();

    let mut summaries = Vec::new();
    for course_id in course_dirs {
        match load_builtin_pack(dir, &course_id) {
            Ok(pack) => {
                let entry_file = match &pack.source {
                    CourseSourceRef::Builtin { entry_file, .. } => entry_file.clone(),
                    _ => RelWorkspacePath::new("README.md").expect("static path valid"),
                };
                let chapter_count = count_effective_chapters(&pack.course);
                let description = pack
                    .manifest
                    .as_ref()
                    .map(|m| m.description.clone())
                    .unwrap_or_default();
                let language = pack
                    .manifest
                    .as_ref()
                    .map(|m| m.language)
                    .or(pack.course.language)
                    .unwrap_or(Language::Go);
                summaries.push(BuiltinCourseSummary {
                    course_id,
                    title: pack.course.title,
                    description,
                    language,
                    chapter_count,
                    entry_file,
                });
            }
            Err(err) => {
                eprintln!("codelab warning: skipping invalid built-in course '{course_id}': {err}");
            }
        }
    }

    summaries
}

pub fn load_builtin_pack(
    builtin_dir: &Path,
    course_id: &str,
) -> Result<LoadedCoursePack, AppError> {
    if !is_safe_single_segment(course_id) {
        return Err(sandbox_error(
            "courseId",
            format!("invalid built-in course id '{course_id}'"),
        ));
    }

    let canonical_builtin = canonicalize_pack_dir(builtin_dir, "builtinCoursesDir")?;
    let candidate = canonical_builtin.join(course_id);
    let meta = fs::symlink_metadata(&candidate).map_err(|err| {
        if err.kind() == io::ErrorKind::NotFound {
            invalid_input_error(
                "courseId",
                format!("built-in course '{course_id}' not found"),
            )
        } else {
            map_io_error(&candidate, err)
        }
    })?;

    if meta.file_type().is_symlink() {
        return Err(sandbox_error(
            "courseId",
            format!("symlink rejected for built-in course '{course_id}'"),
        ));
    }
    if !meta.is_dir() {
        return Err(invalid_input_error(
            "courseId",
            format!("built-in course '{course_id}' is not a directory"),
        ));
    }

    let canonical_pack =
        dunce::canonicalize(&candidate).map_err(|err| map_io_error(&candidate, err))?;
    if !canonical_pack.starts_with(&canonical_builtin) {
        return Err(sandbox_error(
            "courseId",
            "built-in course directory escapes built-in root",
        ));
    }

    let course_id_owned = course_id.to_owned();
    let source_key = format!("builtin:{course_id}");
    load_pack_dir_internal(&canonical_pack, false, source_key, move |entry_file| {
        CourseSourceRef::Builtin {
            course_id: course_id_owned,
            entry_file,
        }
    })
}

pub fn load_local_markdown(file_path: &Path) -> Result<LoadedCoursePack, AppError> {
    let canonical = dunce::canonicalize(file_path).map_err(|err| map_io_error(file_path, err))?;
    let source_text = read_markdown_file_utf8(&canonical)?;
    let canonical_str = canonical.to_string_lossy().into_owned();
    let file_name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("README.md");

    let (pack_fingerprint, starter_fingerprints) = compute_pack_fingerprint(vec![(
        file_name.to_owned(),
        FingerprintSource::Bytes(source_text.as_bytes()),
    )])?;

    let options = ParseMarkdownOptions {
        course_id: None,
        default_title: derive_fallback_title(&canonical),
        language_hint: None,
    };
    let course = parse_markdown(&source_text, &options)?;
    let pack_root = canonical
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();

    Ok(LoadedCoursePack {
        source: CourseSourceRef::LocalMarkdown {
            file_path: canonical_str.clone(),
        },
        source_key: format!("local-md:{canonical_str}"),
        pack_fingerprint,
        requires_trust: true,
        pack_root,
        entry_file_abs: canonical,
        starter_dir_abs: None,
        starter_fingerprints,
        manifest: None,
        checks: None,
        course,
        validation_issues: Vec::new(),
    })
}

pub fn load_local_pack_dir(dir_path: &Path) -> Result<LoadedCoursePack, AppError> {
    let canonical = canonicalize_pack_dir(dir_path, "dirPath")?;
    let canonical_str = canonical.to_string_lossy().into_owned();
    let source_key = format!("local-dir:{canonical_str}");
    load_pack_dir_internal(&canonical, true, source_key, move |entry_file| {
        CourseSourceRef::LocalPackDir {
            dir_path: canonical_str,
            entry_file,
        }
    })
}

pub fn load_git_pack_dir(
    cache_dir: &Path,
    url: &str,
    git_ref: Option<String>,
    source_key: String,
) -> Result<LoadedCoursePack, AppError> {
    let canonical = canonicalize_pack_dir(cache_dir, "gitRepo")?;
    let url_owned = url.to_owned();
    load_pack_dir_internal(&canonical, true, source_key, move |entry_file| {
        CourseSourceRef::GitRepo {
            url: url_owned,
            git_ref,
            entry_file,
        }
    })
}

pub(crate) fn reload_loaded_pack(loaded: &LoadedCoursePack) -> Result<LoadedCoursePack, AppError> {
    match &loaded.source {
        CourseSourceRef::LocalMarkdown { .. } => load_local_markdown(&loaded.entry_file_abs),
        CourseSourceRef::LocalPackDir { .. } => load_local_pack_dir(&loaded.pack_root),
        CourseSourceRef::GitRepo { url, git_ref, .. } => load_git_pack_dir(
            &loaded.pack_root,
            url,
            git_ref.clone(),
            loaded.source_key.clone(),
        ),
        CourseSourceRef::Builtin { course_id, .. } => {
            if let Some(parent) = loaded.pack_root.parent() {
                load_builtin_pack(parent, course_id)
            } else {
                load_local_pack_dir(&loaded.pack_root)
            }
        }
    }
}

pub fn build_inspect_result(pack: &LoadedCoursePack, trusted: bool) -> CourseInspectResult {
    let mut chapters_preview = Vec::with_capacity(pack.course.chapters.len());
    let mut command_summaries = Vec::new();

    for chapter in &pack.course.chapters {
        let chapter_checks = checks_for_chapter(pack.checks.as_ref(), &chapter.chapter_id);
        let auto_check_count = chapter_checks
            .iter()
            .filter(|spec| !matches!(spec, CheckSpec::SelfCheck { .. }))
            .count() as u32;
        let spec_self_checks = chapter_checks
            .iter()
            .filter(|spec| matches!(spec, CheckSpec::SelfCheck { .. }))
            .count() as u32;

        chapters_preview.push(CourseChapterPreview {
            chapter_id: chapter.chapter_id.clone(),
            index: chapter.index,
            kind: chapter.kind,
            title: chapter.title.clone(),
            command_count: chapter.command_blocks.len() as u32,
            auto_check_count,
            self_check_count: chapter.self_check_items.len() as u32 + spec_self_checks,
            default_cwd: chapter.default_cwd.clone(),
        });

        for cmd in &chapter.command_blocks {
            command_summaries.push(CommandSummary {
                id: cmd.id.clone(),
                chapter_id: cmd.chapter_id.clone(),
                command: cmd.command.clone(),
                cwd: cmd.cwd.clone(),
                role: cmd.role.clone(),
                is_secondary: cmd.is_secondary,
            });
        }
    }

    CourseInspectResult {
        source: pack.source.clone(),
        source_key: pack.source_key.clone(),
        pack_fingerprint: pack.pack_fingerprint.clone(),
        requires_trust: pack.requires_trust,
        trusted,
        manifest: pack.manifest.clone(),
        checks: pack.checks.clone(),
        course: pack.course.clone(),
        starter_files: pack.starter_fingerprints.keys().cloned().collect(),
        chapters_preview,
        command_summaries,
        validation_issues: pack.validation_issues.clone(),
    }
}

pub(crate) fn count_effective_chapters(course: &ParsedCourse) -> u32 {
    let exercise_count = course
        .chapters
        .iter()
        .filter(|ch| ch.kind == ChapterKind::Exercise)
        .count();
    if exercise_count > 0 {
        exercise_count as u32
    } else {
        course.chapters.len() as u32
    }
}

pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn load_pack_dir_internal<F>(
    canonical_root: &Path,
    requires_trust: bool,
    source_key: String,
    make_source: F,
) -> Result<LoadedCoursePack, AppError>
where
    F: FnOnce(RelWorkspacePath) -> CourseSourceRef,
{
    let (manifest, manifest_bytes) = match load_optional_manifest(canonical_root)? {
        Some((m, bytes)) => (Some(m), Some(bytes)),
        None => (None, None),
    };

    let entry_file = resolve_entry_file_rel(canonical_root, manifest.as_ref())?;
    let entry_file_abs = resolve_contained_path(canonical_root, &entry_file)?;
    let entry_text = read_markdown_file_utf8(&entry_file_abs)?;

    let parse_options = ParseMarkdownOptions {
        course_id: manifest.as_ref().map(|m| m.course_id.clone()),
        default_title: manifest
            .as_ref()
            .map(|m| m.title.clone())
            .or_else(|| derive_fallback_title(&entry_file_abs)),
        language_hint: manifest.as_ref().map(|m| m.language),
    };
    let mut course = parse_markdown(&entry_text, &parse_options)?;
    let mut validation_issues = Vec::new();

    if let Some(m) = manifest.as_ref() {
        apply_manifest_overlay(&mut course, m, &mut validation_issues);
    }

    let (checks, checks_for_hash) = match load_checks_spec(canonical_root, manifest.as_ref())? {
        Some((rel, spec, bytes)) => {
            validate_checks_against_course(&course, &spec, rel.as_str(), &mut validation_issues);
            (Some(spec), Some((rel, bytes)))
        }
        None => (None, None),
    };

    let (starter_dir_abs, starter_files) = resolve_starter_dir(canonical_root, manifest.as_ref())?;

    let mut hash_entries: Vec<(String, FingerprintSource<'_>)> = Vec::new();
    if let Some(bytes) = manifest_bytes.as_deref() {
        hash_entries.push(("codelab.json".to_owned(), FingerprintSource::Bytes(bytes)));
    }
    hash_entries.push((
        entry_file.as_str().to_owned(),
        FingerprintSource::Bytes(entry_text.as_bytes()),
    ));
    if let Some((rel, bytes)) = checks_for_hash.as_ref() {
        hash_entries.push((rel.as_str().to_owned(), FingerprintSource::Bytes(bytes)));
    }
    if let Some(starter_root) = starter_dir_abs.as_deref() {
        for rel in &starter_files {
            hash_entries.push((
                format!("starter/{}", rel.as_str()),
                FingerprintSource::File {
                    path: join_rel_path(starter_root, rel),
                    starter_path: rel.clone(),
                },
            ));
        }
    }

    let (pack_fingerprint, starter_fingerprints) = compute_pack_fingerprint(hash_entries)?;
    let source = make_source(entry_file);

    Ok(LoadedCoursePack {
        source,
        source_key,
        pack_fingerprint,
        requires_trust,
        pack_root: canonical_root.to_path_buf(),
        entry_file_abs,
        starter_dir_abs,
        starter_fingerprints,
        manifest,
        checks,
        course,
        validation_issues,
    })
}

fn is_safe_single_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.contains('/')
        && !segment.contains('\\')
        && !segment.contains('\0')
        && !segment.chars().any(char::is_control)
        && segment != "."
        && segment != ".."
        && RelWorkspacePath::new(segment).is_ok()
}

pub(super) fn load_optional_manifest(
    canonical_root: &Path,
) -> Result<Option<(CourseManifest, Vec<u8>)>, AppError> {
    let manifest_path = canonical_root.join("codelab.json");
    match fs::symlink_metadata(&manifest_path) {
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(map_io_error(&manifest_path, err)),
    }

    let manifest_rel = RelWorkspacePath::new("codelab.json").expect("static path valid");
    let resolved = resolve_contained_path(canonical_root, &manifest_rel)?;
    let bytes = read_bounded_bytes(&resolved, "codelab.json", MAX_PACK_METADATA_BYTES)?;
    let manifest: CourseManifest = serde_json::from_slice(&bytes)
        .map_err(|err| schema_error("codelab.json", err.to_string()))?;

    if manifest.course_id.trim().is_empty() {
        return Err(schema_error("codelab.json", "courseId must not be empty"));
    }
    if manifest.title.trim().is_empty() {
        return Err(schema_error("codelab.json", "title must not be empty"));
    }

    Ok(Some((manifest, bytes)))
}

fn resolve_entry_file_rel(
    canonical_root: &Path,
    manifest: Option<&CourseManifest>,
) -> Result<RelWorkspacePath, AppError> {
    if let Some(m) = manifest {
        return Ok(m.entry_file.clone());
    }

    for candidate in ["README.md", "readme.md"] {
        match fs::symlink_metadata(canonical_root.join(candidate)) {
            Ok(_) => return Ok(RelWorkspacePath::new(candidate).expect("static path valid")),
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => return Err(map_io_error(&canonical_root.join(candidate), err)),
        }
    }

    Err(schema_error(
        "README.md",
        "course pack directory does not contain README.md",
    ))
}

fn load_checks_spec(
    canonical_root: &Path,
    manifest: Option<&CourseManifest>,
) -> Result<Option<(RelWorkspacePath, ChecksSpec, Vec<u8>)>, AppError> {
    let checks_rel = if let Some(rel) = manifest.and_then(|m| m.checks_file.clone()) {
        rel
    } else {
        let default_path = canonical_root.join(".codelab-checks.json");
        match fs::symlink_metadata(&default_path) {
            Ok(_) => RelWorkspacePath::new(".codelab-checks.json").expect("static path valid"),
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(map_io_error(&default_path, err)),
        }
    };

    let resolved = resolve_contained_path(canonical_root, &checks_rel)?;
    let bytes = read_bounded_bytes(&resolved, checks_rel.as_str(), MAX_PACK_METADATA_BYTES)?;
    let spec: ChecksSpec = serde_json::from_slice(&bytes)
        .map_err(|err| schema_error(checks_rel.as_str(), err.to_string()))?;

    Ok(Some((checks_rel, spec, bytes)))
}

fn read_bounded_bytes(
    path: &Path,
    display_name: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, AppError> {
    let file = File::open(path).map_err(|err| map_io_error(path, err))?;
    let metadata = file.metadata().map_err(|err| map_io_error(path, err))?;

    if !metadata.is_file() {
        return Err(schema_error(display_name, "expected a regular file"));
    }
    if metadata.len() > max_bytes as u64 {
        return Err(AppError::new(
            ErrorCode::FileTooLargeOrBinary,
            "error.fileTooLargeOrBinary",
        )
        .with_param("path", display_name)
        .with_detail(format!(
            "file '{display_name}' ({} bytes) exceeds limit of {max_bytes} bytes",
            metadata.len()
        )));
    }

    let capacity = usize::try_from(metadata.len()).unwrap_or(0).min(max_bytes);
    let mut bytes = Vec::with_capacity(capacity);
    file.take((max_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|err| map_io_error(path, err))?;

    if bytes.len() > max_bytes {
        return Err(AppError::new(
            ErrorCode::FileTooLargeOrBinary,
            "error.fileTooLargeOrBinary",
        )
        .with_param("path", display_name)
        .with_detail(format!(
            "file '{display_name}' exceeds limit of {max_bytes} bytes"
        )));
    }

    Ok(bytes)
}

fn compute_pack_fingerprint(
    mut items: Vec<(String, FingerprintSource<'_>)>,
) -> Result<(String, BTreeMap<RelWorkspacePath, [u8; 32]>), AppError> {
    items.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    let mut starter_fingerprints = BTreeMap::new();

    for (key, source) in items {
        hasher.update((key.len() as u64).to_be_bytes());
        hasher.update(key.as_bytes());

        match source {
            FingerprintSource::Bytes(slice) => {
                hasher.update((slice.len() as u64).to_be_bytes());
                hasher.update(slice);
            }
            FingerprintSource::File { path, starter_path } => {
                let mut file = File::open(&path).map_err(|err| map_io_error(&path, err))?;
                let expected_len = file
                    .metadata()
                    .map_err(|err| map_io_error(&path, err))?
                    .len();
                hasher.update(expected_len.to_be_bytes());
                let mut starter_hasher = Sha256::new();

                let mut total_read = 0u64;
                loop {
                    let n = file
                        .read(&mut buf)
                        .map_err(|err| map_io_error(&path, err))?;
                    if n == 0 {
                        break;
                    }
                    total_read = total_read.saturating_add(n as u64);
                    hasher.update(&buf[..n]);
                    starter_hasher.update(&buf[..n]);
                }

                if total_read != expected_len {
                    return Err(AppError::new(
                        ErrorCode::ExternalEditConflict,
                        "error.unknown",
                    )
                    .with_param("detail", format!("file changed while hashing: {key}"))
                    .with_detail(format!(
                        "starter file '{key}' changed length during hashing ({expected_len} -> {total_read})"
                    ))
                    .with_retryable(true));
                }
                starter_fingerprints.insert(starter_path, starter_hasher.finalize().into());
            }
        }
    }

    Ok((
        format!("sha256:{}", bytes_to_hex(&hasher.finalize())),
        starter_fingerprints,
    ))
}

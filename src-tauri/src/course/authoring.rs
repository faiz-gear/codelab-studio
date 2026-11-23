use std::fs;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use crate::course::pack_fs::{
    canonicalize_pack_dir, ensure_contained_parent, join_rel_path, schema_error,
    verify_no_symlink_along_dest, write_two_files_atomically,
};
use crate::course::pack_loader::load_optional_manifest;
use crate::course::pack_overlay::{apply_manifest_overlay, validate_checks_against_course};
use crate::course::{
    derive_fallback_title, markdown_limit_error, parse_markdown, ParseMarkdownOptions,
    MAX_MARKDOWN_BYTES, MAX_PACK_METADATA_BYTES,
};
use crate::domain::course::{AuthorPreviewResult, ChecksSpec, PackValidationIssue, ParsedCourse};
use crate::domain::error::AppError;
use crate::domain::ids::RelWorkspacePath;

static AUTHOR_SAVES: Mutex<()> = Mutex::new(());

pub fn preview_author_draft(
    markdown: &str,
    checks_json: Option<&str>,
) -> Result<AuthorPreviewResult, AppError> {
    let course = parse_markdown(markdown, &ParseMarkdownOptions::default())?;
    let mut validation_issues = Vec::new();

    let checks = match checks_json.map(str::trim).filter(|raw| !raw.is_empty()) {
        None => None,
        Some(raw) if raw.len() > MAX_PACK_METADATA_BYTES => {
            validation_issues.push(PackValidationIssue {
                file: ".codelab-checks.json".to_owned(),
                message: format!(
                    "checks file size {} bytes exceeds limit of {MAX_PACK_METADATA_BYTES} bytes",
                    raw.len()
                ),
            });
            None
        }
        Some(raw) => match serde_json::from_str::<ChecksSpec>(raw) {
            Ok(spec) => {
                validate_checks_against_course(
                    &course,
                    &spec,
                    ".codelab-checks.json",
                    &mut validation_issues,
                );
                Some(spec)
            }
            Err(err) => {
                validation_issues.push(PackValidationIssue {
                    file: ".codelab-checks.json".to_owned(),
                    message: err.to_string(),
                });
                None
            }
        },
    };

    Ok(AuthorPreviewResult {
        course,
        checks,
        validation_issues,
    })
}

pub fn save_author_pack(
    pack_dir: &Path,
    markdown: &str,
    checks_json: &str,
) -> Result<ParsedCourse, AppError> {
    if markdown.len() > MAX_MARKDOWN_BYTES {
        return Err(markdown_limit_error(markdown.len() as u64));
    }
    if checks_json.len() > MAX_PACK_METADATA_BYTES {
        return Err(schema_error(
            ".codelab-checks.json",
            format!(
                "checks file size {} bytes exceeds limit of {MAX_PACK_METADATA_BYTES} bytes",
                checks_json.len()
            ),
        ));
    }

    let canonical_dir = canonicalize_pack_dir(pack_dir, "packDir")?;
    let _save_guard = AUTHOR_SAVES.lock().unwrap_or_else(PoisonError::into_inner);
    let manifest = load_optional_manifest(&canonical_dir)?.map(|(manifest, _)| manifest);
    let entry_rel = manifest
        .as_ref()
        .map(|manifest| manifest.entry_file.clone())
        .unwrap_or_else(|| RelWorkspacePath::new("README.md").expect("static path valid"));
    let checks_rel = manifest
        .as_ref()
        .and_then(|manifest| manifest.checks_file.clone())
        .unwrap_or_else(|| {
            RelWorkspacePath::new(".codelab-checks.json").expect("static path valid")
        });

    verify_no_symlink_along_dest(&canonical_dir, &entry_rel)?;
    verify_no_symlink_along_dest(&canonical_dir, &checks_rel)?;

    let entry_dest = join_rel_path(&canonical_dir, &entry_rel);
    let checks_dest = join_rel_path(&canonical_dir, &checks_rel);
    let manifest_dest = canonical_dir.join("codelab.json");
    if destinations_match(&entry_dest, &checks_dest)
        || destinations_match(&entry_dest, &manifest_dest)
        || destinations_match(&checks_dest, &manifest_dest)
    {
        return Err(schema_error(
            "codelab.json",
            "entryFile, checksFile and the manifest must use different paths",
        ));
    }

    let parse_options = ParseMarkdownOptions {
        course_id: manifest.as_ref().map(|manifest| manifest.course_id.clone()),
        default_title: manifest
            .as_ref()
            .map(|manifest| manifest.title.clone())
            .or_else(|| derive_fallback_title(&entry_dest)),
        language_hint: manifest.as_ref().map(|manifest| manifest.language),
    };
    let mut course = parse_markdown(markdown, &parse_options)?;
    let mut validation_issues = Vec::new();

    if let Some(manifest) = manifest.as_ref() {
        apply_manifest_overlay(&mut course, manifest, &mut validation_issues);
    }

    let checks_spec: ChecksSpec = serde_json::from_str(checks_json)
        .map_err(|err| schema_error(checks_rel.as_str(), err.to_string()))?;
    validate_checks_against_course(
        &course,
        &checks_spec,
        checks_rel.as_str(),
        &mut validation_issues,
    );

    if let Some(first_issue) = validation_issues.first() {
        return Err(schema_error(
            first_issue.file.as_str(),
            first_issue.message.as_str(),
        ));
    }

    ensure_contained_parent(&canonical_dir, &entry_dest, entry_rel.as_str())?;
    ensure_contained_parent(&canonical_dir, &checks_dest, checks_rel.as_str())?;
    write_two_files_atomically(
        &entry_dest,
        markdown.as_bytes(),
        &checks_dest,
        checks_json.as_bytes(),
    )?;

    Ok(course)
}

fn destinations_match(first: &Path, second: &Path) -> bool {
    if first == second {
        return true;
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    if first
        .to_string_lossy()
        .eq_ignore_ascii_case(&second.to_string_lossy())
    {
        return true;
    }
    match (fs::canonicalize(first), fs::canonicalize(second)) {
        (Ok(first), Ok(second)) => first == second,
        _ => false,
    }
}

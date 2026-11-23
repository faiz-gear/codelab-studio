use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::course::map_io_error;
use crate::course::pack_fs::{
    canonicalize_pack_dir, ensure_contained_parent, join_rel_path, next_sibling_temp_path,
    resolve_contained_path, sandbox_error, schema_error, verify_no_symlink_along_dest,
};
use crate::domain::course::CourseManifest;
use crate::domain::error::{AppError, ErrorCode};
use crate::domain::ids::RelWorkspacePath;
use crate::storage::WorkspaceStore;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HydrationReport {
    pub newly_hydrated: Vec<RelWorkspacePath>,
    pub skipped_existing: Vec<RelWorkspacePath>,
}

pub fn hydrate_starter_into_workspace(
    starter_dir: &Path,
    workspace_root: &Path,
    store: &WorkspaceStore,
    expected_fingerprints: &BTreeMap<RelWorkspacePath, [u8; 32]>,
) -> Result<HydrationReport, AppError> {
    let canonical_starter = canonicalize_pack_dir(starter_dir, "starterDir")?;
    let canonical_workspace = canonicalize_pack_dir(workspace_root, "workspaceRoot")?;

    let starter_files = enumerate_starter_files(&canonical_starter)?;
    let actual_files: BTreeSet<_> = starter_files.iter().cloned().collect();
    let expected_files: BTreeSet<_> = expected_fingerprints.keys().cloned().collect();
    if actual_files != expected_files {
        return Err(starter_fingerprint_error(
            "starter file list changed after inspection",
        ));
    }

    let meta = store.read_meta().map_err(AppError::from)?;
    let mut hydrated_set: BTreeSet<RelWorkspacePath> = meta.hydrated_files.into_iter().collect();

    let mut report = HydrationReport::default();
    let mut meta_changed = false;

    for rel in starter_files {
        if hydrated_set.contains(&rel) {
            continue;
        }

        verify_no_symlink_along_dest(&canonical_workspace, &rel)?;
        let dest = join_rel_path(&canonical_workspace, &rel);

        match fs::symlink_metadata(&dest) {
            Ok(existing_meta) => {
                if existing_meta.file_type().is_symlink() {
                    return Err(sandbox_error(
                        rel.as_str(),
                        format!(
                            "symlink rejected at workspace destination '{}'",
                            rel.as_str()
                        ),
                    ));
                }
                report.skipped_existing.push(rel.clone());
                hydrated_set.insert(rel);
                meta_changed = true;
                continue;
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(map_io_error(&dest, err)),
        }

        let src = resolve_contained_path(&canonical_starter, &rel)?;
        ensure_contained_parent(&canonical_workspace, &dest, rel.as_str())?;
        let copied = copy_file_if_missing(
            &src,
            &dest,
            expected_fingerprints
                .get(&rel)
                .expect("starter file set was checked above"),
        )?;

        if copied {
            report.newly_hydrated.push(rel.clone());
        } else {
            verify_no_symlink_along_dest(&canonical_workspace, &rel)?;
            report.skipped_existing.push(rel.clone());
        }
        hydrated_set.insert(rel);
        meta_changed = true;
    }

    if meta_changed {
        let updated: Vec<RelWorkspacePath> = hydrated_set.into_iter().collect();
        store
            .update_meta(|meta| {
                meta.hydrated_files = updated;
                Ok(())
            })
            .map_err(AppError::from)?;
    }

    Ok(report)
}

pub(crate) fn resolve_starter_dir(
    canonical_root: &Path,
    manifest: Option<&CourseManifest>,
) -> Result<(Option<PathBuf>, Vec<RelWorkspacePath>), AppError> {
    let starter_abs = if let Some(rel) = manifest.and_then(|m| m.starter_dir.as_ref()) {
        let abs = resolve_contained_path(canonical_root, rel)?;
        if !abs.is_dir() {
            return Err(schema_error(
                "codelab.json",
                format!("starterDir '{}' is not a directory", rel.as_str()),
            ));
        }
        Some(abs)
    } else {
        let default_starter = canonical_root.join("starter");
        match fs::symlink_metadata(&default_starter) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(sandbox_error(
                    "starter",
                    "symlink rejected for starter directory",
                ));
            }
            Ok(meta) if meta.is_dir() => {
                let rel = RelWorkspacePath::new("starter").expect("static path valid");
                Some(resolve_contained_path(canonical_root, &rel)?)
            }
            Ok(_) => None,
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) => return Err(map_io_error(&default_starter, err)),
        }
    };

    let files = match &starter_abs {
        Some(dir) => enumerate_starter_files(dir)?,
        None => Vec::new(),
    };

    Ok((starter_abs, files))
}

fn enumerate_starter_files(canonical_starter: &Path) -> Result<Vec<RelWorkspacePath>, AppError> {
    let mut files = Vec::new();
    walk_starter_dir(canonical_starter, canonical_starter, "", &mut files)?;
    files.sort();
    Ok(files)
}

fn walk_starter_dir(
    canonical_root: &Path,
    current_dir: &Path,
    prefix: &str,
    out: &mut Vec<RelWorkspacePath>,
) -> Result<(), AppError> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(current_dir).map_err(|err| map_io_error(current_dir, err))? {
        entries.push(entry.map_err(|err| map_io_error(current_dir, err))?);
    }
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|err| map_io_error(&path, err))?;
        if meta.file_type().is_symlink() {
            return Err(sandbox_error(
                "starter",
                format!("symlink rejected in starter directory: {}", path.display()),
            ));
        }

        let name = entry.file_name().into_string().map_err(|_| {
            sandbox_error(
                "starter",
                "non-UTF-8 file name rejected in starter directory",
            )
        })?;

        if name.starts_with(".codelab") {
            return Err(sandbox_error(
                "starter",
                format!("reserved .codelab entry rejected in starter directory: {name}"),
            ));
        }

        let rel_str = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let rel = RelWorkspacePath::new(rel_str)
            .map_err(|err| sandbox_error("starter", err.to_string()))?;

        if meta.is_dir() {
            walk_starter_dir(canonical_root, &path, rel.as_str(), out)?;
        } else if meta.is_file() {
            let canonical_file =
                dunce::canonicalize(&path).map_err(|err| map_io_error(&path, err))?;
            if !canonical_file.starts_with(canonical_root) {
                return Err(sandbox_error(
                    rel.as_str(),
                    "starter file escapes starter directory",
                ));
            }
            out.push(rel);
        } else {
            return Err(sandbox_error(
                rel.as_str(),
                "unsupported special file in starter directory",
            ));
        }
    }

    Ok(())
}

fn copy_file_if_missing(
    src: &Path,
    dest: &Path,
    expected_fingerprint: &[u8; 32],
) -> Result<bool, AppError> {
    let mut src_file = File::open(src).map_err(|err| map_io_error(src, err))?;
    let tmp_path = next_sibling_temp_path(dest, "hydrate");
    let mut tmp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)
        .map_err(|err| map_io_error(&tmp_path, err))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = match src_file.read(&mut buffer) {
            Ok(count) => count,
            Err(err) => {
                drop(tmp_file);
                let _ = fs::remove_file(&tmp_path);
                return Err(map_io_error(src, err));
            }
        };
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        if let Err(err) = tmp_file.write_all(&buffer[..count]) {
            drop(tmp_file);
            let _ = fs::remove_file(&tmp_path);
            return Err(map_io_error(&tmp_path, err));
        }
    }

    let actual_fingerprint: [u8; 32] = hasher.finalize().into();
    if &actual_fingerprint != expected_fingerprint {
        drop(tmp_file);
        let _ = fs::remove_file(&tmp_path);
        return Err(starter_fingerprint_error(
            "starter file content changed after inspection",
        ));
    }

    if let Err(err) = tmp_file.sync_all() {
        drop(tmp_file);
        let _ = fs::remove_file(&tmp_path);
        return Err(map_io_error(&tmp_path, err));
    }
    drop(tmp_file);

    let published = fs::hard_link(&tmp_path, dest);
    let _ = fs::remove_file(&tmp_path);
    match published {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(err) => Err(map_io_error(dest, err)),
    }
}

fn starter_fingerprint_error(detail: &str) -> AppError {
    AppError::new(
        ErrorCode::ExternalPackUntrusted,
        "error.externalPackUntrusted",
    )
    .with_param("source", "starter")
    .with_detail(detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_publish_preserves_a_destination_created_before_commit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("starter.txt");
        let dest = dir.path().join("learner.txt");
        fs::write(&source, "starter").unwrap();
        fs::write(&dest, "learner").unwrap();
        let fingerprint = Sha256::digest(b"starter").into();

        assert!(!copy_file_if_missing(&source, &dest, &fingerprint).unwrap());
        assert_eq!(fs::read_to_string(dest).unwrap(), "learner");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }
}

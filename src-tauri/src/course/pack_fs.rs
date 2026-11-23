use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::course::map_io_error;
use crate::domain::error::{AppError, ErrorCode};
use crate::domain::ids::RelWorkspacePath;
use crate::storage::atomic_json::atomic_replace;

static NEXT_TMP_SEQ: AtomicU64 = AtomicU64::new(1);

pub(crate) fn canonicalize_pack_dir(dir_path: &Path, field: &str) -> Result<PathBuf, AppError> {
    let canonical = dunce::canonicalize(dir_path).map_err(|err| {
        if err.kind() == io::ErrorKind::NotFound {
            invalid_input_error(
                field,
                format!("directory '{}' does not exist", dir_path.display()),
            )
        } else {
            map_io_error(dir_path, err)
        }
    })?;

    if !canonical.is_dir() {
        return Err(invalid_input_error(
            field,
            format!("'{}' is not a directory", canonical.display()),
        ));
    }

    Ok(canonical)
}

pub(crate) fn join_rel_path(root: &Path, rel: &RelWorkspacePath) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in rel.as_str().split('/') {
        path.push(segment);
    }
    path
}

pub(crate) fn resolve_contained_path(
    canonical_root: &Path,
    rel: &RelWorkspacePath,
) -> Result<PathBuf, AppError> {
    let mut current = canonical_root.to_path_buf();
    for segment in rel.as_str().split('/') {
        current.push(segment);
        let meta = fs::symlink_metadata(&current).map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                schema_error(
                    rel.as_str(),
                    format!("required pack path '{}' does not exist", rel.as_str()),
                )
            } else {
                map_io_error(&current, err)
            }
        })?;

        if meta.file_type().is_symlink() {
            return Err(sandbox_error(
                rel.as_str(),
                format!("symlink rejected inside course pack: {}", rel.as_str()),
            ));
        }
    }

    let canonical = dunce::canonicalize(&current).map_err(|err| map_io_error(&current, err))?;
    if !canonical.starts_with(canonical_root) {
        return Err(sandbox_error(
            rel.as_str(),
            format!("path '{}' escapes course pack root", rel.as_str()),
        ));
    }

    Ok(canonical)
}

pub(crate) fn verify_no_symlink_along_dest(
    canonical_root: &Path,
    rel: &RelWorkspacePath,
) -> Result<(), AppError> {
    let mut current = canonical_root.to_path_buf();
    for segment in rel.as_str().split('/') {
        current.push(segment);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(sandbox_error(
                    rel.as_str(),
                    format!("symlink rejected along target path '{}'", rel.as_str()),
                ));
            }
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => break,
            Err(err) => return Err(map_io_error(&current, err)),
        }
    }
    Ok(())
}

pub(crate) fn ensure_contained_parent(
    canonical_root: &Path,
    dest: &Path,
    rel_label: &str,
) -> Result<(), AppError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|err| map_io_error(parent, err))?;
        let canonical_parent =
            dunce::canonicalize(parent).map_err(|err| map_io_error(parent, err))?;
        if !canonical_parent.starts_with(canonical_root) {
            return Err(sandbox_error(
                rel_label,
                "target parent directory escapes pack root",
            ));
        }
    }
    Ok(())
}

pub(crate) fn write_two_files_atomically(
    first_dest: &Path,
    first_bytes: &[u8],
    second_dest: &Path,
    second_bytes: &[u8],
) -> Result<(), AppError> {
    write_two_files_atomically_with(
        first_dest,
        first_bytes,
        second_dest,
        second_bytes,
        atomic_replace,
    )
}

fn write_two_files_atomically_with<F>(
    first_dest: &Path,
    first_bytes: &[u8],
    second_dest: &Path,
    second_bytes: &[u8],
    mut replace: F,
) -> Result<(), AppError>
where
    F: FnMut(&Path, &Path) -> io::Result<()>,
{
    if first_dest == second_dest {
        return Err(schema_error(
            "codelab.json",
            "output paths must be different",
        ));
    }

    let first_temp = stage_bytes(first_dest, first_bytes, "author")?;
    let second_temp = match stage_bytes(second_dest, second_bytes, "author") {
        Ok(path) => path,
        Err(err) => {
            let _ = fs::remove_file(first_temp);
            return Err(err);
        }
    };
    let first_backup = match backup_existing_file(first_dest) {
        Ok(backup) => backup,
        Err(err) => {
            remove_paths([&first_temp, &second_temp]);
            return Err(err);
        }
    };
    let second_backup = match backup_existing_file(second_dest) {
        Ok(backup) => backup,
        Err(err) => {
            remove_paths([&first_temp, &second_temp]);
            if let Some(path) = &first_backup {
                let _ = fs::remove_file(path);
            }
            return Err(err);
        }
    };

    let result = commit_staged_pair(
        (&first_temp, first_dest, first_backup.as_deref()),
        (&second_temp, second_dest, second_backup.as_deref()),
        &mut replace,
    );
    remove_paths([&first_temp, &second_temp]);

    if result.is_ok() {
        if let Some(path) = first_backup {
            let _ = fs::remove_file(path);
        }
        if let Some(path) = second_backup {
            let _ = fs::remove_file(path);
        }
    }

    result
}

fn commit_staged_pair<F>(
    first: (&Path, &Path, Option<&Path>),
    second: (&Path, &Path, Option<&Path>),
    replace: &mut F,
) -> Result<(), AppError>
where
    F: FnMut(&Path, &Path) -> io::Result<()>,
{
    let (failed_destination, commit_error) = match replace(first.0, first.1) {
        Ok(()) => match replace(second.0, second.1) {
            Ok(()) => return Ok(()),
            Err(err) => (second.1, err),
        },
        Err(err) => (first.1, err),
    };

    let first_rollback = restore_destination(first.1, first.2);
    let second_rollback = restore_destination(second.1, second.2);
    let detail = match (first_rollback, second_rollback) {
        (Ok(()), Ok(())) => format!("two-file save failed and both original files were restored: {commit_error}"),
        (first, second) => format!(
            "two-file save failed: {commit_error}; rollback results: first={first:?}, second={second:?}"
        ),
    };

    Err(map_io_error(failed_destination, commit_error).with_detail(detail))
}

fn restore_destination(dest: &Path, backup: Option<&Path>) -> Result<(), String> {
    if let Some(backup) = backup {
        atomic_replace(backup, dest).map_err(|err| err.to_string())
    } else {
        match fs::remove_file(dest) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}

fn backup_existing_file(path: &Path) -> Result<Option<PathBuf>, AppError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(map_io_error(path, err)),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(schema_error(
            path.display().to_string(),
            "author output destination must be a regular file",
        ));
    }

    let backup_path = next_sibling_temp_path(path, "author-backup");
    let mut source = File::open(path).map_err(|err| map_io_error(path, err))?;
    let mut backup = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup_path)
        .map_err(|err| map_io_error(&backup_path, err))?;
    if let Err(err) = io::copy(&mut source, &mut backup).and_then(|_| backup.sync_all()) {
        drop(backup);
        let _ = fs::remove_file(&backup_path);
        return Err(map_io_error(&backup_path, err));
    }
    drop(backup);
    if let Err(err) = fs::set_permissions(&backup_path, metadata.permissions()) {
        let _ = fs::remove_file(&backup_path);
        return Err(map_io_error(&backup_path, err));
    }
    Ok(Some(backup_path))
}

fn stage_bytes(dest: &Path, bytes: &[u8], kind: &str) -> Result<PathBuf, AppError> {
    let temp_path = next_sibling_temp_path(dest, kind);
    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|err| map_io_error(&temp_path, err))?;
    if let Err(err) = temp_file
        .write_all(bytes)
        .and_then(|()| temp_file.sync_all())
    {
        drop(temp_file);
        let _ = fs::remove_file(&temp_path);
        return Err(map_io_error(&temp_path, err));
    }
    Ok(temp_path)
}

fn remove_paths<const N: usize>(paths: [&Path; N]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

pub(crate) fn next_sibling_temp_path(dest: &Path, kind: &str) -> PathBuf {
    let pid = std::process::id();
    let seq = NEXT_TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    dest.with_file_name(format!(".codelab-{kind}.tmp.{pid}.{seq}"))
}

pub(crate) fn schema_error(file: impl Into<String>, detail: impl Into<String>) -> AppError {
    let detail_str = detail.into();
    AppError::new(
        ErrorCode::SchemaValidationFailed,
        "error.schemaValidationFailed",
    )
    .with_param("file", file)
    .with_param("detail", detail_str.as_str())
    .with_detail(detail_str)
}

pub(crate) fn sandbox_error(field: impl Into<String>, detail: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::PathOutsideSandbox, "error.invalidInput")
        .with_param("field", field)
        .with_detail(detail)
}

pub(crate) fn invalid_input_error(field: impl Into<String>, detail: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::SchemaValidationFailed, "error.invalidInput")
        .with_param("field", field)
        .with_detail(detail)
}

#[cfg(test)]
mod tests {
    use super::{atomic_replace, write_two_files_atomically, write_two_files_atomically_with};
    use std::fs;
    use std::io;

    #[test]
    fn failed_second_replacement_restores_both_original_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = dir.path().join("README.md");
        let second = dir.path().join(".codelab-checks.json");
        fs::write(&first, "old markdown").expect("write old markdown");
        fs::write(&second, "old checks").expect("write old checks");
        let mut replace_count = 0;

        let error = write_two_files_atomically_with(
            &first,
            b"new markdown",
            &second,
            b"new checks",
            |source, destination| {
                replace_count += 1;
                if replace_count == 2 {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "injected second replacement failure",
                    ));
                }
                atomic_replace(source, destination)
            },
        )
        .expect_err("second replacement should fail");

        assert!(error
            .detail
            .unwrap()
            .contains("both original files were restored"));
        assert_eq!(fs::read_to_string(first).unwrap(), "old markdown");
        assert_eq!(fs::read_to_string(second).unwrap(), "old checks");
        assert!(fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().contains(".tmp.")));
    }

    #[test]
    fn successful_two_file_save_replaces_both_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = dir.path().join("README.md");
        let second = dir.path().join(".codelab-checks.json");
        fs::write(&first, "old markdown").expect("write old markdown");
        fs::write(&second, "old checks").expect("write old checks");

        write_two_files_atomically(&first, b"new markdown", &second, b"new checks")
            .expect("replace both files");

        assert_eq!(fs::read_to_string(first).unwrap(), "new markdown");
        assert_eq!(fs::read_to_string(second).unwrap(), "new checks");
    }
}

use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTimeError, UNIX_EPOCH};

use cap_fs_ext::DirExt;
use cap_std::fs::{Dir, Metadata};
use sha2::{Digest, Sha256};

use super::model::{MAX_WORKSPACE_TEXT_BYTES, WorkspaceFile, WorkspaceFileContents};
use super::paths::{WorkspacePaths, invalid_path, is_ignored, relative_path, wire_path};
use crate::filesystem::{open_canonical_dir, open_dir_at, open_regular};
use crate::{DomainError, ErrorCode};

impl WorkspacePaths {
    pub(crate) fn read(&self, raw: &str) -> Result<WorkspaceFile, DomainError> {
        self.read_io(raw)
            .map_err(|err| workspace_file_error(&err, raw, ErrorCode::WorkspaceFileReadFailed))
    }

    pub(super) fn read_io(&self, raw: &str) -> io::Result<WorkspaceFile> {
        let input = self.input(raw)?;
        let path = match self.canonical(&input) {
            Ok(path) => path,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // A missing file is a normal editor state. A dangling symlink or an
                // escaped parent is not a file that the editor should recreate.
                self.validate_missing(&input)?;
                return Ok(WorkspaceFile {
                    path: wire_path(&input)?,
                    size_bytes: 0,
                    revision: None,
                    contents: WorkspaceFileContents::Missing,
                });
            }
            Err(error) => return Err(error),
        };
        let wire = wire_path(&path)?;
        match read_bounded_file(&self.dir, &path)? {
            BoundedFile::TooLarge {
                size_bytes,
                modified,
            } => Ok(WorkspaceFile {
                path: wire,
                size_bytes,
                revision: Some(format!("large:{size_bytes}:{modified:?}")),
                contents: WorkspaceFileContents::TooLarge,
            }),
            BoundedFile::Bytes(bytes) => {
                let size_bytes = bytes.len() as u64;
                let revision = Some(file_revision(&wire, &bytes));
                let contents = match String::from_utf8(bytes) {
                    Ok(text) if is_text(&text) => WorkspaceFileContents::Text { text },
                    _ => WorkspaceFileContents::Binary,
                };
                Ok(WorkspaceFile {
                    path: wire,
                    size_bytes,
                    revision,
                    contents,
                })
            }
        }
    }

    pub(super) fn read_comparison_target(
        &self,
        exact_path: &str,
    ) -> Result<WorkspaceFile, DomainError> {
        let current = self.read(exact_path)?;
        if current.path != exact_path {
            return Err(DomainError::new(ErrorCode::WorkspacePathOutsideRoot)
                .with_detail("path", exact_path));
        }
        if !is_editable(&current.contents) {
            return Err(not_text(exact_path));
        }
        Ok(current)
    }
}

pub(super) struct SnapshotDir {
    dir: Dir,
    error_path: String,
}

impl SnapshotDir {
    pub(super) fn open_optional(
        course_root: &Path,
        relative: &Path,
        error_path: &str,
    ) -> Result<Option<Self>, DomainError> {
        let relative = relative_path(relative).map_err(|_| invalid_course_path(error_path))?;
        let mut dir =
            open_canonical_dir(course_root).map_err(|_| invalid_course_path(error_path))?;
        for name in &relative {
            match dir.symlink_metadata(name) {
                Ok(metadata) if metadata.is_dir() => {
                    dir = dir
                        .open_dir_nofollow(name)
                        .map_err(|_| invalid_course_path(error_path))?;
                }
                Ok(_) => return Err(invalid_course_path(error_path)),
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(_) => return Err(invalid_course_path(error_path)),
            }
        }
        Ok(Some(Self {
            dir,
            error_path: error_path.to_owned(),
        }))
    }

    pub(super) fn list_all_files(&self) -> Result<Vec<PathBuf>, DomainError> {
        self.list_files(false)
    }

    pub(super) fn list_visible_files(&self) -> Result<Vec<PathBuf>, DomainError> {
        self.list_files(true)
    }

    fn list_files(&self, skip_ignored: bool) -> Result<Vec<PathBuf>, DomainError> {
        let walk = || -> io::Result<Vec<PathBuf>> {
            let mut paths = Vec::new();
            let mut pending = vec![(self.dir.try_clone()?, PathBuf::new())];
            while let Some((dir, prefix)) = pending.pop() {
                for entry in dir.entries()? {
                    let entry = entry?;
                    let name = entry.file_name();
                    let path = prefix.join(&name);
                    if skip_ignored && is_ignored(&path) {
                        continue;
                    }
                    let file_type = entry.file_type()?;
                    if file_type.is_dir() {
                        pending.push((dir.open_dir_nofollow(&name)?, path));
                    } else if file_type.is_file() {
                        paths.push(path);
                    } else {
                        return Err(invalid_path());
                    }
                }
            }
            paths.sort();
            Ok(paths)
        };
        walk().map_err(|_| invalid_course_path(&self.error_path))
    }

    pub(super) fn read_text(&self, wire_name: &str) -> Result<String, DomainError> {
        let relative =
            relative_path(Path::new(wire_name)).map_err(|_| invalid_course_path(wire_name))?;
        match read_bounded_file(&self.dir, &relative) {
            Ok(BoundedFile::Bytes(bytes)) => decode_text(&bytes, wire_name),
            Ok(BoundedFile::TooLarge { .. }) => Err(not_text(wire_name)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Err(DomainError::new(ErrorCode::WorkspaceFileNotFound)
                    .with_detail("path", wire_name))
            }
            Err(_) => Err(invalid_course_path(wire_name)),
        }
    }
}

enum BoundedFile {
    Bytes(Vec<u8>),
    TooLarge {
        size_bytes: u64,
        modified: Option<Result<Duration, SystemTimeError>>,
    },
}

fn read_bounded_file(root: &Dir, path: &Path) -> io::Result<BoundedFile> {
    let parent = open_dir_at(root, path.parent().ok_or_else(invalid_path)?)?;
    let file = open_regular(
        &parent,
        Path::new(path.file_name().ok_or_else(invalid_path)?),
    )?;
    let metadata = file.metadata()?;
    let size_bytes = metadata.len();
    if size_bytes > MAX_WORKSPACE_TEXT_BYTES as u64 {
        return Ok(BoundedFile::TooLarge {
            size_bytes,
            modified: modified_since_epoch(&metadata),
        });
    }
    let mut bytes = Vec::new();
    file.take((MAX_WORKSPACE_TEXT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_WORKSPACE_TEXT_BYTES {
        return Ok(BoundedFile::TooLarge {
            size_bytes: bytes.len() as u64,
            modified: modified_since_epoch(&metadata),
        });
    }
    Ok(BoundedFile::Bytes(bytes))
}

fn modified_since_epoch(metadata: &Metadata) -> Option<Result<Duration, SystemTimeError>> {
    metadata
        .modified()
        .ok()
        .map(|time| time.into_std().duration_since(UNIX_EPOCH))
}

pub(super) fn decode_text(bytes: &[u8], path: &str) -> Result<String, DomainError> {
    if bytes.len() > MAX_WORKSPACE_TEXT_BYTES {
        return Err(not_text(path));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| not_text(path))?;
    validate_text(text, path)?;
    Ok(text.to_owned())
}

pub(super) fn validate_text(text: &str, path: &str) -> Result<(), DomainError> {
    if text.len() <= MAX_WORKSPACE_TEXT_BYTES && is_text(text) {
        return Ok(());
    }
    Err(not_text(path))
}

pub(super) fn is_editable(contents: &WorkspaceFileContents) -> bool {
    !matches!(
        contents,
        WorkspaceFileContents::Binary | WorkspaceFileContents::TooLarge
    )
}

pub(super) fn workspace_text_file(path: String, text: String) -> WorkspaceFile {
    WorkspaceFile {
        revision: Some(file_revision(&path, text.as_bytes())),
        size_bytes: text.len() as u64,
        path,
        contents: WorkspaceFileContents::Text { text },
    }
}

pub(super) fn workspace_file_error(err: &io::Error, path: &str, failure: ErrorCode) -> DomainError {
    let code = match err.kind() {
        io::ErrorKind::PermissionDenied | io::ErrorKind::InvalidInput => {
            ErrorCode::WorkspacePathOutsideRoot
        }
        io::ErrorKind::NotFound => ErrorCode::WorkspaceFileNotFound,
        io::ErrorKind::AlreadyExists => ErrorCode::WorkspaceFileExists,
        io::ErrorKind::InvalidData => ErrorCode::WorkspaceFileNotText,
        _ => failure,
    };
    DomainError::new(code)
        .with_detail("path", path)
        .with_detail("reason", err.to_string())
}

pub(super) fn invalid_course_path(path: &str) -> DomainError {
    DomainError::new(ErrorCode::CoursePathInvalid).with_detail("path", path)
}

fn not_text(path: &str) -> DomainError {
    DomainError::new(ErrorCode::WorkspaceFileNotText).with_detail("path", path)
}

fn file_revision(path: &str, bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(path);
    hash.update([0]);
    hash.update(bytes);
    format!("{:x}", hash.finalize())
}

fn is_text(text: &str) -> bool {
    !text
        .bytes()
        .any(|byte| byte.is_ascii_control() && !matches!(byte, b'\t' | b'\r' | b'\n' | 12))
}

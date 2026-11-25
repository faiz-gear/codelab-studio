use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use cap_fs_ext::DirExt;
use cap_std::fs::{Dir, OpenOptions};
use cap_tempfile::TempFile;

use super::model::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceFile};
use super::paths::{WorkspacePaths, invalid_path, is_ignored, wire_path};
use super::reader::{is_editable, validate_text, workspace_file_error, workspace_text_file};
use crate::filesystem::{open_dir_at, open_regular};
use crate::{DomainError, ErrorCode};

impl WorkspacePaths {
    pub(crate) fn list(&self, raw: &str) -> Result<Vec<WorkspaceEntry>, DomainError> {
        let path = self.resolve(raw, ErrorCode::WorkspaceFileReadFailed)?;
        let directory = open_dir_at(&self.dir, &path)
            .map_err(|err| workspace_file_error(&err, raw, ErrorCode::WorkspaceFileReadFailed))?;
        let entries = directory
            .entries()
            .map_err(|err| workspace_file_error(&err, raw, ErrorCode::WorkspaceFileReadFailed))?;
        let mut entries = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name();
                let child = path.join(&name);
                if is_ignored(&child) || self.validate(&child).is_err() {
                    return None;
                }
                let file_type = entry.file_type().ok()?;
                let kind = if file_type.is_dir() {
                    WorkspaceEntryKind::Directory
                } else if file_type.is_file() {
                    WorkspaceEntryKind::File
                } else {
                    return None;
                };
                Some(WorkspaceEntry {
                    path: wire_path(&child).ok()?,
                    name: name.into_string().ok()?,
                    kind,
                })
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| {
            (a.kind == WorkspaceEntryKind::File, &a.name)
                .cmp(&(b.kind == WorkspaceEntryKind::File, &b.name))
        });
        Ok(entries)
    }

    pub(crate) fn write(
        &self,
        raw: &str,
        text: &str,
        expected: Option<&str>,
        comparison: bool,
    ) -> Result<Result<WorkspaceFile, WorkspaceFile>, DomainError> {
        validate_text(text, raw)?;
        self.write_file(raw, text, expected, comparison)
            .map_err(|err| workspace_file_error(&err, raw, ErrorCode::WorkspaceFileWriteFailed))
    }

    fn write_file(
        &self,
        raw: &str,
        text: &str,
        expected: Option<&str>,
        comparison: bool,
    ) -> io::Result<Result<WorkspaceFile, WorkspaceFile>> {
        let before = self.read_io(raw)?;
        if comparison && raw != before.path {
            return Err(invalid_path());
        }
        if before.revision.as_deref() != expected {
            return Ok(Err(before));
        }
        if !is_editable(&before.contents) {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        let target = Path::new(&before.path);
        if comparison && before.revision.is_none() {
            self.create_missing_parents(target)?;
            let latest = self.read_io(raw)?;
            if latest.path != before.path || latest.revision.is_some() {
                return Ok(Err(latest));
            }
        }
        let (dir, path) = self.parent(target)?;
        if comparison && path != target {
            return Err(invalid_path());
        }
        let name = path.file_name().ok_or_else(invalid_path)?;
        if before.revision.is_none() {
            match create_file(&dir, Path::new(name), text.as_bytes()) {
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    return Ok(Err(self.read_io(raw)?));
                }
                result => result?,
            }
        } else {
            let mut temp = TempFile::new(&dir)?;
            temp.write_all(text.as_bytes())?;
            temp.as_file().set_permissions(
                open_regular(&dir, Path::new(name))?
                    .metadata()?
                    .permissions(),
            )?;
            temp.as_file().sync_all()?;

            let latest = self.read_io(raw)?;
            if latest.path != before.path || latest.revision != before.revision {
                return Ok(Err(latest));
            }
            temp.replace(name)?;
        }
        Ok(Ok(workspace_text_file(wire_path(&path)?, text.into())))
    }

    fn create_missing_parents(&self, path: &Path) -> io::Result<()> {
        let mut directory = self.dir.try_clone()?;
        for name in path.parent().ok_or_else(invalid_path)? {
            match directory.create_dir(name) {
                Ok(()) => (),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
                Err(error) => return Err(error),
            }
            directory = directory.open_dir_nofollow(name)?;
        }
        Ok(())
    }

    pub(crate) fn create(&self, raw: &str) -> Result<WorkspaceFile, DomainError> {
        let result = (|| {
            let input = self.input(raw)?;
            let (dir, path) = self.parent(&input)?;
            create_file(
                &dir,
                Path::new(path.file_name().ok_or_else(invalid_path)?),
                b"",
            )?;
            self.read_io(&wire_path(&path)?)
        })();
        result.map_err(|err| workspace_file_error(&err, raw, ErrorCode::WorkspaceFileWriteFailed))
    }

    pub(crate) fn rename(&self, raw: &str, new_raw: &str) -> Result<WorkspaceFile, DomainError> {
        let result = (|| {
            let source = self.input(raw)?;
            let (source_dir, source_path) = self.parent(&source)?;
            self.canonical(&source_path)?;
            let source_name = source_path.file_name().ok_or_else(invalid_path)?;
            if !source_dir.symlink_metadata(source_name)?.is_file() {
                return Err(invalid_path());
            }
            let target = self.input(new_raw)?;
            let (target_dir, target_path) = self.parent(&target)?;
            let target_name = target_path.file_name().ok_or_else(invalid_path)?;
            if source_path != target_path {
                // Reserve the destination without clobbering a file created by an IDE.
                source_dir.hard_link(source_name, &target_dir, target_name)?;
                if let Err(error) = source_dir.remove_file(source_name) {
                    let _ = target_dir.remove_file(target_name);
                    return Err(error);
                }
            }
            self.read_io(&wire_path(&target_path)?)
        })();
        result
            .map_err(|err| workspace_file_error(&err, new_raw, ErrorCode::WorkspaceFileWriteFailed))
    }

    pub(super) fn ide_path(&self, raw: &str) -> Result<PathBuf, DomainError> {
        let path = self.resolve(raw, ErrorCode::WorkspaceFileReadFailed)?;
        let absolute = self.root.join(path);
        let metadata = fs::symlink_metadata(&absolute)
            .map_err(|err| workspace_file_error(&err, raw, ErrorCode::WorkspaceFileReadFailed))?;
        if !metadata.is_file() && absolute != self.root {
            return Err(
                DomainError::new(ErrorCode::WorkspaceFileReadFailed).with_detail("path", raw)
            );
        }
        Ok(absolute)
    }

    fn resolve(&self, raw: &str, failure: ErrorCode) -> Result<PathBuf, DomainError> {
        self.input(raw)
            .and_then(|path| self.canonical(&path))
            .map_err(|err| workspace_file_error(&err, raw, failure))
    }
}

pub(super) fn create_file(dir: &Dir, name: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = dir.open_with(name, &options)?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = dir.remove_file(name);
        return Err(error);
    }
    Ok(())
}

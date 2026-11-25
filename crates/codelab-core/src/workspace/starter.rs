use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use cap_fs_ext::DirExt;
use cap_std::fs::{Dir, Metadata, OpenOptions};

use super::paths::{identity, invalid_path, is_metadata, relative_path};
use crate::course::Starter;
use crate::filesystem::{open_canonical_dir, open_dir_at, open_regular};

pub(super) fn instantiate(starter: &Starter, workspace: &Dir, metadata: &Dir) -> io::Result<()> {
    let staging = cap_tempfile::TempDir::new_in(metadata)?;
    match starter {
        Starter::Builtin(entries) => {
            for (path, bytes) in *entries {
                let path = checked_relative(Path::new(path))?;
                staging.create_dir_all(path.parent().ok_or_else(invalid_path)?)?;
                staging.create(&path)?.write_all(bytes)?;
            }
        }
        Starter::Local { root, directory } => {
            let relative = checked_relative(Path::new(directory))?;
            let course = open_canonical_dir(root)?;
            match open_dir_at(&course, &relative) {
                Ok(source) => copy_tree(&source, &staging, Path::new(""), &mut Vec::new())?,
                Err(error)
                    if error.kind() == io::ErrorKind::NotFound
                        && fs::symlink_metadata(root.join(relative))
                            .is_err_and(|err| err.kind() == io::ErrorKind::NotFound) => {}
                Err(error) => return Err(error),
            }
        }
    }

    // Validate and stage the full Starter before adding any learner-facing files.
    let mut created = Vec::new();
    if let Err(error) = copy_tree(&staging, workspace, Path::new(""), &mut created) {
        for entry in created.into_iter().rev() {
            entry.rollback(workspace);
        }
        return Err(error);
    }
    Ok(())
}

struct CreatedEntry {
    path: PathBuf,
    metadata: Metadata,
}

impl CreatedEntry {
    fn rollback(self, root: &Dir) {
        let Some(parent) = self
            .path
            .parent()
            .and_then(|path| open_dir_at(root, path).ok())
        else {
            return;
        };
        let Some(name) = self.path.file_name() else {
            return;
        };
        let Ok(current) = parent.symlink_metadata(name) else {
            return;
        };
        if identity(&current) != identity(&self.metadata) {
            return;
        }
        if current.is_dir() {
            // Leave directories that have acquired learner files.
            let _ = parent.remove_dir(name);
        } else if current.is_file()
            && current.len() == self.metadata.len()
            && current.modified().ok() == self.metadata.modified().ok()
        {
            let _ = parent.remove_file(name);
        }
    }
}

pub(super) fn checked_relative(path: &Path) -> io::Result<PathBuf> {
    let relative = relative_path(path)?;
    if relative.as_os_str().is_empty() || relative.iter().any(is_metadata) {
        return Err(invalid_path());
    }
    Ok(relative)
}

fn copy_tree(
    source: &Dir,
    target: &Dir,
    prefix: &Path,
    created: &mut Vec<CreatedEntry>,
) -> io::Result<()> {
    for entry in source.entries()? {
        let entry = entry?;
        let name = checked_relative(Path::new(&entry.file_name()))?;
        let path = prefix.join(&name);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            target.create_dir(&name)?;
            let target_child = target.open_dir_nofollow(&name)?;
            created.push(CreatedEntry {
                path: path.clone(),
                metadata: target_child.dir_metadata()?,
            });
            copy_tree(
                &source.open_dir_nofollow(&name)?,
                &target_child,
                &path,
                created,
            )?;
        } else if file_type.is_file() {
            let mut file = open_regular(source, &name)?;
            let permissions = file.metadata()?.permissions();
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            let mut copy = target.open_with(&name, &options)?;
            let result = io::copy(&mut file, &mut copy)
                .and_then(|_| copy.set_permissions(permissions))
                .and_then(|()| copy.sync_all());
            created.push(CreatedEntry {
                path,
                metadata: copy.metadata()?,
            });
            result?;
        } else {
            return Err(invalid_path());
        }
    }
    Ok(())
}

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::execution::{CommandLaunch, Directory, model::CommandCwdOverrides};
use crate::filesystem::open_dir_at;
use crate::storage::JsonStore;
use crate::{CommandBlock, DomainError, ErrorCode, LoadedCourse};

use super::paths::{WorkspacePaths, relative_path, wire_path};

pub(crate) struct CommandCwds {
    pub store: JsonStore<CommandCwdOverrides>,
    pub values: CommandCwdOverrides,
}

impl CommandCwds {
    pub fn for_course(&self, course: &LoadedCourse) -> CommandCwdOverrides {
        course
            .chapters
            .iter()
            .flat_map(|chapter| &chapter.commands)
            .filter_map(|block| {
                self.values
                    .get(&key(course, block))
                    .map(|value| (block.id.clone(), value.clone()))
            })
            .collect()
    }

    pub fn set(
        &mut self,
        paths: &WorkspacePaths,
        course: &LoadedCourse,
        block: &CommandBlock,
        raw: Option<&str>,
    ) -> Result<(), DomainError> {
        let mut next = self.values.clone();
        let key = key(course, block);
        match raw {
            Some(raw) => {
                let dir = directory(paths, raw)?;
                let relative = wire_path(
                    dir.path
                        .strip_prefix(&paths.root)
                        .map_err(|_| invalid_cwd(raw))?,
                )
                .map_err(|_| invalid_cwd(raw))?;
                next.insert(
                    key,
                    if relative.is_empty() {
                        ".".into()
                    } else {
                        relative
                    },
                );
            }
            None => {
                next.remove(&key);
            }
        }
        self.store.save_atomic(&next)?;
        self.values = next;
        Ok(())
    }

    pub fn resolve(
        &self,
        paths: &WorkspacePaths,
        course: &LoadedCourse,
        block: &CommandBlock,
        restart_terminal_id: Option<u64>,
    ) -> Result<(CommandLaunch, Directory), DomainError> {
        self.resolve_saved(paths, block, key(course, block), restart_terminal_id)
    }

    pub fn resolve_saved(
        &self,
        paths: &WorkspacePaths,
        block: &CommandBlock,
        cwd_key: String,
        restart_terminal_id: Option<u64>,
    ) -> Result<(CommandLaunch, Directory), DomainError> {
        let override_cwd = self.values.get(&cwd_key).map(String::as_str);
        let cwd = override_cwd.or(block.cwd.as_deref()).unwrap_or(".");
        let dir = directory(paths, cwd)?;
        // A leading cd used to infer this block's directory has already been
        // applied by the PTY launch. Preserve the rest of the block.
        let script = crate::course::execution_script(block);
        Ok((
            CommandLaunch {
                block: block.clone(),
                cwd: dir.path.clone(),
                script,
                restart_terminal_id,
                cwd_key,
            },
            dir,
        ))
    }
}

pub(super) fn valid_cwds(values: &CommandCwdOverrides) -> bool {
    values.iter().all(|(key, path)| {
        !key.is_empty() && !path.is_empty() && relative_path(Path::new(path)).is_ok()
    })
}

pub(crate) fn directory(paths: &WorkspacePaths, raw: &str) -> Result<Directory, DomainError> {
    let relative = paths
        .input(raw)
        .and_then(|path| paths.canonical(&path))
        .map_err(|_| invalid_cwd(raw))?;
    let handle = open_dir_at(&paths.dir, &relative).map_err(|_| invalid_cwd(raw))?;
    Ok(Directory {
        path: if relative.as_os_str().is_empty() {
            paths.root.clone()
        } else {
            paths.root.join(relative)
        },
        handle,
    })
}

fn invalid_cwd(raw: &str) -> DomainError {
    DomainError::new(ErrorCode::CommandCwdInvalid).with_detail("path", raw)
}

fn key(course: &LoadedCourse, block: &CommandBlock) -> String {
    // Command ids are positional. Use content and the occurrence within its
    // Chapter so inserting an unrelated block cannot redirect an override.
    let (chapter_id, _) = block.id.rsplit_once(":cmd-").unwrap_or((&block.id, ""));
    let ordinal = course
        .chapters
        .iter()
        .find(|chapter| chapter.chapter_id == chapter_id)
        .map_or(0, |chapter| {
            chapter
                .commands
                .iter()
                .take_while(|item| item.id != block.id)
                .filter(|item| item.command == block.command && item.shell == block.shell)
                .count()
        });
    let mut hash = Sha256::new();
    hash.update(block.shell.as_bytes());
    hash.update([0]);
    hash.update(block.command.as_bytes());
    format!("{chapter_id}:{:x}:{ordinal}", hash.finalize())
}

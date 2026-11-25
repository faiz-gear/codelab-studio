use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use cap_fs_ext::DirExt;
use cap_std::fs::Dir;
use serde::{Deserialize, Serialize};

use super::files::is_text;
use super::model::{MAX_WORKSPACE_TEXT_BYTES, WorkspaceFile, WorkspaceFileContents};
use super::paths::{WorkspacePaths, invalid_path, is_ignored, relative_path, wire_path};
use super::starter::checked_relative;
use crate::course::{ChapterReferences, CourseSource, LoadedCourse, Starter, builtin_solution};
use crate::filesystem::{open_canonical_dir, open_dir_at, open_regular};
use crate::{DomainError, ErrorCode};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceComparison {
    pub current: WorkspaceFile,
    pub proposed: String,
}

impl WorkspacePaths {
    pub(crate) fn preview_chapter(
        &self,
        course: &LoadedCourse,
        references: &ChapterReferences,
        chapter_id: &str,
    ) -> Result<Vec<WorkspaceComparison>, DomainError> {
        let chapter = course
            .chapters
            .iter()
            .find(|chapter| chapter.chapter_id == chapter_id)
            .ok_or_else(|| {
                DomainError::new(ErrorCode::CourseNotFound).with_detail("source", chapter_id)
            })?;
        if let CourseSource::Builtin { course_id } = &course.source
            && let Some(files) = builtin_solution(course_id, chapter_id)
        {
            return files
                .iter()
                .map(|(path, bytes)| self.comparison(path, checked_text(bytes, path)?))
                .collect();
        }
        if let CourseSource::LocalPack { path } = &course.source {
            let chapter_dir = relative_path(Path::new(&chapter.chapter_id))
                .ok()
                .filter(|path| path.components().count() == 1 && path.as_os_str() == chapter_id)
                .ok_or_else(|| invalid_course_path(chapter_id))?;
            let root =
                open_canonical_dir(Path::new(path)).map_err(|_| invalid_course_path(path))?;
            let directory = Path::new("solutions").join(chapter_dir);
            if let Some(snapshot) =
                open_optional_dir(&root, &directory).map_err(|_| invalid_course_path(chapter_id))?
            {
                return tree_files(&snapshot, false)
                    .map_err(|_| invalid_course_path(chapter_id))?
                    .into_iter()
                    .map(|path| {
                        let name = wire_path(&path).map_err(|_| invalid_course_path(chapter_id))?;
                        self.comparison(&name, read_text(&snapshot, &path, &name)?)
                    })
                    .collect();
            }
        }
        let mut files = BTreeMap::new();
        for (raw, text) in references.get(chapter_id).into_iter().flatten() {
            let path = relative_path(Path::new(raw)).map_err(|_| invalid_course_path(raw))?;
            let name = wire_path(&path).map_err(|_| invalid_course_path(raw))?;
            if name.is_empty() || raw.contains(['\\', ':']) || files.insert(name, text).is_some() {
                return Err(invalid_course_path(raw));
            }
        }
        files
            .into_iter()
            .map(|(path, text)| self.comparison(&path, checked_text(text.as_bytes(), &path)?))
            .collect()
    }

    pub(crate) fn list_starter(
        &self,
        starter: Option<&Starter>,
    ) -> Result<Vec<String>, DomainError> {
        let files = match starter {
            Some(Starter::Builtin(entries)) => entries
                .iter()
                .map(|(path, _)| PathBuf::from(path))
                .filter(|path| !is_ignored(path))
                .collect(),
            Some(Starter::Local { root, directory }) => {
                let Some(starter) = open_starter(root, directory)? else {
                    return Ok(Vec::new());
                };
                tree_files(&starter, true).map_err(|_| invalid_course_path(directory))?
            }
            None => return Ok(Vec::new()),
        };

        let mut paths = files
            .iter()
            .filter(|path| self.validate(path).is_ok())
            .map(|path| wire_path(path).map_err(|_| invalid_course_path(&path.to_string_lossy())))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        Ok(paths)
    }

    pub(crate) fn preview_starter(
        &self,
        starter: Option<&Starter>,
        raw: &str,
    ) -> Result<WorkspaceComparison, DomainError> {
        let target = relative_path(Path::new(raw))
            .and_then(|path| {
                self.validate(&path)?;
                Ok(path)
            })
            .map_err(|_| {
                DomainError::new(ErrorCode::WorkspacePathOutsideRoot).with_detail("path", raw)
            })?;
        let name = wire_path(&target).map_err(|_| invalid_course_path(raw))?;
        let missing =
            || DomainError::new(ErrorCode::WorkspaceFileNotFound).with_detail("path", raw);
        let proposed = match starter {
            Some(Starter::Builtin(entries)) => {
                let (_, bytes) = entries
                    .iter()
                    .find(|(path, _)| *path == name)
                    .ok_or_else(missing)?;
                checked_text(bytes, &name)?
            }
            Some(Starter::Local { root, directory }) => {
                let starter = open_starter(root, directory)?.ok_or_else(missing)?;
                read_text(&starter, &target, &name)?
            }
            None => return Err(missing()),
        };
        self.comparison(&name, proposed)
    }

    fn comparison(&self, path: &str, proposed: String) -> Result<WorkspaceComparison, DomainError> {
        let current = self.read(path)?;
        if current.path != path {
            return Err(
                DomainError::new(ErrorCode::WorkspacePathOutsideRoot).with_detail("path", path)
            );
        }
        if matches!(
            current.contents,
            WorkspaceFileContents::Binary | WorkspaceFileContents::TooLarge
        ) {
            return Err(not_text(path));
        }
        Ok(WorkspaceComparison { current, proposed })
    }
}

fn open_starter(root: &Path, directory: &str) -> Result<Option<Dir>, DomainError> {
    let path =
        checked_relative(Path::new(directory)).map_err(|_| invalid_course_path(directory))?;
    let root = open_canonical_dir(root).map_err(|_| invalid_course_path(directory))?;
    open_optional_dir(&root, &path).map_err(|_| invalid_course_path(directory))
}

fn open_optional_dir(root: &Dir, path: &Path) -> io::Result<Option<Dir>> {
    let mut dir = root.try_clone()?;
    for name in path {
        match dir.symlink_metadata(name) {
            Ok(metadata) if metadata.is_dir() => dir = dir.open_dir_nofollow(name)?,
            Ok(_) => return Err(invalid_path()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        }
    }
    Ok(Some(dir))
}

fn tree_files(root: &Dir, skip_ignored: bool) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let mut pending = vec![(root.try_clone()?, PathBuf::new())];
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
}

fn read_text(root: &Dir, path: &Path, name: &str) -> Result<String, DomainError> {
    let read = || -> io::Result<Vec<u8>> {
        let parent = open_dir_at(root, path.parent().ok_or_else(invalid_path)?)?;
        let file = open_regular(
            &parent,
            Path::new(path.file_name().ok_or_else(invalid_path)?),
        )?;
        let mut bytes = Vec::new();
        file.take((MAX_WORKSPACE_TEXT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    let bytes = read().map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            DomainError::new(ErrorCode::WorkspaceFileNotFound).with_detail("path", name)
        } else {
            invalid_course_path(name)
        }
    })?;
    checked_text(&bytes, name)
}

fn checked_text(bytes: &[u8], path: &str) -> Result<String, DomainError> {
    if bytes.len() <= MAX_WORKSPACE_TEXT_BYTES
        && let Ok(text) = std::str::from_utf8(bytes)
        && is_text(text)
    {
        return Ok(text.to_owned());
    }
    Err(not_text(path))
}

fn invalid_course_path(path: &str) -> DomainError {
    DomainError::new(ErrorCode::CoursePathInvalid).with_detail("path", path)
}

fn not_text(path: &str) -> DomainError {
    DomainError::new(ErrorCode::WorkspaceFileNotText).with_detail("path", path)
}

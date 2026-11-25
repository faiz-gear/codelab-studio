use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::model::WorkspaceFile;
use super::paths::{WorkspacePaths, is_ignored, relative_path, wire_path};
use super::reader::{SnapshotDir, decode_text, invalid_course_path};
use super::starter::checked_relative;
use crate::course::{ChapterReferences, CourseSource, LoadedCourse, Starter, builtin_solution};
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
        starter: Option<&Starter>,
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
                .map(|(path, bytes)| self.compare_bytes(path, bytes))
                .collect();
        }
        if let Some(Starter::Local { root, .. }) = starter {
            let chapter_dir = relative_path(Path::new(&chapter.chapter_id))
                .ok()
                .filter(|path| path.components().count() == 1 && path.as_os_str() == chapter_id)
                .ok_or_else(|| invalid_course_path(chapter_id))?;
            let directory = Path::new("solutions").join(chapter_dir);
            if let Some(snapshot) = SnapshotDir::open_optional(root, &directory, chapter_id)? {
                return snapshot
                    .list_all_files()?
                    .into_iter()
                    .map(|path| {
                        let name = wire_path(&path).map_err(|_| invalid_course_path(chapter_id))?;
                        self.compare_snapshot_file(&snapshot, &name)
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
            .map(|(path, text)| self.compare_bytes(&path, text.as_bytes()))
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
                starter.list_visible_files()?
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
        match starter {
            Some(Starter::Builtin(entries)) => {
                let (_, bytes) = entries
                    .iter()
                    .find(|(path, _)| *path == name)
                    .ok_or_else(missing)?;
                self.compare_bytes(&name, bytes)
            }
            Some(Starter::Local { root, directory }) => {
                let starter = open_starter(root, directory)?.ok_or_else(missing)?;
                self.compare_snapshot_file(&starter, &name)
            }
            None => Err(missing()),
        }
    }

    fn compare_bytes(
        &self,
        path: &str,
        proposed: &[u8],
    ) -> Result<WorkspaceComparison, DomainError> {
        let proposed = decode_text(proposed, path)?;
        let current = self.read_comparison_target(path)?;
        Ok(WorkspaceComparison { current, proposed })
    }

    fn compare_snapshot_file(
        &self,
        snapshot: &SnapshotDir,
        path: &str,
    ) -> Result<WorkspaceComparison, DomainError> {
        let proposed = snapshot.read_text(path)?;
        let current = self.read_comparison_target(path)?;
        Ok(WorkspaceComparison { current, proposed })
    }
}

fn open_starter(root: &Path, directory: &str) -> Result<Option<SnapshotDir>, DomainError> {
    let path =
        checked_relative(Path::new(directory)).map_err(|_| invalid_course_path(directory))?;
    SnapshotDir::open_optional(root, &path, directory)
}

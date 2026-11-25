use std::collections::BTreeMap;
use std::sync::Arc;

use cap_fs_ext::DirExt;
use cap_std::fs::Dir;
use serde::{Deserialize, Serialize};

use super::command_cwd::{CommandCwds, valid_cwds};
use super::paths::{METADATA_DIR, WorkspacePaths};
use crate::course::{CourseSource, LoadedCourse, Starter};
use crate::storage::JsonStore;
use crate::{ChapterKind, DomainError, ErrorCode, StorageRecoveryNotice, StoreKind};

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Binding {
    course_id: String,
    source: CourseSource,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    chapters: BTreeMap<String, ChapterProgress>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChapterProgress {
    completion_state: CompletionState,
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CompletionState {
    NotStarted,
    InProgress,
    Completed,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Metrics {
    course_opened: u64,
}

pub(super) struct WorkspaceMetadata {
    pub recovery_notices: Vec<StorageRecoveryNotice>,
    pub completed_exercises: u32,
    pub command_cwds: CommandCwds,
}

impl WorkspaceMetadata {
    pub fn bind(
        paths: &WorkspacePaths,
        course: &LoadedCourse,
        starter: Option<&Starter>,
    ) -> Result<Self, DomainError> {
        let is_empty = paths
            .dir
            .entries()
            .and_then(|mut entries| entries.next().transpose())
            .map_err(|err| metadata_error(ErrorCode::StorageReadFailed, err))?
            .is_none();

        let (dir, created) = metadata_dir(&paths.dir)?;
        let result = Self::load(paths, course, starter.filter(|_| is_empty), Arc::new(dir));
        // An invalid Starter can be fixed and retried when this directory was empty.
        if result.is_err() && created {
            let _ = paths.dir.remove_dir(METADATA_DIR);
        }
        result
    }

    fn load(
        paths: &WorkspacePaths,
        course: &LoadedCourse,
        starter: Option<&Starter>,
        dir: Arc<Dir>,
    ) -> Result<Self, DomainError> {
        let location = paths.root.join(METADATA_DIR);
        let binding_store = JsonStore::new(&location, "workspace.json", StoreKind::Workspace)
            .scoped(Arc::clone(&dir));
        let progress_store = JsonStore::new(&location, "progress.json", StoreKind::Progress)
            .scoped(Arc::clone(&dir));
        let metrics_store =
            JsonStore::new(&location, "metrics.json", StoreKind::Metrics).scoped(Arc::clone(&dir));

        let command_cwd_store =
            JsonStore::new(&location, "command-cwds.json", StoreKind::Workspace)
                .scoped(Arc::clone(&dir));
        let bound = Binding {
            course_id: course.course_id.clone(),
            source: course.source.clone(),
        };
        let binding = binding_store.load_or_recover_with(
            Binding {
                course_id: bound.course_id.clone(),
                source: bound.source.clone(),
            },
            |binding| !binding.course_id.trim().is_empty() && valid_source(&binding.source),
        )?;
        if binding.value.source != bound.source {
            return Err(DomainError::new(ErrorCode::WorkspaceBoundToAnotherCourse));
        }
        let progress = progress_store.load_or_recover(|progress: &Progress| {
            progress.chapters.keys().all(|id| !id.trim().is_empty())
        })?;
        let command_cwds = command_cwd_store.load_or_recover(valid_cwds)?;
        let mut metrics = metrics_store.load_or_recover(|_: &Metrics| true)?;

        if let Some(starter) = starter {
            super::starter::instantiate(starter, &paths.dir, &dir).map_err(|err| {
                DomainError::new(ErrorCode::WorkspaceStarterFailed)
                    .with_detail("reason", err.to_string())
            })?;
        }

        binding_store.save_atomic(&bound)?;
        if !dir
            .try_exists("progress.json")
            .map_err(|err| metadata_error(ErrorCode::StorageReadFailed, err))?
        {
            progress_store.save_atomic(&progress.value)?;
        }
        metrics.value.course_opened = metrics.value.course_opened.saturating_add(1);
        metrics_store.save_atomic(&metrics.value)?;

        let completed_exercises = progress.value.completed_for(course);
        Ok(Self {
            recovery_notices: [
                binding.recovery,
                progress.recovery,
                metrics.recovery,
                command_cwds.recovery,
            ]
            .into_iter()
            .flatten()
            .collect(),
            completed_exercises,
            command_cwds: CommandCwds {
                store: command_cwd_store,
                values: command_cwds.value,
            },
        })
    }
}

impl Progress {
    fn completed_for(&self, course: &LoadedCourse) -> u32 {
        course
            .chapters
            .iter()
            .filter(|chapter| {
                chapter.kind == ChapterKind::Exercise
                    && self
                        .chapters
                        .get(&chapter.chapter_id)
                        .is_some_and(|status| status.completion_state == CompletionState::Completed)
            })
            .count() as u32
    }
}

pub(super) fn completed_for(
    paths: &WorkspacePaths,
    course: &LoadedCourse,
) -> Result<u32, DomainError> {
    let dir = paths
        .dir
        .open_dir_nofollow(METADATA_DIR)
        .map_err(|err| metadata_error(ErrorCode::StorageReadFailed, err))?;
    let progress = JsonStore::<Progress>::new(
        &paths.root.join(METADATA_DIR),
        "progress.json",
        StoreKind::Progress,
    )
    .scoped(Arc::new(dir))
    .load_or_recover(|progress| progress.chapters.keys().all(|id| !id.trim().is_empty()))?;
    Ok(progress.value.completed_for(course))
}

fn valid_source(source: &CourseSource) -> bool {
    match source {
        CourseSource::Builtin { course_id } => !course_id.trim().is_empty(),
        CourseSource::GitPack { url } => {
            crate::course::git_pack::normalize_git_url(url).as_ref() == Some(url)
        }
        CourseSource::LocalMarkdown { path } | CourseSource::LocalPack { path } => {
            std::path::Path::new(path).is_absolute()
        }
    }
}

fn metadata_dir(root: &Dir) -> Result<(Dir, bool), DomainError> {
    let created = match root.create_dir(METADATA_DIR) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
        Err(error) => return Err(metadata_error(ErrorCode::StorageWriteFailed, error)),
    };
    root.open_dir_nofollow(METADATA_DIR)
        .map(|dir| (dir, created))
        .map_err(|error| metadata_error(ErrorCode::StorageReadFailed, error))
}

fn metadata_error(code: ErrorCode, error: std::io::Error) -> DomainError {
    DomainError::new(code)
        .with_detail("store", "workspace")
        .with_detail("reason", error.to_string())
}

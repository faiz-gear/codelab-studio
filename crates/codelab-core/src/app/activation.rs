use std::path::Path;
use std::sync::Arc;

use crate::course::{CourseWatcher, LoadedCourse, OpenedCourse, Starter};
use crate::workspace::{Workspace, WorkspaceSummary};

use super::{ActiveCourseState, AppEvent, AppState, Subscribers, emit_to_subscribers};

/// Owns the transition invariants shared by Course and Workspace activation.
pub(super) struct CourseWorkspaceActivation {
    data_dir: std::path::PathBuf,
    subscribers: Arc<Subscribers>,
}

impl CourseWorkspaceActivation {
    pub(super) fn new(data_dir: &Path, subscribers: &Arc<Subscribers>) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            subscribers: Arc::clone(subscribers),
        }
    }

    pub(super) fn activate_course(
        &self,
        state: &mut AppState,
        opened: OpenedCourse,
    ) -> Result<LoadedCourse, crate::DomainError> {
        let watcher = if opened.watched_files.is_empty() {
            None
        } else {
            let subscribers = Arc::clone(&self.subscribers);
            Some(CourseWatcher::start(
                opened.course.course_id.clone(),
                opened.watched_files,
                move |changed| {
                    emit_to_subscribers(&subscribers, AppEvent::CourseSourceChanged(changed));
                },
            ))
        };
        state.verification = None;
        state.execution.invalidate_confirmation();
        let recovered = state
            .active_workspace
            .as_mut()
            .map(|workspace| workspace.prepare_progress_for_course(&opened.course))
            .transpose()?
            .unwrap_or(false);
        let course = opened.course;
        state.active_course = Some(ActiveCourseState {
            course: course.clone(),
            starter: opened.starter,
            references: opened.references,
            _watcher: watcher,
        });
        if recovered {
            emit_to_subscribers(
                &self.subscribers,
                AppEvent::ProgressUpdated(super::verification::progress_snapshot(state)),
            );
        }
        emit_to_subscribers(&self.subscribers, AppEvent::CourseLoaded(course.clone()));
        Ok(course)
    }

    pub(super) fn prepare_workspace(
        &self,
        id: u64,
        path: &str,
        course: &LoadedCourse,
        starter: Option<&Starter>,
    ) -> Result<Workspace, crate::DomainError> {
        let subscribers = Arc::clone(&self.subscribers);
        Workspace::bind(id, path, &self.data_dir, course, starter, move |event| {
            emit_to_subscribers(&subscribers, AppEvent::WorkspaceFilesChanged(event));
        })
    }

    pub(super) fn activate_workspace(
        &self,
        state: &mut AppState,
        workspace: Workspace,
    ) -> WorkspaceSummary {
        state.verification = None;
        state.execution.clear();
        state.next_workspace_id += 1;
        let summary = workspace.summary.clone();
        state.active_workspace = Some(workspace);
        summary
    }
}

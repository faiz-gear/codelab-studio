use std::path::Path;
use std::sync::Arc;

use crate::course::{CourseWatcher, LoadedCourse, OpenedCourse, Starter};
use crate::tutor::TutorManager;
use crate::workspace::{Workspace, WorkspaceSummary};

use super::{ActiveCourseState, AppEvent, AppState, Subscribers, emit_to_subscribers};

/// Owns the transition invariants shared by Course and Workspace activation.
pub(super) struct CourseWorkspaceActivation {
    data_dir: std::path::PathBuf,
    subscribers: Arc<Subscribers>,
    tutor: Arc<TutorManager>,
}

impl CourseWorkspaceActivation {
    pub(super) fn new(
        data_dir: &Path,
        subscribers: &Arc<Subscribers>,
        tutor: &Arc<TutorManager>,
    ) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            subscribers: Arc::clone(subscribers),
            tutor: Arc::clone(tutor),
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
        mut workspace: Workspace,
    ) -> WorkspaceSummary {
        state.verification = None;
        state.execution.clear();
        state.next_workspace_id += 1;
        let summary = workspace.summary.clone();
        if let Some(attachment) = workspace.take_tutor_attachment() {
            self.tutor.attach_workspace(attachment);
        } else {
            self.tutor.detach_workspace();
        }
        state.active_workspace = Some(workspace);
        summary
    }
}

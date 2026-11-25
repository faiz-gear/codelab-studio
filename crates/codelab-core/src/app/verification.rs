use super::*;
use crate::execution::preparation::{CommandPreparation, ConfirmationTarget};
use crate::execution::verification::{VerificationTask, model::*};
use crate::{Chapter, ChapterKind};
use std::sync::atomic::AtomicBool;

mod run;

impl App {
    pub(super) fn get_progress(&self, id: u64) -> Result<AppResponse, DomainError> {
        let state = self.lock_state();
        state.workspace(id)?;
        Ok(AppResponse::Progress(progress_snapshot(&state)))
    }
    pub(super) fn run_checks(&self, request: RunChecks) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let id = request.target.workspace_id;
        let workspace = state.workspace(id)?;
        if state
            .verification
            .as_ref()
            .is_some_and(|task| task.chapter_id.is_some())
        {
            return Err(DomainError::new(ErrorCode::VerificationBusy));
        }
        let chapter = exercise(&state, &request.target)?.clone();
        let course = &state.course()?.course;
        let plan = run::VerificationPlan::prepare(course, chapter, workspace)?;
        let target = plan.confirmation_target(workspace.files.root.clone());
        let AppState {
            execution,
            active_workspace,
            ..
        } = &mut *state;
        let saved = match execution.prepare_command(
            active_workspace.as_ref().unwrap(),
            target,
            request.confirmation,
            request.drafts,
        ) {
            CommandPreparation::Ready {
                target: ConfirmationTarget::Checks { chapter_id, .. },
                saved,
            } if chapter_id == plan.chapter().chapter_id => saved,
            CommandPreparation::Ready { .. } => {
                unreachable!("Check preparation target")
            }
            CommandPreparation::ConfirmationRequired(prompt) => {
                return Ok(AppResponse::CommandConfirmationRequired(prompt));
            }
            CommandPreparation::SaveConflict(conflict) => {
                return Ok(AppResponse::CommandSaveConflict(conflict));
            }
            CommandPreparation::Failed(failed) => return Ok(AppResponse::CommandFailed(failed)),
        };
        let workspace = state.active_workspace.as_mut().unwrap();
        if let Err(error) = workspace.start_checks(plan.chapter(), plan.check_ids()) {
            return Ok(AppResponse::CommandFailed(CommandFailed { error, saved }));
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        state.verification = Some(VerificationTask {
            chapter_id: Some(plan.chapter().chapter_id.clone()),
            cancelled: Arc::clone(&cancelled),
            handle: None,
        });
        let initial = progress_snapshot(&state);
        let shared = Arc::clone(&self.state);
        let subscribers = Arc::clone(&self.subscribers);
        let recents_dir = self.data_dir.clone();
        let report = self.runtime.snapshot();
        let handle = run::VerificationRun::new(plan, report, cancelled).spawn(
            shared,
            subscribers,
            recents_dir,
        );
        state.verification.as_mut().unwrap().handle = Some(handle);
        Ok(AppResponse::VerificationStarted(VerificationStarted {
            progress: initial,
            saved,
        }))
    }
    pub(super) fn set_self_check(&self, request: SetSelfCheck) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let chapter = exercise(&state, &request.target)?.clone();
        state.active_workspace.as_mut().unwrap().set_self_check(
            &chapter,
            &request.check_id,
            request.checked,
        )?;
        self.publish_progress(&mut state)
    }
    pub(super) fn complete_exercise(
        &self,
        target: ExerciseTarget,
    ) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let chapter = exercise(&state, &target)?.clone();
        state
            .active_workspace
            .as_mut()
            .unwrap()
            .complete_exercise(&chapter)?;
        self.publish_progress(&mut state)
    }
    pub(super) fn reset_progress(&self, id: u64) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        state.workspace(id)?;
        state.verification = None;
        state.active_workspace.as_mut().unwrap().reset_progress()?;
        self.publish_progress(&mut state)
    }
    fn publish_progress(&self, state: &mut AppState) -> Result<AppResponse, DomainError> {
        publish_progress_snapshot(state, &self.recents_store, &self.subscribers)
            .map(AppResponse::Progress)
    }
    pub(super) fn export_learning_record(&self, id: u64) -> Result<AppResponse, DomainError> {
        let state = self.lock_state();
        state.workspace(id)?;
        Ok(AppResponse::LearningRecord(
            serde_json::json!({ "schemaVersion": 1, "courseId": state.course()?.course.course_id, "exportedAt": Utc::now().to_rfc3339(), "progress": progress_snapshot(&state) }),
        ))
    }
}
fn exercise<'a>(state: &'a AppState, target: &ExerciseTarget) -> Result<&'a Chapter, DomainError> {
    state.workspace(target.workspace_id)?;
    state
        .course()?
        .course
        .chapters
        .iter()
        .find(|c| c.chapter_id == target.chapter_id && c.kind == ChapterKind::Exercise)
        .ok_or_else(|| DomainError::new(ErrorCode::CheckNotFound))
}
pub(super) fn progress_snapshot(state: &AppState) -> LearningProgress {
    let workspace = state.active_workspace.as_ref().unwrap();
    LearningProgress {
        workspace_id: workspace.summary.id,
        chapters: workspace.progress_chapters(),
        completed_exercises: workspace.completed_exercises(),
        total_exercises: state.active_course.as_ref().unwrap().course.total_exercises,
        running_chapter_id: state
            .verification
            .as_ref()
            .and_then(|task| task.chapter_id.clone()),
    }
}
fn publish_progress_snapshot(
    state: &mut AppState,
    store: &JsonStore<Vec<RecentWorkspace>>,
    subscribers: &Subscribers,
) -> Result<LearningProgress, DomainError> {
    let course = &state.active_course.as_ref().unwrap().course;
    let workspace = state.active_workspace.as_mut().unwrap();
    let mut recents = state.recents.clone();
    for recent in &mut recents {
        if recent.workspace_path == workspace.summary.root {
            recent.completed_exercises = workspace.completed_exercises();
            recent.total_exercises = course.total_exercises;
        }
    }
    store.save_atomic(&recents)?;
    state.recents = recents;
    let snapshot = progress_snapshot(state);
    emit_to_subscribers(subscribers, AppEvent::RecentsUpdated(state.recents.clone()));
    emit_to_subscribers(subscribers, AppEvent::ProgressUpdated(snapshot.clone()));
    Ok(snapshot)
}

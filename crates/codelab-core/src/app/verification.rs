use super::*;
use crate::execution::preparation::{CommandPreparation, ConfirmationTarget};
use crate::execution::verification::model::*;
use crate::{Chapter, ChapterKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

mod run;

struct VerificationTask {
    cancelled: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl VerificationTask {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

impl Drop for VerificationTask {
    fn drop(&mut self) {
        self.cancel();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Owns the active Check worker handle outside `Mutex<AppState>`.
///
/// Lock order is strictly `VerificationController` (outer) -> `AppState` (inner).
/// The Check worker thread never locks `VerificationController`; callers stopping
/// an active run signal cancellation, drop `AppState`, and join the worker while
/// still holding the outer `VerificationController` lock.
#[derive(Default)]
pub(super) struct VerificationController {
    task: Mutex<Option<VerificationTask>>,
}

pub(super) struct VerificationGuard<'a> {
    slot: MutexGuard<'a, Option<VerificationTask>>,
}

impl VerificationController {
    pub(super) fn lock(&self) -> VerificationGuard<'_> {
        VerificationGuard {
            slot: self
                .task
                .lock()
                .unwrap_or_else(|poison| poison.into_inner()),
        }
    }
}

impl VerificationGuard<'_> {
    pub(super) fn stop_and_relock<'s>(
        &mut self,
        state_lock: &'s Mutex<AppState>,
        mut state: MutexGuard<'s, AppState>,
    ) -> MutexGuard<'s, AppState> {
        if let Some(task) = self.slot.take() {
            task.cancel();
            if task.is_finished() {
                drop(task);
            } else {
                drop(state);
                drop(task);
                state = lock_state(state_lock);
            }
        }
        state.running_verification_chapter = None;
        state
    }

    fn reap_finished(&mut self) {
        if self
            .slot
            .as_ref()
            .is_some_and(VerificationTask::is_finished)
        {
            drop(self.slot.take());
        }
    }

    fn is_idle(&self) -> bool {
        self.slot.is_none()
    }

    fn replace(&mut self, task: VerificationTask) -> Option<VerificationTask> {
        self.slot.replace(task)
    }
}

impl App {
    pub(super) fn get_progress(&self, id: u64) -> Result<AppResponse, DomainError> {
        let state = self.lock_state();
        state.workspace(id)?;
        Ok(AppResponse::Progress(progress_snapshot(&state)))
    }
    pub(super) fn run_checks(&self, request: RunChecks) -> Result<AppResponse, DomainError> {
        let mut verification = self.verification.lock();
        verification.reap_finished();
        let mut state = self.lock_state();
        if verification.is_idle() {
            state.running_verification_chapter = None;
        }
        let id = request.target.workspace_id;
        let workspace = state.workspace(id)?;
        if state.running_verification_chapter.is_some() {
            return Err(DomainError::new(ErrorCode::VerificationBusy));
        }
        let chapter = exercise(&state, &request.target)?.clone();
        let course = &state.course()?.course;
        let plan = run::VerificationPlan::prepare(course, chapter, workspace)?;
        let had_confirmation = request.confirmation.is_some();
        if !had_confirmation {
            state.record_metric(MetricEventDetail::VerifyChapterClicked {
                check_count: plan.check_count(),
                requires_daemon: plan.has_daemon(),
            });
        }
        let target =
            plan.confirmation_target(state.active_workspace.as_ref().unwrap().files.root.clone());
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
            } if chapter_id == plan.chapter().chapter_id => {
                if had_confirmation {
                    state.record_metric(MetricEventDetail::CmdRiskModalAction {
                        source: MetricRiskSource::Verification,
                        decision: MetricRiskDecision::Confirm,
                    });
                }
                saved
            }
            CommandPreparation::Ready { .. } => {
                unreachable!("Check preparation target")
            }
            CommandPreparation::ConfirmationRequired(prompt) => {
                return Ok(AppResponse::CommandConfirmationRequired(prompt));
            }
            CommandPreparation::SaveConflict(conflict) => {
                if had_confirmation {
                    state.record_metric(MetricEventDetail::CmdRiskModalAction {
                        source: MetricRiskSource::Verification,
                        decision: MetricRiskDecision::Confirm,
                    });
                }
                return Ok(AppResponse::CommandSaveConflict(conflict));
            }
            CommandPreparation::Failed(failed) => return Ok(AppResponse::CommandFailed(failed)),
        };
        let workspace = state.active_workspace.as_mut().unwrap();
        if let Err(error) = workspace.start_checks(plan.chapter(), plan.check_ids()) {
            return Ok(AppResponse::CommandFailed(CommandFailed { error, saved }));
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        state.running_verification_chapter = Some(plan.chapter().chapter_id.clone());
        let initial = progress_snapshot(&state);
        let shared = Arc::clone(&self.state);
        let subscribers = Arc::clone(&self.subscribers);
        let recents_store = Arc::clone(&self.recents_store);
        let report = self.runtime.snapshot();
        let handle = run::VerificationRun::new(plan, report, Arc::clone(&cancelled)).spawn(
            shared,
            subscribers,
            recents_store,
        );
        let previous = verification.replace(VerificationTask {
            cancelled,
            handle: Some(handle),
        });
        drop(state);
        drop(previous);
        Ok(AppResponse::VerificationStarted(VerificationStarted {
            progress: initial,
            saved,
        }))
    }
    pub(super) fn set_self_check(&self, request: SetSelfCheck) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let chapter = exercise(&state, &request.target)?.clone();
        let total_exercises = state.course()?.course.total_exercises;
        let completed_now = state.active_workspace.as_mut().unwrap().set_self_check(
            &chapter,
            &request.check_id,
            request.checked,
        )?;
        if completed_now {
            record_chapter_done(
                &mut state,
                MetricCompletionTrigger::SelfCheck,
                total_exercises,
            );
        }
        self.publish_progress(&mut state)
    }
    pub(super) fn complete_exercise(
        &self,
        target: ExerciseTarget,
    ) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let chapter = exercise(&state, &target)?.clone();
        let total_exercises = state.course()?.course.total_exercises;
        let completed_now = state
            .active_workspace
            .as_mut()
            .unwrap()
            .complete_exercise(&chapter)?;
        if completed_now {
            record_chapter_done(&mut state, MetricCompletionTrigger::Manual, total_exercises);
        }
        self.publish_progress(&mut state)
    }
    pub(super) fn reset_progress(&self, id: u64) -> Result<AppResponse, DomainError> {
        let mut verification = self.verification.lock();
        let state = self.lock_state();
        state.workspace(id)?;
        let mut state = verification.stop_and_relock(&self.state, state);
        state.workspace(id)?;
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
        running_chapter_id: state.running_verification_chapter.clone(),
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

fn record_chapter_done(
    state: &mut AppState,
    trigger: MetricCompletionTrigger,
    total_exercises: u32,
) {
    let completed_exercises = state
        .active_workspace
        .as_ref()
        .unwrap()
        .completed_exercises();
    state.record_metric(MetricEventDetail::ChapterMarkedDone {
        trigger,
        completed_exercises,
        total_exercises,
    });
}

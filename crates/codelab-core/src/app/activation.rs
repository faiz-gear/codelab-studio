use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::Utc;

use crate::course::authoring::{Draft, TrialWorkspaceOpened};
use crate::course::git_pack;
use crate::course::{
    self, CourseSource, CourseWatcher, GitFetchAdapter, LoadedCourse, OpenCourseRequest,
    OpenedCourse, Starter,
};
use crate::error::{DomainError, ErrorCode};
use crate::model::{RecentWorkspace, ShutdownReport, TrustEntry, TrustSource, TrustSourceKind};
use crate::storage::JsonStore;
use crate::tutor::{TutorManager, TutorWorkspaceAttachment};
use crate::workspace::{
    LocalMetricRecord, MetricEventDetail, MetricEventName, Workspace, WorkspaceSummary,
};

use super::verification::{VerificationController, VerificationGuard, progress_snapshot};
use super::{
    ActiveCourseState, AppEvent, AppResponse, AppState, CourseTrustChanged, RecentWorkspaceOpened,
    Subscribers, emit_to_subscribers, lock_state, matches_trust_key, normalize_absolute_path,
    stage_recent,
};

enum TutorAction {
    Attach(TutorWorkspaceAttachment),
    Detach,
    StopProcess,
    Shutdown,
}

struct PendingTutorAction {
    generation: u64,
    action: TutorAction,
}

#[derive(Default)]
struct TutorDispatchState {
    last_workspace_gen: u64,
    shut_down: bool,
}

pub(super) struct SessionLifecycleDeps {
    pub(super) data_dir: PathBuf,
    pub(super) state: Arc<Mutex<AppState>>,
    pub(super) verification: Arc<VerificationController>,
    pub(super) recents_store: Arc<JsonStore<Vec<RecentWorkspace>>>,
    pub(super) trust_store: JsonStore<Vec<TrustEntry>>,
    pub(super) subscribers: Arc<Subscribers>,
    pub(super) tutor: Arc<TutorManager>,
    pub(super) git: Arc<dyn GitFetchAdapter>,
}

/// Owns all coupled Course, Workspace, Draft, Trust, and Shutdown transitions
/// under strict `VerificationController` (outer) -> `AppState` (inner) lock
/// ordering, executing `TutorManager` side effects and emitting transition
/// events only after both locks drop.
pub(super) struct SessionLifecycle {
    data_dir: PathBuf,
    state: Arc<Mutex<AppState>>,
    verification: Arc<VerificationController>,
    recents_store: Arc<JsonStore<Vec<RecentWorkspace>>>,
    trust_store: JsonStore<Vec<TrustEntry>>,
    subscribers: Arc<Subscribers>,
    tutor: Arc<TutorManager>,
    tutor_dispatch: Mutex<TutorDispatchState>,
    git: Arc<dyn GitFetchAdapter>,
}

impl SessionLifecycle {
    pub(super) fn new(deps: SessionLifecycleDeps) -> Self {
        Self {
            data_dir: deps.data_dir,
            state: deps.state,
            verification: deps.verification,
            recents_store: deps.recents_store,
            trust_store: deps.trust_store,
            subscribers: deps.subscribers,
            tutor: deps.tutor,
            tutor_dispatch: Mutex::new(TutorDispatchState::default()),
            git: deps.git,
        }
    }

    pub(super) fn open_course(
        &self,
        request: OpenCourseRequest,
    ) -> Result<AppResponse, DomainError> {
        let (verification, state) = self.lock_session();
        self.load_course(verification, state, &request)
    }

    pub(super) fn reload_active_course(&self) -> Result<AppResponse, DomainError> {
        let (verification, state) = self.lock_session();
        let active = state.course()?;
        if state.trial_workspace.is_some() {
            return Ok(AppResponse::LoadedCourse(active.course.clone()));
        }
        let request = request_from_source(&active.course.source);
        self.load_course(verification, state, &request)
    }

    pub(super) fn refresh_git_course(&self) -> Result<AppResponse, DomainError> {
        let (course, events) = {
            let (verification, state) = self.lock_session();
            let CourseSource::GitPack { url } = &state.course()?.course.source else {
                return Err(DomainError::new(ErrorCode::NoActiveGitCourse));
            };
            let request = OpenCourseRequest::GitPack { url: url.clone() };
            let (mut verification, state, opened) =
                self.prepare_course(verification, state, &request, true)?;
            let mut state = verification.stop_and_relock(&self.state, state);
            let mut events = Vec::new();
            let course = self.activate_course(&mut state, opened, &mut events)?;
            if let Some(workspace) = &state.active_workspace {
                let recent = workspace_recent(workspace, &course);
                let (_, recents_event) = stage_recent(&mut state, &self.recents_store, recent)?;
                events.push(recents_event);
            }
            (course, events)
        };
        self.finish_transition(None, events);
        Ok(AppResponse::LoadedCourse(course))
    }

    pub(super) fn close_active_course(&self) -> AppResponse {
        let tutor_action = {
            let (mut verification, state) = self.lock_session();
            let mut state = verification.stop_and_relock(&self.state, state);
            state.course_load_generation += 1;
            state.deactivate_workspace();
            state.trial_workspace = None;
            state.active_course = None;
            stamp_tutor(&mut state, TutorAction::Detach)
        };
        self.finish_transition(Some(tutor_action), Vec::new());
        AppResponse::CourseClosed
    }

    pub(super) fn bind_workspace(&self, path: &str) -> Result<AppResponse, DomainError> {
        let (summary, tutor_action, recents_event) = {
            let (mut verification, state) = self.lock_session();
            let active = state.trusted_course()?;
            if state.trial_workspace.is_some() {
                return Err(DomainError::new(ErrorCode::InvalidWorkspacePath));
            }
            let course = active.course.clone();
            let starter = active.starter.clone();
            let (_state, summary, tutor_action, recents_event) = self.bind_and_activate_workspace(
                &mut verification,
                state,
                path,
                &course,
                starter.as_ref(),
            )?;
            (summary, tutor_action, recents_event)
        };
        self.finish_transition(Some(tutor_action), vec![recents_event]);
        Ok(AppResponse::WorkspaceBound(summary))
    }

    pub(super) fn open_recent_workspace(&self, path: &str) -> Result<AppResponse, DomainError> {
        let normalized = normalize_absolute_path(path)
            .ok_or_else(|| DomainError::new(ErrorCode::InvalidWorkspacePath))?;
        let (result, tutor_action, events) = {
            let (verification, state) = self.lock_session();
            let recent = state
                .recents
                .iter()
                .find(|recent| recent.workspace_path == path || recent.workspace_path == normalized)
                .ok_or_else(|| DomainError::new(ErrorCode::InvalidWorkspacePath))?;
            let request = match &recent.course_source {
                Some(source) => request_from_source(source),
                None => OpenCourseRequest::Builtin {
                    course_id: recent.course_id.clone(),
                },
            };
            let (mut verification, state, opened) =
                self.prepare_course(verification, state, &request, false)?;
            let mut events = Vec::new();
            let (mut state, workspace, tutor_action) = if opened.course.trusted {
                let (state, summary, action, recents_event) = self.bind_and_activate_workspace(
                    &mut verification,
                    state,
                    path,
                    &opened.course,
                    opened.starter.as_ref(),
                )?;
                events.push(recents_event);
                (state, Some(summary), action)
            } else {
                let mut state = verification.stop_and_relock(&self.state, state);
                state.deactivate_workspace();
                let action = stamp_tutor(&mut state, TutorAction::Detach);
                (state, None, action)
            };
            state.trial_workspace = None;
            let course = self.activate_course(&mut state, opened, &mut events);
            (
                course.map(|course| {
                    AppResponse::RecentWorkspaceOpened(RecentWorkspaceOpened { course, workspace })
                }),
                tutor_action,
                events,
            )
        };
        self.finish_transition(Some(tutor_action), events);
        result
    }

    pub(super) fn open_draft(
        &self,
        open: impl FnOnce(u64, &Path, Option<&Path>) -> Result<Draft, DomainError>,
    ) -> Result<AppResponse, DomainError> {
        let (summary, tutor_action) = {
            let (mut verification, state) = self.lock_session();
            if state.shut_down {
                return Err(DomainError::new(ErrorCode::NoActiveDraft));
            }
            let workspace = state
                .active_workspace
                .as_ref()
                .map(|workspace| workspace.files.root.as_path());
            let draft = open(state.next_draft_id, &self.data_dir, workspace)?;
            let summary = draft.summary()?;
            let mut state = verification.stop_and_relock(&self.state, state);
            state.course_load_generation += 1;
            state.next_draft_id += 1;
            state.deactivate_workspace();
            state.trial_workspace = None;
            state.active_course = None;
            state.draft = Some(draft);
            let tutor_action = stamp_tutor(&mut state, TutorAction::Detach);
            (summary, tutor_action)
        };
        self.finish_transition(Some(tutor_action), Vec::new());
        Ok(AppResponse::CourseDraft(summary))
    }

    pub(super) fn start_trial_workspace(&self, draft_id: u64) -> Result<AppResponse, DomainError> {
        let (result, tutor_action, events) = {
            let (mut verification, state) = self.lock_session();
            let draft = state.draft(draft_id)?;
            let path = draft.paths.root.to_string_lossy().into_owned();
            let source = CourseSource::LocalPack { path: path.clone() };
            if !draft.scaffolded && !is_course_trusted(&source, &state.trust_entries) {
                return Ok(AppResponse::TrialTrustRequired(TrustSource {
                    source_kind: TrustSourceKind::LocalPath,
                    source: path,
                }));
            }
            let mut opened = draft.trial_course()?;
            if opened.course.source != source {
                return Err(DomainError::new(ErrorCode::InvalidDraftPath));
            }
            opened.course.trusted = true;
            let trial = tempfile::Builder::new()
                .prefix("codelab-trial-")
                .tempdir()
                .map_err(|_| DomainError::new(ErrorCode::TrialWorkspaceFailed))?;
            let workspace = self.prepare_workspace(
                state.next_workspace_id,
                &trial.path().to_string_lossy(),
                &opened.course,
                opened.starter.as_ref(),
            )?;
            let mut state = verification.stop_and_relock(&self.state, state);
            let (workspace, tutor_action) =
                self.activate_workspace(&mut state, workspace, &opened.course)?;
            state.course_load_generation += 1;
            state.trial_workspace = Some(trial);
            let mut events = Vec::new();
            let course = self.activate_course(&mut state, opened, &mut events);
            (
                course.map(|course| {
                    AppResponse::TrialWorkspaceOpened(TrialWorkspaceOpened { course, workspace })
                }),
                tutor_action,
                events,
            )
        };
        self.finish_transition(Some(tutor_action), events);
        result
    }

    pub(super) fn mutate_trust(
        &self,
        mutate: impl FnOnce(&mut Vec<TrustEntry>),
    ) -> Result<Vec<TrustEntry>, DomainError> {
        let (next, tutor_action, events) = {
            let (mut verification, mut state) = self.lock_session();
            let mut next = state.trust_entries.clone();
            mutate(&mut next);
            self.trust_store.save_atomic(&next)?;
            state.trust_entries = next.clone();

            let scaffolded_trial = state.trial_workspace.is_some()
                && state.draft.as_ref().is_some_and(|draft| draft.scaffolded);
            let changed = state.active_course.as_mut().and_then(|active| {
                if scaffolded_trial {
                    return None;
                }
                let trusted = is_course_trusted(&active.course.source, &next);
                if std::mem::replace(&mut active.course.trusted, trusted) == trusted {
                    return None;
                }
                Some(CourseTrustChanged {
                    source: active.course.source.clone(),
                    trusted,
                })
            });
            let mut tutor_action = None;
            let mut events = Vec::new();
            if let Some(changed) = changed {
                if let Some(workspace) = &state.active_workspace {
                    workspace.set_trusted(changed.trusted);
                }
                if !changed.trusted {
                    state = verification.stop_and_relock(&self.state, state);
                    state.stop_workspace_activity();
                    tutor_action = Some(stamp_tutor(&mut state, TutorAction::StopProcess));
                }
                events.push(AppEvent::CourseTrustChanged(changed));
            }
            events.push(AppEvent::TrustUpdated(next.clone()));
            (next, tutor_action, events)
        };
        self.finish_transition(tutor_action, events);
        Ok(next)
    }

    pub(super) fn shutdown(&self) -> Result<ShutdownReport, DomainError> {
        let (hooks, active_course, active_workspace, trial_workspace, tutor_action) = {
            let (mut verification, mut state) = self.lock_session();
            if state.shut_down {
                return Ok(ShutdownReport {
                    already_shut_down: true,
                    hooks_executed: 0,
                });
            }
            state.shut_down = true;
            let mut state = verification.stop_and_relock(&self.state, state);
            state.stop_workspace_activity();
            let tutor_action = stamp_tutor(&mut state, TutorAction::Shutdown);
            (
                std::mem::take(&mut state.cleanup_hooks),
                state.active_course.take(),
                state.active_workspace.take(),
                state.trial_workspace.take(),
                tutor_action,
            )
        };

        drop(active_workspace);
        drop(active_course);
        drop(trial_workspace);
        self.apply_tutor_action(tutor_action);

        let mut hooks_executed = 0;
        let mut failures = Vec::new();
        for (name, hook) in hooks.into_iter().rev() {
            hooks_executed += 1;
            if let Err(reason) = hook() {
                failures.push(format!("{name}: {reason}"));
            }
        }

        let report = ShutdownReport {
            already_shut_down: false,
            hooks_executed,
        };
        emit_to_subscribers(
            &self.subscribers,
            AppEvent::ShutdownCompleted(report.clone()),
        );

        if !failures.is_empty() {
            return Err(DomainError::new(ErrorCode::ShutdownFailed)
                .with_detail("failures", failures.join("; ")));
        }

        Ok(report)
    }

    fn lock_session(&self) -> (VerificationGuard<'_>, MutexGuard<'_, AppState>) {
        let verification = self.verification.lock();
        let state = lock_state(&self.state);
        (verification, state)
    }

    fn apply_tutor_action(&self, pending: PendingTutorAction) {
        let mut dispatch = self
            .tutor_dispatch
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        match pending.action {
            TutorAction::Attach(attachment) => {
                if dispatch.shut_down || pending.generation <= dispatch.last_workspace_gen {
                    return;
                }
                dispatch.last_workspace_gen = pending.generation;
                self.tutor.attach_workspace(attachment);
            }
            TutorAction::Detach => {
                if dispatch.shut_down || pending.generation <= dispatch.last_workspace_gen {
                    return;
                }
                dispatch.last_workspace_gen = pending.generation;
                self.tutor.detach_workspace();
            }
            TutorAction::StopProcess => {
                if dispatch.shut_down || pending.generation < dispatch.last_workspace_gen {
                    return;
                }
                self.tutor.stop_process();
            }
            TutorAction::Shutdown => {
                dispatch.shut_down = true;
                dispatch.last_workspace_gen = dispatch.last_workspace_gen.max(pending.generation);
                self.tutor.shutdown();
            }
        }
    }

    fn finish_transition(&self, tutor_action: Option<PendingTutorAction>, events: Vec<AppEvent>) {
        if let Some(action) = tutor_action {
            self.apply_tutor_action(action);
        }
        for event in events {
            emit_to_subscribers(&self.subscribers, event);
        }
    }

    fn load_course(
        &self,
        verification: VerificationGuard<'_>,
        state: MutexGuard<'_, AppState>,
        request: &OpenCourseRequest,
    ) -> Result<AppResponse, DomainError> {
        let (result, tutor_action, events) = {
            let (mut verification, state, opened) =
                self.prepare_course(verification, state, request, false)?;
            let mut state = verification.stop_and_relock(&self.state, state);
            let closed_trial = state.close_trial();
            let switched_course = state
                .active_course
                .as_ref()
                .is_some_and(|active| active.course.source != opened.course.source);
            if switched_course {
                state.deactivate_workspace();
            }
            let tutor_action = (closed_trial || switched_course)
                .then(|| stamp_tutor(&mut state, TutorAction::Detach));
            let mut events = Vec::new();
            let course = self.activate_course(&mut state, opened, &mut events);
            (course.map(AppResponse::LoadedCourse), tutor_action, events)
        };
        self.finish_transition(tutor_action, events);
        result
    }

    fn prepare_course<'a>(
        &'a self,
        mut verification: VerificationGuard<'a>,
        mut state: MutexGuard<'a, AppState>,
        request: &OpenCourseRequest,
        refresh: bool,
    ) -> Result<
        (
            VerificationGuard<'a>,
            MutexGuard<'a, AppState>,
            OpenedCourse,
        ),
        DomainError,
    > {
        if state.shut_down {
            return Err(DomainError::new(ErrorCode::NoActiveCourse));
        }
        state.course_load_generation += 1;
        let mut opened = if let OpenCourseRequest::GitPack { url } = request {
            let generation = state.course_load_generation;
            // Release both locks while fetching so CloseActiveCourse is never blocked.
            drop(state);
            drop(verification);
            let prepared = git_pack::prepare(&self.data_dir, url, self.git.as_ref(), refresh);
            (verification, state) = self.lock_session();
            if state.shut_down || generation != state.course_load_generation {
                return Err(DomainError::new(if refresh {
                    ErrorCode::NoActiveGitCourse
                } else {
                    ErrorCode::NoActiveCourse
                }));
            }
            prepared?.publish()?
        } else {
            course::open_local_course(request, &self.data_dir)?
        };
        opened.course.trusted = is_course_trusted(&opened.course.source, &state.trust_entries);
        Ok((verification, state, opened))
    }

    fn activate_course(
        &self,
        state: &mut AppState,
        opened: OpenedCourse,
        events: &mut Vec<AppEvent>,
    ) -> Result<LoadedCourse, DomainError> {
        debug_assert!(state.running_verification_chapter.is_none());
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
            events.push(AppEvent::ProgressUpdated(progress_snapshot(state)));
        }
        events.push(AppEvent::CourseLoaded(course.clone()));
        Ok(course)
    }

    fn bind_and_activate_workspace<'a>(
        &'a self,
        verification: &mut VerificationGuard<'_>,
        mut state: MutexGuard<'a, AppState>,
        path: &str,
        course: &LoadedCourse,
        starter: Option<&Starter>,
    ) -> Result<
        (
            MutexGuard<'a, AppState>,
            WorkspaceSummary,
            PendingTutorAction,
            AppEvent,
        ),
        DomainError,
    > {
        state.protect_draft_path(path)?;
        let rebinding_active = is_rebinding_active_workspace(&state, path, course);
        if rebinding_active {
            // Stop an active Check before re-reading the same workspace's progress.json.
            state = verification.stop_and_relock(&self.state, state);
        }
        let workspace = self.prepare_workspace(state.next_workspace_id, path, course, starter)?;
        if !rebinding_active {
            state = verification.stop_and_relock(&self.state, state);
        }
        let recent = workspace_recent(&workspace, course);
        let (_, recents_event) = stage_recent(&mut state, &self.recents_store, recent)?;
        let (summary, tutor_action) = self.activate_workspace(&mut state, workspace, course)?;
        Ok((state, summary, tutor_action, recents_event))
    }

    fn prepare_workspace(
        &self,
        id: u64,
        path: &str,
        course: &LoadedCourse,
        starter: Option<&Starter>,
    ) -> Result<Workspace, DomainError> {
        let subscribers = Arc::clone(&self.subscribers);
        Workspace::bind(id, path, &self.data_dir, course, starter, move |event| {
            emit_to_subscribers(&subscribers, AppEvent::WorkspaceFilesChanged(event));
        })
    }

    fn activate_workspace(
        &self,
        state: &mut AppState,
        mut workspace: Workspace,
        course: &LoadedCourse,
    ) -> Result<(WorkspaceSummary, PendingTutorAction), DomainError> {
        state.stop_workspace_activity();
        let mut pending = std::mem::take(&mut state.pending_metrics);
        if !pending
            .iter()
            .any(|record| record.event == MetricEventName::CourseOpened)
        {
            pending.push(LocalMetricRecord::now(MetricEventDetail::CourseOpened {
                course_kind: (&course.source).into(),
                language: course.language,
            }));
        }
        workspace.record_metric_records(pending)?;
        state.next_workspace_id += 1;
        let summary = workspace.summary.clone();
        let tutor_action = stamp_tutor(
            state,
            match workspace.take_tutor_attachment() {
                Some(attachment) => TutorAction::Attach(attachment),
                None => TutorAction::Detach,
            },
        );
        state.active_workspace = Some(workspace);
        Ok((summary, tutor_action))
    }
}

fn stamp_tutor(state: &mut AppState, action: TutorAction) -> PendingTutorAction {
    state.tutor_generation += 1;
    PendingTutorAction {
        generation: state.tutor_generation,
        action,
    }
}

fn is_rebinding_active_workspace(state: &AppState, raw_path: &str, course: &LoadedCourse) -> bool {
    let Some(active) = &state.active_workspace else {
        return false;
    };
    let path = Path::new(raw_path);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| component == Component::ParentDir)
    {
        return false;
    }
    let Ok(root) = fs::canonicalize(path) else {
        return false;
    };
    if root != active.files.root {
        return false;
    }
    if let CourseSource::LocalPack { path: pack } = &course.source {
        let pack = Path::new(pack);
        if root.starts_with(pack) || pack.starts_with(&root) {
            return false;
        }
    }
    true
}

fn is_course_trusted(source: &CourseSource, entries: &[TrustEntry]) -> bool {
    match source {
        CourseSource::Builtin { .. } => true,
        CourseSource::GitPack { url } => entries
            .iter()
            .any(|entry| matches_trust_key(entry, TrustSourceKind::GitRepo, url)),
        CourseSource::LocalMarkdown { path } | CourseSource::LocalPack { path } => entries
            .iter()
            .any(|entry| matches_trust_key(entry, TrustSourceKind::LocalPath, path)),
    }
}

fn request_from_source(source: &CourseSource) -> OpenCourseRequest {
    match source {
        CourseSource::Builtin { course_id } => OpenCourseRequest::Builtin {
            course_id: course_id.clone(),
        },
        CourseSource::LocalMarkdown { path } => {
            OpenCourseRequest::LocalMarkdown { path: path.clone() }
        }
        CourseSource::LocalPack { path } => OpenCourseRequest::LocalPack { path: path.clone() },
        CourseSource::GitPack { url } => OpenCourseRequest::GitPack { url: url.clone() },
    }
}

fn workspace_recent(workspace: &Workspace, course: &LoadedCourse) -> RecentWorkspace {
    RecentWorkspace {
        workspace_path: workspace.summary.root.clone(),
        course_id: course.course_id.clone(),
        course_title: course.title.clone(),
        language: course.language,
        course_source: Some(course.source.clone()),
        completed_exercises: workspace.completed_exercises(),
        total_exercises: course.total_exercises,
        last_opened_at: Utc::now().to_rfc3339(),
    }
}

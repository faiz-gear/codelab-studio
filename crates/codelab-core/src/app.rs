mod activation;
mod verification;
use crate::execution::verification::model::*;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak, mpsc};
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::course::authoring::{
    CourseDraft, CourseDraftRequest, Draft, DraftFilePath, DraftFileWrite, DraftPreview,
    DraftPreviewRequest, TrialWorkspaceOpened,
};
use crate::course::git_pack::{self, GitCli, normalize_git_url};
use crate::course::{
    self, ChapterReferences, CourseSource, CourseSourceChangedEvent, CourseSummary, CourseWatcher,
    GitFetchAdapter, LoadedCourse, OpenCourseRequest, Starter,
};
use crate::error::{DomainError, ErrorCode};
use crate::execution::preparation::{CommandPreparation, ConfirmationTarget};
use crate::execution::{Execution, model::*};
use crate::lsp::{
    LanguageDiagnostics, LanguageDocumentUpdate, LanguageFeatureRequest, LanguageFeatureResult,
    LanguageServers, LanguageStatusChanged, REQUEST_TIMEOUT,
};
use crate::model::{
    AppBootstrap, GlobalSettings, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE, RecentWorkspace,
    RecordRecentWorkspaceInput, ShutdownReport, StorageRecoveryNotice, StoreKind, TrustEntry,
    TrustSource, TrustSourceKind, TrustState, UpdateSettingsPatch,
};
use crate::runtime::{PreflightReport, RuntimeAdapters, RuntimeManager, RuntimePreference};
use crate::storage::JsonStore;
use crate::tutor::{
    AcpAgentPreset, AcpStdioBackend, ContextSnapshot, ContextSnapshotInput, ResolvedTurnContext,
    RespondTutorPermission, SendTutorPrompt, TutorBackend, TutorHandshakeRequest,
    TutorHandshakeResult, TutorManager, TutorSessionSnapshot, WorkspaceTutorRequest,
    context as tutor_context,
};
use crate::workspace::{
    LocalMetricEvent, LocalMetricRecord, MetricCompletionTrigger, MetricEventDetail,
    MetricEventName, MetricRiskDecision, MetricRiskSource, RecordLocalMetric, Workspace,
    WorkspaceComparison, WorkspaceEntry, WorkspaceFile, WorkspaceFileRename, WorkspaceFileWrite,
    WorkspaceFilesChanged, WorkspaceIdeRequest, WorkspaceMetrics, WorkspacePath, WorkspaceSummary,
    metrics::MAX_PENDING_METRICS,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentWorkspaceOpened {
    pub course: LoadedCourse,
    pub workspace: Option<WorkspaceSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseTrustChanged {
    pub source: CourseSource,
    pub trusted: bool,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum AppCommand {
    Bootstrap,
    GetSettings,
    UpdateSettings(UpdateSettingsPatch),
    ListRecents,
    RecordRecentWorkspace(RecordRecentWorkspaceInput),
    RemoveRecentWorkspace {
        #[serde(rename = "workspacePath")]
        workspace_path: String,
    },
    ListTrustEntries,
    CheckTrust(TrustSource),
    GrantTrust(TrustSource),
    RevokeTrust(TrustSource),
    ListBuiltinCourses,
    OpenCourse(OpenCourseRequest),
    ReloadActiveCourse,
    RefreshGitCourse,
    CreateCourseDraft(CourseDraftRequest),
    OpenCourseDraft {
        path: String,
    },
    ReadCourseDraftFile(DraftFilePath),
    WriteCourseDraftFile(DraftFileWrite),
    PreviewCourseDraft(DraftPreviewRequest),
    StartTrialWorkspace {
        #[serde(rename = "draftId")]
        draft_id: u64,
    },
    CloseActiveCourse,
    BindWorkspace {
        path: String,
    },
    OpenRecentWorkspace {
        path: String,
    },
    ListWorkspaceFiles(WorkspacePath),
    ReadWorkspaceFile(WorkspacePath),
    SyncLanguageDocument(LanguageDocumentUpdate),
    RequestLanguageFeature(LanguageFeatureRequest),
    WriteWorkspaceFile(WorkspaceFileWrite),
    PreviewChapterSolution {
        #[serde(rename = "workspaceId")]
        workspace_id: u64,
        #[serde(rename = "chapterId")]
        chapter_id: String,
    },
    ListStarterFiles {
        #[serde(rename = "workspaceId")]
        workspace_id: u64,
    },
    PreviewStarterFile(WorkspacePath),
    ApplyWorkspaceComparison(WorkspaceFileWrite),
    CreateWorkspaceFile(WorkspacePath),
    RenameWorkspaceFile(WorkspaceFileRename),
    OpenWorkspaceIde(WorkspaceIdeRequest),
    OpenTerminal(WorkspaceTerminalRequest),
    ReadTerminals(TerminalRead),
    WriteTerminal(TerminalInput),
    ResizeTerminal(TerminalSize),
    StopTerminal(TerminalTarget),
    CloseTerminal(TerminalTarget),
    GetCommandCwdOverrides(WorkspaceTerminalRequest),
    SetCommandCwdOverride(CommandCwdOverride),
    RunCommandBlock(RunCommandBlock),
    RunChecks(RunChecks),
    GetProgress(WorkspaceTerminalRequest),
    SetSelfCheck(SetSelfCheck),
    CompleteExercise(ExerciseTarget),
    ResetProgress(WorkspaceTerminalRequest),
    ExportLearningRecord(WorkspaceTerminalRequest),
    GetPreflight,
    RefreshPreflight,
    SetRuntimePreference(RuntimePreference),
    InstallManagedRuntime {
        language: crate::CourseLanguage,
    },
    TestTutorHandshake(TutorHandshakeRequest),
    GetTutorSession(WorkspaceTutorRequest),
    BuildTutorContext(ContextSnapshotInput),
    SendTutorPrompt(SendTutorPrompt),
    StopTutorTurn(WorkspaceTutorRequest),
    RetryTutorTurn(WorkspaceTutorRequest),
    RespondTutorPermission(RespondTutorPermission),
    ClearTutorSession(WorkspaceTutorRequest),
    GetWorkspaceMetrics(WorkspaceTerminalRequest),
    RecordLocalMetric(RecordLocalMetric),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum AppResponse {
    Bootstrap(AppBootstrap),
    Settings(GlobalSettings),
    Recents(Vec<RecentWorkspace>),
    TrustEntries(Vec<TrustEntry>),
    TrustState(TrustState),
    BuiltinCourses(Vec<CourseSummary>),
    LoadedCourse(LoadedCourse),
    CourseDraft(CourseDraft),
    DraftPreview(DraftPreview),
    TrialTrustRequired(TrustSource),
    TrialWorkspaceOpened(TrialWorkspaceOpened),
    CourseClosed,
    WorkspaceBound(WorkspaceSummary),
    RecentWorkspaceOpened(RecentWorkspaceOpened),
    WorkspaceEntries(Vec<WorkspaceEntry>),
    WorkspaceFile(WorkspaceFile),
    LanguageDocumentSynced,
    LanguageFeature(Option<LanguageFeatureResult>),
    WorkspaceFileConflict(WorkspaceFile),
    WorkspaceComparisons(Vec<WorkspaceComparison>),
    StarterFiles(Vec<String>),
    WorkspaceIdeOpened,
    TerminalOpened(TerminalSummary),
    Terminals(TerminalUpdates),
    TerminalUpdated,
    CommandCwdOverrides(CommandCwdOverrides),
    CommandConfirmationRequired(CommandConfirmation),
    CommandSaveConflict(CommandSaveConflict),
    CommandFailed(CommandFailed),
    CommandStarted(CommandStarted),
    VerificationStarted(VerificationStarted),
    Progress(LearningProgress),
    LearningRecord(Value),
    Preflight(PreflightReport),
    TutorHandshake(TutorHandshakeResult),
    TutorContext(ContextSnapshot),
    TutorSession(TutorSessionSnapshot),
    WorkspaceMetrics(WorkspaceMetrics),
    Shutdown(ShutdownReport),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum AppEvent {
    ProgressUpdated(LearningProgress),
    SettingsUpdated(GlobalSettings),
    RecentsUpdated(Vec<RecentWorkspace>),
    TrustUpdated(Vec<TrustEntry>),
    CourseLoaded(LoadedCourse),
    CourseSourceChanged(CourseSourceChangedEvent),
    CourseTrustChanged(CourseTrustChanged),
    WorkspaceFilesChanged(WorkspaceFilesChanged),
    LanguageStatusChanged(LanguageStatusChanged),
    LanguageDiagnostics(LanguageDiagnostics),
    PreflightUpdated(PreflightReport),
    TutorSessionUpdated(TutorSessionSnapshot),
    ShutdownCompleted(ShutdownReport),
}

pub struct EventSubscription {
    rx: mpsc::Receiver<AppEvent>,
}

impl EventSubscription {
    pub(crate) fn new(rx: mpsc::Receiver<AppEvent>) -> Self {
        Self { rx }
    }

    pub fn try_recv(&self) -> Result<AppEvent, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<AppEvent, mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }

    pub fn recv(&self) -> Result<AppEvent, mpsc::RecvError> {
        self.rx.recv()
    }
}

pub type CleanupHook = Box<dyn FnOnce() -> Result<(), String> + Send + 'static>;

struct ActiveCourseState {
    course: LoadedCourse,
    starter: Option<Starter>,
    references: ChapterReferences,
    _watcher: Option<CourseWatcher>,
}

struct AppState {
    settings: GlobalSettings,
    recents: Vec<RecentWorkspace>,
    trust_entries: Vec<TrustEntry>,
    recovery_notices: Vec<StorageRecoveryNotice>,
    builtin_courses: Vec<CourseSummary>,
    active_course: Option<ActiveCourseState>,
    course_load_generation: u64,
    active_workspace: Option<Workspace>,
    trial_workspace: Option<tempfile::TempDir>,
    draft: Option<Draft>,
    next_draft_id: u64,
    next_workspace_id: u64,
    execution: Execution,
    language_servers: LanguageServers,
    running_verification_chapter: Option<String>,
    tutor_generation: u64,
    pending_metrics: Vec<LocalMetricRecord>,
    shut_down: bool,
    cleanup_hooks: Vec<(String, CleanupHook)>,
}

type Subscribers = Mutex<Vec<mpsc::Sender<AppEvent>>>;

pub struct App {
    data_dir: PathBuf,
    settings_store: JsonStore<GlobalSettings>,
    recents_store: Arc<JsonStore<Vec<RecentWorkspace>>>,
    state: Arc<Mutex<AppState>>,
    verification: Arc<verification::VerificationController>,
    subscribers: Arc<Subscribers>,
    runtime: Arc<RuntimeManager>,
    tutor: Arc<TutorManager>,
    session: activation::SessionLifecycle,
}

impl App {
    pub fn open(config: AppConfig) -> Result<Self, DomainError> {
        Self::open_with_runtime_adapters(config, RuntimeAdapters::default())
    }

    pub fn open_with_runtime_adapters(
        config: AppConfig,
        adapters: RuntimeAdapters,
    ) -> Result<Self, DomainError> {
        Self::open_with_adapters(
            config,
            adapters,
            Arc::new(GitCli),
            Arc::new(AcpStdioBackend),
        )
    }

    pub fn open_with_git_fetcher(
        config: AppConfig,
        git: Arc<dyn GitFetchAdapter>,
    ) -> Result<Self, DomainError> {
        Self::open_with_adapters(
            config,
            RuntimeAdapters::default(),
            git,
            Arc::new(AcpStdioBackend),
        )
    }

    pub fn open_with_tutor_backend(
        config: AppConfig,
        tutor: Arc<dyn TutorBackend>,
    ) -> Result<Self, DomainError> {
        Self::open_with_adapters(config, RuntimeAdapters::default(), Arc::new(GitCli), tutor)
    }

    pub fn open_with_test_adapters(
        config: AppConfig,
        adapters: RuntimeAdapters,
        tutor: Arc<dyn TutorBackend>,
    ) -> Result<Self, DomainError> {
        Self::open_with_adapters(config, adapters, Arc::new(GitCli), tutor)
    }

    fn open_with_adapters(
        config: AppConfig,
        adapters: RuntimeAdapters,
        git: Arc<dyn GitFetchAdapter>,
        tutor_backend: Arc<dyn TutorBackend>,
    ) -> Result<Self, DomainError> {
        fs::create_dir_all(&config.data_dir).map_err(|err| {
            DomainError::new(ErrorCode::StorageWriteFailed)
                .with_detail("store", "dataDir")
                .with_detail("reason", err.to_string())
        })?;

        let data_dir = fs::canonicalize(&config.data_dir).map_err(|err| {
            DomainError::new(ErrorCode::StorageReadFailed)
                .with_detail("store", "dataDir")
                .with_detail("reason", err.to_string())
        })?;

        let settings_store = JsonStore::new(&data_dir, "settings.json", StoreKind::Settings);
        let recents_store = Arc::new(JsonStore::new(
            &data_dir,
            "recents.json",
            StoreKind::Recents,
        ));
        let trust_store = JsonStore::new(&data_dir, "trust.json", StoreKind::Trust);

        let settings_loaded = settings_store.load_or_recover(is_valid_settings)?;
        let recents_loaded = recents_store.load_or_recover(|entries: &Vec<RecentWorkspace>| {
            entries.iter().all(|item| {
                !item.workspace_path.trim().is_empty()
                    && !item.course_id.trim().is_empty()
                    && item.completed_exercises <= item.total_exercises
            })
        })?;
        let trust_loaded = trust_store.load_or_recover(|entries: &Vec<TrustEntry>| {
            entries
                .iter()
                .all(|item| !item.normalized_source.trim().is_empty())
        })?;

        let builtin_courses = course::list_builtin_courses()?;
        let subscribers = Arc::new(Subscribers::default());
        let metric_state: Arc<OnceLock<Weak<Mutex<AppState>>>> = Arc::new(OnceLock::new());
        let runtime_subscribers = Arc::downgrade(&subscribers);
        let runtime_metrics = Arc::clone(&metric_state);
        let (runtime, runtime_recovery) = RuntimeManager::open(
            &config.data_dir,
            adapters,
            move |report| {
                if let Some(subscribers) = runtime_subscribers.upgrade() {
                    emit_to_subscribers(&subscribers, AppEvent::PreflightUpdated(report));
                }
            },
            move |detail| {
                if let Some(state_lock) = runtime_metrics.get().and_then(Weak::upgrade) {
                    let mut state = state_lock
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner());
                    state.record_metric(detail);
                }
            },
        )?;

        let recovery_notices = [
            settings_loaded.recovery,
            recents_loaded.recovery,
            trust_loaded.recovery,
            runtime_recovery,
        ]
        .into_iter()
        .flatten()
        .collect();

        let tutor_subscribers = Arc::downgrade(&subscribers);
        let tutor_fs_subscribers = Arc::downgrade(&subscribers);
        let tutor_metrics = Arc::clone(&metric_state);
        let tutor = Arc::new(TutorManager::new(
            tutor_backend,
            settings_loaded.value.tutor.clone(),
            move |snapshot| {
                if let Some(subscribers) = tutor_subscribers.upgrade() {
                    emit_to_subscribers(&subscribers, AppEvent::TutorSessionUpdated(snapshot));
                }
            },
            move |changed| {
                if let Some(subscribers) = tutor_fs_subscribers.upgrade() {
                    emit_to_subscribers(&subscribers, AppEvent::WorkspaceFilesChanged(changed));
                }
            },
            move |workspace_id, detail| {
                if let Some(state_lock) = tutor_metrics.get().and_then(Weak::upgrade) {
                    let mut state = state_lock
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner());
                    if state
                        .active_workspace
                        .as_ref()
                        .is_some_and(|ws| ws.summary.id == workspace_id)
                    {
                        state.record_metric(detail);
                    }
                }
            },
        ));

        let verification = Arc::new(verification::VerificationController::default());
        let state = Arc::new(Mutex::new(AppState {
            settings: settings_loaded.value,
            recents: recents_loaded.value,
            trust_entries: trust_loaded.value,
            recovery_notices,
            builtin_courses,
            active_course: None,
            course_load_generation: 0,
            active_workspace: None,
            trial_workspace: None,
            draft: None,
            next_draft_id: 1,
            next_workspace_id: 1,
            execution: Execution::default(),
            language_servers: LanguageServers::default(),
            running_verification_chapter: None,
            tutor_generation: 0,
            pending_metrics: vec![LocalMetricRecord::now(MetricEventDetail::AppLaunched)],
            shut_down: false,
            cleanup_hooks: Vec::new(),
        }));
        let _ = metric_state.set(Arc::downgrade(&state));
        let session = activation::SessionLifecycle::new(activation::SessionLifecycleDeps {
            data_dir: data_dir.clone(),
            state: Arc::clone(&state),
            verification: Arc::clone(&verification),
            recents_store: Arc::clone(&recents_store),
            trust_store,
            subscribers: Arc::clone(&subscribers),
            tutor: Arc::clone(&tutor),
            git,
        });
        Ok(Self {
            data_dir,
            settings_store,
            recents_store,
            state,
            verification,
            subscribers,
            runtime,
            tutor,
            session,
        })
    }

    pub fn execute(&self, command: AppCommand) -> Result<AppResponse, DomainError> {
        let applying_comparison = matches!(command, AppCommand::ApplyWorkspaceComparison(_));
        match command {
            AppCommand::Bootstrap => {
                let state = self.lock_state();
                Ok(AppResponse::Bootstrap(AppBootstrap {
                    settings: state.settings.clone(),
                    recents: state.recents.clone(),
                    trust_entries: state.trust_entries.clone(),
                    recovery_notices: state.recovery_notices.clone(),
                    builtin_courses: state.builtin_courses.clone(),
                }))
            }
            AppCommand::GetSettings => {
                Ok(AppResponse::Settings(self.lock_state().settings.clone()))
            }
            AppCommand::UpdateSettings(patch) => self.update_settings(patch),
            AppCommand::ListRecents => Ok(AppResponse::Recents(self.lock_state().recents.clone())),
            AppCommand::RecordRecentWorkspace(input) => self.record_recent_workspace(input),
            AppCommand::RemoveRecentWorkspace { workspace_path } => {
                self.remove_recent_workspace(&workspace_path)
            }
            AppCommand::ListTrustEntries => Ok(AppResponse::TrustEntries(
                self.lock_state().trust_entries.clone(),
            )),
            AppCommand::CheckTrust(source) => self.check_trust(&source),
            AppCommand::GrantTrust(source) => self.grant_trust(&source),
            AppCommand::RevokeTrust(source) => self.revoke_trust(&source),
            AppCommand::ListBuiltinCourses => Ok(AppResponse::BuiltinCourses(
                self.lock_state().builtin_courses.clone(),
            )),
            AppCommand::OpenCourse(request) => self.session.open_course(request),
            AppCommand::ReloadActiveCourse => self.session.reload_active_course(),
            AppCommand::RefreshGitCourse => self.session.refresh_git_course(),
            AppCommand::CreateCourseDraft(request) => {
                self.session.open_draft(|id, data_dir, workspace| {
                    Draft::create(id, request, data_dir, workspace)
                })
            }
            AppCommand::OpenCourseDraft { path } => self
                .session
                .open_draft(|id, data_dir, workspace| Draft::open(id, &path, data_dir, workspace)),
            AppCommand::ReadCourseDraftFile(target) => self
                .lock_state()
                .draft(target.draft_id)?
                .paths
                .read(&target.path)
                .map(AppResponse::WorkspaceFile),
            AppCommand::WriteCourseDraftFile(request) => {
                let state = self.lock_state();
                let result = state.draft(request.target.draft_id)?.paths.write(
                    &request.target.path,
                    &request.text,
                    request.expected_revision.as_deref(),
                    false,
                )?;
                Ok(match result {
                    Ok(file) => AppResponse::WorkspaceFile(file),
                    Err(file) => AppResponse::WorkspaceFileConflict(file),
                })
            }
            AppCommand::PreviewCourseDraft(request) => self
                .lock_state()
                .draft(request.draft_id)?
                .preview(&request.edits)
                .map(AppResponse::DraftPreview),
            AppCommand::StartTrialWorkspace { draft_id } => {
                self.session.start_trial_workspace(draft_id)
            }
            AppCommand::CloseActiveCourse => Ok(self.session.close_active_course()),
            AppCommand::BindWorkspace { path } => self.session.bind_workspace(&path),
            AppCommand::OpenRecentWorkspace { path } => self.session.open_recent_workspace(&path),
            AppCommand::ListWorkspaceFiles(target) => self
                .lock_state()
                .workspace(target.workspace_id)?
                .files
                .list(&target.path)
                .map(AppResponse::WorkspaceEntries),
            AppCommand::ReadWorkspaceFile(target) => self
                .lock_state()
                .workspace(target.workspace_id)?
                .files
                .read(&target.path)
                .map(AppResponse::WorkspaceFile),
            AppCommand::SyncLanguageDocument(update) => {
                let mut state = self.lock_state();
                let files = Arc::clone(&state.workspace(update.workspace_id)?.files);
                let subscribers = Arc::downgrade(&self.subscribers);
                let emit: crate::lsp::Emit = Arc::new(move |event| {
                    if let Some(subscribers) = subscribers.upgrade() {
                        emit_to_subscribers(&subscribers, event);
                    }
                });
                state
                    .language_servers
                    .sync(&files, update, &self.runtime, &emit);
                Ok(AppResponse::LanguageDocumentSynced)
            }
            AppCommand::RequestLanguageFeature(request) => {
                let reply = {
                    let state = self.lock_state();
                    state.workspace(request.workspace_id)?;
                    state.language_servers.request(request)
                };
                let result = reply.and_then(|reply| reply.recv_timeout(REQUEST_TIMEOUT).ok());
                Ok(AppResponse::LanguageFeature(result))
            }
            AppCommand::PreviewChapterSolution {
                workspace_id,
                chapter_id,
            } => {
                let state = self.lock_state();
                let active = state.course()?;
                state
                    .workspace(workspace_id)?
                    .files
                    .preview_chapter(
                        &active.course,
                        active.starter.as_ref(),
                        &active.references,
                        &chapter_id,
                    )
                    .map(AppResponse::WorkspaceComparisons)
            }
            AppCommand::ListStarterFiles { workspace_id } => {
                let state = self.lock_state();
                let workspace = state.workspace(workspace_id)?;
                let starter = self.workspace_starter(workspace, state.course()?)?;
                workspace
                    .files
                    .list_starter(starter.as_ref())
                    .map(AppResponse::StarterFiles)
            }
            AppCommand::PreviewStarterFile(target) => {
                let state = self.lock_state();
                let workspace = state.workspace(target.workspace_id)?;
                let starter = self.workspace_starter(workspace, state.course()?)?;
                let comparison = workspace
                    .files
                    .preview_starter(starter.as_ref(), &target.path)?;
                Ok(AppResponse::WorkspaceComparisons(vec![comparison]))
            }
            AppCommand::WriteWorkspaceFile(request)
            | AppCommand::ApplyWorkspaceComparison(request) => {
                let state = self.lock_state();
                let result = state.workspace(request.target.workspace_id)?.write_file(
                    &request.target.path,
                    &request.text,
                    request.expected_revision.as_deref(),
                    applying_comparison,
                )?;
                Ok(match result {
                    Ok(file) => AppResponse::WorkspaceFile(file),
                    Err(file) => AppResponse::WorkspaceFileConflict(file),
                })
            }
            AppCommand::CreateWorkspaceFile(target) => self
                .lock_state()
                .workspace(target.workspace_id)?
                .create_file(&target.path)
                .map(AppResponse::WorkspaceFile),
            AppCommand::RenameWorkspaceFile(request) => self
                .lock_state()
                .workspace(request.target.workspace_id)?
                .rename_file(&request.target.path, &request.new_path)
                .map(AppResponse::WorkspaceFile),
            AppCommand::OpenWorkspaceIde(request) => {
                self.lock_state()
                    .workspace(request.target.workspace_id)?
                    .open_in_ide(&request)?;
                Ok(AppResponse::WorkspaceIdeOpened)
            }
            AppCommand::OpenTerminal(request) => self.open_terminal(request.workspace_id),
            AppCommand::RunCommandBlock(request) => self.run_command_block(request),
            AppCommand::RunChecks(request) => self.run_checks(request),
            AppCommand::GetProgress(request) => self.get_progress(request.workspace_id),
            AppCommand::SetSelfCheck(request) => self.set_self_check(request),
            AppCommand::CompleteExercise(request) => self.complete_exercise(request),
            AppCommand::ResetProgress(request) => self.reset_progress(request.workspace_id),
            AppCommand::ExportLearningRecord(request) => {
                self.export_learning_record(request.workspace_id)
            }
            AppCommand::ReadTerminals(request) => {
                let mut state = self.lock_state();
                state.workspace(request.workspace_id)?;
                let updates = state.execution.read(request);
                state.drain_finished_command_metrics();
                Ok(AppResponse::Terminals(updates))
            }
            AppCommand::WriteTerminal(request) => self
                .update_terminal(request.target.workspace_id, |execution| {
                    execution.input(request)
                }),
            AppCommand::ResizeTerminal(request) => self
                .update_terminal(request.target.workspace_id, |execution| {
                    execution.resize(request)
                }),
            AppCommand::StopTerminal(target) => self
                .update_terminal(target.workspace_id, |execution| {
                    execution.stop(target.terminal_id)
                }),
            AppCommand::CloseTerminal(target) => self
                .update_terminal(target.workspace_id, |execution| {
                    execution.close(target.terminal_id)
                }),
            AppCommand::GetCommandCwdOverrides(request) => {
                let state = self.lock_state();
                let workspace = state.workspace(request.workspace_id)?;
                Ok(AppResponse::CommandCwdOverrides(
                    workspace.command_cwds.for_course(&state.course()?.course),
                ))
            }
            AppCommand::SetCommandCwdOverride(request) => self.set_command_cwd(request),
            AppCommand::GetPreflight => Ok(AppResponse::Preflight(self.runtime.snapshot())),
            AppCommand::RefreshPreflight => self.runtime.refresh().map(AppResponse::Preflight),
            AppCommand::SetRuntimePreference(preference) => self
                .runtime
                .set_preference(preference)
                .map(AppResponse::Preflight),
            AppCommand::InstallManagedRuntime { language } => {
                self.runtime.install(language).map(AppResponse::Preflight)
            }
            AppCommand::TestTutorHandshake(request) => self.test_tutor_handshake(request),
            AppCommand::GetTutorSession(request) => self.get_tutor_session(request.workspace_id),
            AppCommand::BuildTutorContext(request) => self.build_tutor_context(request),
            AppCommand::SendTutorPrompt(request) => self.send_tutor_prompt(request),
            AppCommand::StopTutorTurn(request) => self.stop_tutor_turn(request.workspace_id),
            AppCommand::RetryTutorTurn(request) => self.retry_tutor_turn(request.workspace_id),
            AppCommand::RespondTutorPermission(request) => self.respond_tutor_permission(request),
            AppCommand::ClearTutorSession(request) => {
                self.clear_tutor_session(request.workspace_id)
            }
            AppCommand::GetWorkspaceMetrics(request) => {
                let state = self.lock_state();
                let workspace = state.workspace(request.workspace_id)?;
                Ok(AppResponse::WorkspaceMetrics(workspace.metrics_snapshot()))
            }
            AppCommand::RecordLocalMetric(request) => self.record_local_metric(request),
            AppCommand::Shutdown => self.shutdown().map(AppResponse::Shutdown),
        }
    }

    fn open_terminal(&self, workspace_id: u64) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let workspace = state.workspace(workspace_id)?;
        let dir = crate::workspace::command_cwd::directory(&workspace.files, ".")?;
        let language = state.course()?.course.language;
        state
            .execution
            .start(None, dir, &self.runtime.snapshot(), language)
            .map(AppResponse::TerminalOpened)
    }

    fn run_command_block(&self, request: RunCommandBlock) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let workspace = state.workspace(request.workspace_id)?;
        if state.running_verification_chapter.is_some() {
            return Err(DomainError::new(ErrorCode::VerificationBusy));
        }
        let course = &state.course()?.course;
        let language = course.language;
        let (launch, dir) = if let Some(id) = request.restart_terminal_id {
            let previous = state.execution.restart_launch(id, &request.command_id)?;
            workspace.command_cwds.resolve_saved(
                &workspace.files,
                &previous.block,
                previous.cwd_key.clone(),
                Some(id),
            )?
        } else {
            let block = find_block(course, &request.command_id)?;
            workspace
                .command_cwds
                .resolve(&workspace.files, course, block, None)?
        };
        let had_confirmation = request.confirmation.is_some();
        if !had_confirmation {
            let mode = launch.block.mode;
            let is_restart = request.restart_terminal_id.is_some();
            state.record_metric(MetricEventDetail::CmdBlockClicked { mode, is_restart });
        }
        let AppState {
            execution,
            active_workspace,
            ..
        } = &mut *state;
        match execution.prepare_command(
            active_workspace.as_ref().unwrap(),
            ConfirmationTarget::Block(Box::new(launch)),
            request.confirmation,
            request.drafts,
        ) {
            CommandPreparation::Ready {
                target: ConfirmationTarget::Block(launch),
                saved,
            } => {
                if had_confirmation {
                    state.record_metric(MetricEventDetail::CmdRiskModalAction {
                        source: MetricRiskSource::CommandBlock,
                        decision: MetricRiskDecision::Confirm,
                    });
                }
                let result =
                    state
                        .execution
                        .start(Some(*launch), dir, &self.runtime.snapshot(), language);
                state.drain_finished_command_metrics();
                match result {
                    Ok(terminal) => Ok(AppResponse::CommandStarted(CommandStarted {
                        terminal,
                        saved,
                    })),
                    Err(error) if saved.is_empty() => Err(error),
                    Err(error) => Ok(AppResponse::CommandFailed(CommandFailed { error, saved })),
                }
            }
            CommandPreparation::Ready {
                target: ConfirmationTarget::Checks { .. },
                ..
            } => unreachable!("block preparation target"),
            CommandPreparation::ConfirmationRequired(prompt) => {
                Ok(AppResponse::CommandConfirmationRequired(prompt))
            }
            CommandPreparation::SaveConflict(conflict) => {
                if had_confirmation {
                    state.record_metric(MetricEventDetail::CmdRiskModalAction {
                        source: MetricRiskSource::CommandBlock,
                        decision: MetricRiskDecision::Confirm,
                    });
                }
                Ok(AppResponse::CommandSaveConflict(conflict))
            }
            CommandPreparation::Failed(failed) if failed.saved.is_empty() => Err(failed.error),
            CommandPreparation::Failed(failed) => Ok(AppResponse::CommandFailed(failed)),
        }
    }

    fn set_command_cwd(&self, request: CommandCwdOverride) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        state.workspace(request.workspace_id)?;
        let AppState {
            active_course,
            active_workspace,
            execution,
            ..
        } = &mut *state;
        let course = &active_course.as_ref().unwrap().course;
        let workspace = active_workspace.as_mut().unwrap();
        let block = find_block(course, &request.command_id)?;
        workspace
            .command_cwds
            .set(&workspace.files, course, block, request.cwd.as_deref())?;
        execution.invalidate_confirmation();
        Ok(AppResponse::CommandCwdOverrides(
            workspace.command_cwds.for_course(course),
        ))
    }

    fn update_terminal(
        &self,
        id: u64,
        update: impl FnOnce(&mut Execution) -> Result<(), DomainError>,
    ) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        state.workspace(id)?;
        update(&mut state.execution)?;
        state.drain_finished_command_metrics();
        Ok(AppResponse::TerminalUpdated)
    }

    fn record_local_metric(&self, request: RecordLocalMetric) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        state.workspace(request.workspace_id)?;
        if matches!(
            request.event,
            LocalMetricEvent::CmdRiskModalCancelled { .. }
        ) {
            state.execution.invalidate_confirmation();
        }
        let detail = request.event.into_detail();
        let workspace = state.active_workspace.as_mut().unwrap();
        workspace.record_metric(detail)?;
        Ok(AppResponse::WorkspaceMetrics(workspace.metrics_snapshot()))
    }

    pub fn subscribe(&self) -> EventSubscription {
        let (tx, rx) = mpsc::channel();
        let mut subs = self
            .subscribers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        subs.push(tx);
        EventSubscription::new(rx)
    }

    pub fn register_cleanup_hook(&self, name: impl Into<String>, hook: CleanupHook) {
        let already_shut_down = {
            let mut state = self.lock_state();
            if state.shut_down {
                true
            } else {
                state.cleanup_hooks.push((name.into(), hook));
                return;
            }
        };
        if already_shut_down {
            let _ = hook();
        }
    }

    pub fn shutdown(&self) -> Result<ShutdownReport, DomainError> {
        self.session.shutdown()
    }

    fn update_settings(&self, patch: UpdateSettingsPatch) -> Result<AppResponse, DomainError> {
        if let Some(size) = patch.code_font_size
            && !is_valid_code_font_size(size)
        {
            return Err(DomainError::new(ErrorCode::InvalidCodeFontSize)
                .with_detail("min", MIN_CODE_FONT_SIZE.to_string())
                .with_detail("max", MAX_CODE_FONT_SIZE.to_string()));
        }

        let updated = {
            let mut state = self.lock_state();
            let mut next = state.settings.clone();
            if let Some(theme) = patch.theme {
                next.theme = theme;
            }
            if let Some(accent) = patch.accent {
                next.accent = accent;
            }
            if let Some(size) = patch.code_font_size {
                next.code_font_size = size;
            }
            if let Some(locale) = patch.locale {
                next.locale = locale;
            }
            if let Some(preferred_ide) = patch.preferred_ide {
                next.preferred_ide = preferred_ide;
            }
            let prev_style = state.settings.tutor.teaching_style;
            if let Some(tutor_patch) = patch.tutor {
                if let Some(preset) = tutor_patch.preset {
                    next.tutor.preset = preset;
                    if tutor_patch.command.is_none() && preset != AcpAgentPreset::Custom {
                        next.tutor.command = preset.default_command().to_owned();
                    }
                }
                if let Some(command) = tutor_patch.command {
                    next.tutor.command = command.trim().to_owned();
                }
                if let Some(permission_policy) = tutor_patch.permission_policy {
                    next.tutor.permission_policy = permission_policy;
                }
                if let Some(teaching_style) = tutor_patch.teaching_style {
                    next.tutor.teaching_style = teaching_style;
                }
            }
            if !next.tutor.is_valid() {
                return Err(DomainError::new(ErrorCode::AcpCommandInvalid));
            }
            self.settings_store.save_atomic(&next)?;
            state.settings = next.clone();
            if next.tutor.teaching_style != prev_style {
                state.record_metric(MetricEventDetail::AiModeSwitched {
                    from: prev_style,
                    to: next.tutor.teaching_style,
                });
            }
            next
        };

        self.tutor.sync_settings(updated.tutor.clone());
        self.emit(AppEvent::SettingsUpdated(updated.clone()));
        Ok(AppResponse::Settings(updated))
    }

    fn test_tutor_handshake(
        &self,
        request: TutorHandshakeRequest,
    ) -> Result<AppResponse, DomainError> {
        let cwd = {
            let state = self.lock_state();
            if state.shut_down {
                return Err(DomainError::new(ErrorCode::AcpHandshakeFailed));
            }
            if state
                .active_course
                .as_ref()
                .is_some_and(|active| !active.course.trusted)
            {
                return Err(DomainError::new(ErrorCode::CourseUntrusted));
            }
            state
                .active_workspace
                .as_ref()
                .map(|ws| ws.files.root.clone())
                .unwrap_or_else(|| self.data_dir.clone())
        };
        self.tutor
            .handshake(request, cwd)
            .map(AppResponse::TutorHandshake)
    }

    fn get_tutor_session(&self, workspace_id: u64) -> Result<AppResponse, DomainError> {
        {
            let state = self.lock_state();
            state.workspace(workspace_id)?;
        }
        self.tutor
            .snapshot(workspace_id)
            .map(AppResponse::TutorSession)
    }

    fn build_snapshot_locked(
        state: &AppState,
        request: &ContextSnapshotInput,
    ) -> Result<ContextSnapshot, DomainError> {
        let workspace = state.workspace(request.workspace_id)?;
        let course = &state.trusted_course()?.course;
        let recent_errors = state.execution.recent_errors();
        let default_style = state.settings.tutor.teaching_style;
        Ok(tutor_context::build_context_snapshot(
            course,
            workspace,
            &recent_errors,
            request,
            default_style,
        ))
    }

    fn resolve_turn_context_locked(
        state: &AppState,
        mut input: ContextSnapshotInput,
        workspace_id: u64,
        prompt: &str,
        teaching_style: Option<crate::TeachingStyle>,
        fallback_preamble: Option<String>,
    ) -> Result<(ContextSnapshotInput, ResolvedTurnContext), DomainError> {
        input.workspace_id = workspace_id;
        input.prompt = Some(prompt.to_owned());
        input.teaching_style = Some(teaching_style.unwrap_or(state.settings.tutor.teaching_style));
        let snapshot = Self::build_snapshot_locked(state, &input)?;
        let preamble = if snapshot.rendered_preamble.trim().is_empty() {
            fallback_preamble
        } else {
            Some(snapshot.rendered_preamble)
        };
        let dimensions = snapshot.chips.iter().map(|c| c.dimension).collect();
        Ok((
            input,
            ResolvedTurnContext {
                preamble,
                quote: snapshot.quote,
                dimensions,
            },
        ))
    }

    fn build_tutor_context(
        &self,
        request: ContextSnapshotInput,
    ) -> Result<AppResponse, DomainError> {
        let state = self.lock_state();
        Self::build_snapshot_locked(&state, &request).map(AppResponse::TutorContext)
    }

    fn send_tutor_prompt(&self, mut request: SendTutorPrompt) -> Result<AppResponse, DomainError> {
        let resolved_context = {
            let state = self.lock_state();
            state.workspace(request.workspace_id)?;
            if let Some(ctx_input) = request.context.take() {
                let (normalized, resolved) = Self::resolve_turn_context_locked(
                    &state,
                    ctx_input,
                    request.workspace_id,
                    &request.prompt,
                    request.teaching_style,
                    request.context_preamble.clone(),
                )?;
                request.context = Some(normalized);
                Some(resolved)
            } else {
                None
            }
        };
        self.tutor
            .send_prompt(request, resolved_context)
            .map(AppResponse::TutorSession)
    }

    fn stop_tutor_turn(&self, workspace_id: u64) -> Result<AppResponse, DomainError> {
        {
            let state = self.lock_state();
            state.workspace(workspace_id)?;
        }
        self.tutor
            .stop_turn(workspace_id)
            .map(AppResponse::TutorSession)
    }

    fn retry_tutor_turn(&self, workspace_id: u64) -> Result<AppResponse, DomainError> {
        let (prompt, style, seed_context) = self.tutor.retry_turn_seed(workspace_id)?;
        let refreshed_context = {
            let state = self.lock_state();
            state.workspace(workspace_id)?;
            match seed_context {
                Some(ctx_input) => Some(Self::resolve_turn_context_locked(
                    &state,
                    ctx_input,
                    workspace_id,
                    &prompt,
                    Some(style),
                    None,
                )?),
                None => None,
            }
        };
        self.tutor
            .retry_turn(workspace_id, refreshed_context)
            .map(AppResponse::TutorSession)
    }

    fn respond_tutor_permission(
        &self,
        request: RespondTutorPermission,
    ) -> Result<AppResponse, DomainError> {
        {
            let state = self.lock_state();
            state.workspace(request.workspace_id)?;
        }
        self.tutor
            .respond_permission(request)
            .map(AppResponse::TutorSession)
    }

    fn clear_tutor_session(&self, workspace_id: u64) -> Result<AppResponse, DomainError> {
        {
            let state = self.lock_state();
            state.workspace(workspace_id)?;
        }
        self.tutor
            .clear_session(workspace_id)
            .map(AppResponse::TutorSession)
    }

    fn record_recent_workspace(
        &self,
        input: RecordRecentWorkspaceInput,
    ) -> Result<AppResponse, DomainError> {
        let normalized_path = normalize_absolute_path(&input.workspace_path)
            .ok_or_else(|| DomainError::new(ErrorCode::InvalidWorkspacePath))?;
        let course_id = input.course_id.trim();
        let course_title = input.course_title.trim();
        if course_id.is_empty() || course_title.is_empty() {
            return Err(DomainError::new(ErrorCode::InvalidCourseMetadata));
        }
        if input.completed_exercises > input.total_exercises {
            return Err(DomainError::new(ErrorCode::InvalidProgressCount)
                .with_detail("completed", input.completed_exercises.to_string())
                .with_detail("total", input.total_exercises.to_string()));
        }

        let entry = RecentWorkspace {
            workspace_path: normalized_path,
            course_id: course_id.to_string(),
            course_title: course_title.to_string(),
            language: Some(input.language),
            course_source: None,
            completed_exercises: input.completed_exercises,
            total_exercises: input.total_exercises,
            last_opened_at: Utc::now().to_rfc3339(),
        };

        let (updated, event) = {
            let mut state = self.lock_state();
            stage_recent(&mut state, &self.recents_store, entry)?
        };
        self.emit(event);
        Ok(AppResponse::Recents(updated))
    }

    fn remove_recent_workspace(&self, workspace_path: &str) -> Result<AppResponse, DomainError> {
        let normalized_path = normalize_absolute_path(workspace_path)
            .ok_or_else(|| DomainError::new(ErrorCode::InvalidWorkspacePath))?;
        let (updated, event) = {
            let mut state = self.lock_state();
            stage_recents_mutation(&mut state, &self.recents_store, |next| {
                next.retain(|item| item.workspace_path != normalized_path)
            })?
        };
        self.emit(event);
        Ok(AppResponse::Recents(updated))
    }

    fn check_trust(&self, source: &TrustSource) -> Result<AppResponse, DomainError> {
        let normalized_source = normalize_trust_source(source.source_kind, &source.source)
            .ok_or_else(|| DomainError::new(ErrorCode::InvalidTrustSource))?;
        let state = self.lock_state();
        let matched = state
            .trust_entries
            .iter()
            .find(|entry| matches_trust_key(entry, source.source_kind, &normalized_source));
        Ok(AppResponse::TrustState(TrustState {
            source_kind: source.source_kind,
            normalized_source,
            trusted: matched.is_some(),
            trusted_at: matched.map(|entry| entry.trusted_at.clone()),
        }))
    }

    fn grant_trust(&self, source: &TrustSource) -> Result<AppResponse, DomainError> {
        let normalized_source = normalize_trust_source(source.source_kind, &source.source)
            .ok_or_else(|| DomainError::new(ErrorCode::InvalidTrustSource))?;
        let trusted_at = Utc::now().to_rfc3339();
        let status = TrustState {
            source_kind: source.source_kind,
            normalized_source: normalized_source.clone(),
            trusted: true,
            trusted_at: Some(trusted_at.clone()),
        };

        self.session.mutate_trust(|next| {
            next.retain(|entry| !matches_trust_key(entry, source.source_kind, &normalized_source));
            next.insert(
                0,
                TrustEntry {
                    source_kind: source.source_kind,
                    normalized_source,
                    trusted_at,
                },
            );
        })?;

        Ok(AppResponse::TrustState(status))
    }

    fn revoke_trust(&self, source: &TrustSource) -> Result<AppResponse, DomainError> {
        let normalized_source = normalize_trust_source(source.source_kind, &source.source)
            .ok_or_else(|| DomainError::new(ErrorCode::InvalidTrustSource))?;
        let updated = self.session.mutate_trust(|next| {
            let exact = source.source.trim();
            // The active Course's canonical key can now be an alias to a different source.
            let key = if source.source_kind == TrustSourceKind::LocalPath
                && next
                    .iter()
                    .any(|entry| matches_trust_key(entry, source.source_kind, exact))
            {
                exact
            } else {
                &normalized_source
            };
            next.retain(|entry| !matches_trust_key(entry, source.source_kind, key));
        })?;
        Ok(AppResponse::TrustEntries(updated))
    }

    fn workspace_starter(
        &self,
        workspace: &Workspace,
        active: &ActiveCourseState,
    ) -> Result<Option<Starter>, DomainError> {
        if let (CourseSource::GitPack { url }, Some(key)) = (
            &active.course.source,
            workspace.git_starter_snapshot.as_deref(),
        ) && git_pack::snapshot_key(active.starter.as_ref()) != Some(key)
        {
            return git_pack::snapshot_starter(&self.data_dir, url, key);
        }
        Ok(active.starter.clone())
    }

    fn lock_state(&self) -> MutexGuard<'_, AppState> {
        lock_state(&self.state)
    }

    fn emit(&self, event: AppEvent) {
        emit_to_subscribers(&self.subscribers, event);
    }
}

fn lock_state(state: &Mutex<AppState>) -> MutexGuard<'_, AppState> {
    state.lock().unwrap_or_else(|poison| poison.into_inner())
}

fn stage_recent(
    state: &mut AppState,
    recents_store: &JsonStore<Vec<RecentWorkspace>>,
    mut entry: RecentWorkspace,
) -> Result<(Vec<RecentWorkspace>, AppEvent), DomainError> {
    if entry.course_source.is_none() {
        entry.course_source = state
            .recents
            .iter()
            .find(|recent| {
                recent.workspace_path == entry.workspace_path && recent.course_id == entry.course_id
            })
            .and_then(|recent| recent.course_source.clone());
    }
    stage_recents_mutation(state, recents_store, |next| {
        next.retain(|item| item.workspace_path != entry.workspace_path);
        next.insert(0, entry);
        next.truncate(MAX_RECENT_WORKSPACES);
    })
}

fn stage_recents_mutation(
    state: &mut AppState,
    recents_store: &JsonStore<Vec<RecentWorkspace>>,
    mutate: impl FnOnce(&mut Vec<RecentWorkspace>),
) -> Result<(Vec<RecentWorkspace>, AppEvent), DomainError> {
    let mut next = state.recents.clone();
    mutate(&mut next);
    recents_store.save_atomic(&next)?;
    state.recents = next.clone();
    let event = AppEvent::RecentsUpdated(next.clone());
    Ok((next, event))
}

impl AppState {
    fn record_metric(&mut self, detail: MetricEventDetail) {
        let trusted_active = self
            .active_course
            .as_ref()
            .is_some_and(|active| active.course.trusted);
        if trusted_active && let Some(workspace) = self.active_workspace.as_mut() {
            let _ = workspace.record_metric(detail);
            return;
        }
        if detail.event_name() == MetricEventName::CourseOpened {
            self.pending_metrics
                .retain(|record| record.event != MetricEventName::CourseOpened);
        }
        self.pending_metrics.push(LocalMetricRecord::now(detail));
        if self.pending_metrics.len() > MAX_PENDING_METRICS {
            let overflow = self.pending_metrics.len() - MAX_PENDING_METRICS;
            self.pending_metrics.drain(..overflow);
        }
    }

    fn drain_finished_command_metrics(&mut self) {
        let finished = self.execution.take_finished_metrics();
        if finished.is_empty() {
            return;
        }
        for detail in finished {
            self.record_metric(detail);
        }
    }

    fn stop_workspace_activity(&mut self) {
        debug_assert!(self.running_verification_chapter.is_none());
        self.execution.clear();
        self.language_servers.suspend();
    }

    fn deactivate_workspace(&mut self) {
        self.stop_workspace_activity();
        self.active_workspace = None;
    }

    fn close_trial(&mut self) -> bool {
        if self.trial_workspace.is_some() {
            self.deactivate_workspace();
            self.trial_workspace = None;
            true
        } else {
            false
        }
    }

    fn draft(&self, id: u64) -> Result<&Draft, DomainError> {
        self.draft
            .as_ref()
            .filter(|draft| draft.id == id && !self.shut_down)
            .ok_or_else(|| DomainError::new(ErrorCode::NoActiveDraft))
    }

    fn protect_draft_path(&self, path: &str) -> Result<(), DomainError> {
        if let Some(draft) = &self.draft
            && let Ok(root) = fs::canonicalize(path)
            && (root.starts_with(&draft.paths.root) || draft.paths.root.starts_with(root))
        {
            return Err(DomainError::new(ErrorCode::InvalidWorkspacePath));
        }
        Ok(())
    }

    fn course(&self) -> Result<&ActiveCourseState, DomainError> {
        self.active_course
            .as_ref()
            .ok_or_else(|| DomainError::new(ErrorCode::NoActiveCourse))
    }

    fn trusted_course(&self) -> Result<&ActiveCourseState, DomainError> {
        let active = self.course()?;
        if !active.course.trusted {
            return Err(DomainError::new(ErrorCode::CourseUntrusted));
        }
        Ok(active)
    }

    fn workspace(&self, id: u64) -> Result<&Workspace, DomainError> {
        self.trusted_course()?;
        self.active_workspace
            .as_ref()
            .filter(|workspace| workspace.summary.id == id)
            .ok_or_else(|| DomainError::new(ErrorCode::NoActiveWorkspace))
    }
}

fn find_block<'a>(
    course: &'a LoadedCourse,
    id: &str,
) -> Result<&'a crate::CommandBlock, DomainError> {
    course
        .chapters
        .iter()
        .flat_map(|chapter| &chapter.commands)
        .find(|block| block.id == id)
        .ok_or_else(|| DomainError::new(ErrorCode::CommandBlockNotFound))
}

fn emit_to_subscribers(subscribers: &Subscribers, event: AppEvent) {
    let mut subs = subscribers
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    subs.retain(|tx| tx.send(event.clone()).is_ok());
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

const MAX_RECENT_WORKSPACES: usize = 20;

fn matches_trust_key(entry: &TrustEntry, kind: TrustSourceKind, normalized: &str) -> bool {
    entry.source_kind == kind && entry.normalized_source == normalized
}

fn is_valid_code_font_size(size: f32) -> bool {
    size.is_finite() && (MIN_CODE_FONT_SIZE..=MAX_CODE_FONT_SIZE).contains(&size)
}

fn is_valid_settings(settings: &GlobalSettings) -> bool {
    is_valid_code_font_size(settings.code_font_size) && settings.tutor.is_valid()
}

fn normalize_absolute_path(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = Path::new(trimmed);
    if !path.is_absolute() {
        return None;
    }
    if let Ok(canonical) = fs::canonicalize(path) {
        return Some(canonical.to_string_lossy().into_owned());
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    if normalized.as_os_str().is_empty() {
        return None;
    }
    Some(normalized.to_string_lossy().into_owned())
}

fn normalize_trust_source(kind: TrustSourceKind, raw: &str) -> Option<String> {
    match kind {
        TrustSourceKind::LocalPath => normalize_absolute_path(raw),
        TrustSourceKind::GitRepo => normalize_git_url(raw),
    }
}

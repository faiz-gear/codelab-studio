use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::course::authoring::{
    CourseDraft, CourseDraftRequest, Draft, DraftFilePath, DraftFileWrite, DraftPreview,
    DraftPreviewRequest, TrialWorkspaceOpened,
};
use crate::course::git_pack::{self, GitCli, normalize_git_url};
use crate::course::{
    self, ChapterReferences, CourseSource, CourseSourceChangedEvent, CourseSummary, CourseWatcher,
    GitFetchAdapter, LoadedCourse, OpenCourseRequest, OpenedCourse, Starter,
};
use crate::error::{DomainError, ErrorCode};
use crate::model::{
    AppBootstrap, GlobalSettings, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE, RecentWorkspace,
    RecordRecentWorkspaceInput, ShutdownReport, StorageRecoveryNotice, StoreKind, TrustEntry,
    TrustSource, TrustSourceKind, TrustState, UpdateSettingsPatch,
};
use crate::runtime::{PreflightReport, RuntimeAdapters, RuntimeManager, RuntimePreference};
use crate::storage::JsonStore;
use crate::workspace::{
    Workspace, WorkspaceComparison, WorkspaceEntry, WorkspaceFile, WorkspaceFileRename,
    WorkspaceFileWrite, WorkspaceFilesChanged, WorkspaceIdeRequest, WorkspacePath,
    WorkspaceSummary,
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
    GetPreflight,
    RefreshPreflight,
    SetRuntimePreference(RuntimePreference),
    InstallManagedRuntime {
        language: crate::CourseLanguage,
    },
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
    TrialWorkspaceOpened(TrialWorkspaceOpened),
    CourseClosed,
    WorkspaceBound(WorkspaceSummary),
    RecentWorkspaceOpened(RecentWorkspaceOpened),
    WorkspaceEntries(Vec<WorkspaceEntry>),
    WorkspaceFile(WorkspaceFile),
    WorkspaceFileConflict(WorkspaceFile),
    WorkspaceComparisons(Vec<WorkspaceComparison>),
    StarterFiles(Vec<String>),
    WorkspaceIdeOpened,
    Preflight(PreflightReport),
    Shutdown(ShutdownReport),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum AppEvent {
    SettingsUpdated(GlobalSettings),
    RecentsUpdated(Vec<RecentWorkspace>),
    TrustUpdated(Vec<TrustEntry>),
    CourseLoaded(LoadedCourse),
    CourseSourceChanged(CourseSourceChangedEvent),
    CourseTrustChanged(CourseTrustChanged),
    WorkspaceFilesChanged(WorkspaceFilesChanged),
    PreflightUpdated(PreflightReport),
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
    active_workspace: Option<Workspace>,
    trial_workspace: Option<tempfile::TempDir>,
    draft: Option<Draft>,
    next_draft_id: u64,
    next_workspace_id: u64,
    shut_down: bool,
    cleanup_hooks: Vec<(String, CleanupHook)>,
}

type Subscribers = Mutex<Vec<mpsc::Sender<AppEvent>>>;

pub struct App {
    data_dir: PathBuf,
    settings_store: JsonStore<GlobalSettings>,
    recents_store: JsonStore<Vec<RecentWorkspace>>,
    trust_store: JsonStore<Vec<TrustEntry>>,
    state: Mutex<AppState>,
    subscribers: Arc<Subscribers>,
    runtime: Arc<RuntimeManager>,
    git: Arc<dyn GitFetchAdapter>,
}

impl App {
    pub fn open(config: AppConfig) -> Result<Self, DomainError> {
        Self::open_with_runtime_adapters(config, RuntimeAdapters::default())
    }

    pub fn open_with_runtime_adapters(
        config: AppConfig,
        adapters: RuntimeAdapters,
    ) -> Result<Self, DomainError> {
        Self::open_with_adapters(config, adapters, Arc::new(GitCli))
    }

    pub fn open_with_git_fetcher(
        config: AppConfig,
        git: Arc<dyn GitFetchAdapter>,
    ) -> Result<Self, DomainError> {
        Self::open_with_adapters(config, RuntimeAdapters::default(), git)
    }

    fn open_with_adapters(
        config: AppConfig,
        adapters: RuntimeAdapters,
        git: Arc<dyn GitFetchAdapter>,
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
        let recents_store = JsonStore::new(&data_dir, "recents.json", StoreKind::Recents);
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
        let runtime_subscribers = Arc::downgrade(&subscribers);
        let (runtime, runtime_recovery) =
            RuntimeManager::open(&config.data_dir, adapters, move |report| {
                if let Some(subscribers) = runtime_subscribers.upgrade() {
                    emit_to_subscribers(&subscribers, AppEvent::PreflightUpdated(report));
                }
            })?;

        let recovery_notices = [
            settings_loaded.recovery,
            recents_loaded.recovery,
            trust_loaded.recovery,
            runtime_recovery,
        ]
        .into_iter()
        .flatten()
        .collect();

        Ok(Self {
            data_dir,
            settings_store,
            recents_store,
            trust_store,
            state: Mutex::new(AppState {
                settings: settings_loaded.value,
                recents: recents_loaded.value,
                trust_entries: trust_loaded.value,
                recovery_notices,
                builtin_courses,
                active_course: None,
                active_workspace: None,
                trial_workspace: None,
                draft: None,
                next_draft_id: 1,
                next_workspace_id: 1,
                shut_down: false,
                cleanup_hooks: Vec::new(),
            }),
            subscribers,
            runtime,
            git,
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
            AppCommand::OpenCourse(request) => self.open_course(request),
            AppCommand::ReloadActiveCourse => self.reload_active_course(),
            AppCommand::RefreshGitCourse => self.refresh_git_course(),
            AppCommand::CreateCourseDraft(request) => self
                .open_draft(|id, workspace| Draft::create(id, request, &self.data_dir, workspace)),
            AppCommand::OpenCourseDraft { path } => {
                self.open_draft(|id, workspace| Draft::open(id, &path, &self.data_dir, workspace))
            }
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
            AppCommand::StartTrialWorkspace { draft_id } => self.start_trial_workspace(draft_id),
            AppCommand::CloseActiveCourse => Ok(self.close_active_course()),
            AppCommand::BindWorkspace { path } => self.bind_workspace(&path),
            AppCommand::OpenRecentWorkspace { path } => self.open_recent_workspace(&path),
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
                state
                    .workspace(workspace_id)?
                    .files
                    .list_starter(state.course()?.starter.as_ref())
                    .map(AppResponse::StarterFiles)
            }
            AppCommand::PreviewStarterFile(target) => {
                let state = self.lock_state();
                let comparison = state
                    .workspace(target.workspace_id)?
                    .files
                    .preview_starter(state.course()?.starter.as_ref(), &target.path)?;
                Ok(AppResponse::WorkspaceComparisons(vec![comparison]))
            }
            AppCommand::WriteWorkspaceFile(request)
            | AppCommand::ApplyWorkspaceComparison(request) => {
                let state = self.lock_state();
                let result = state.workspace(request.target.workspace_id)?.files.write(
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
                .files
                .create(&target.path)
                .map(AppResponse::WorkspaceFile),
            AppCommand::RenameWorkspaceFile(request) => self
                .lock_state()
                .workspace(request.target.workspace_id)?
                .files
                .rename(&request.target.path, &request.new_path)
                .map(AppResponse::WorkspaceFile),
            AppCommand::OpenWorkspaceIde(request) => {
                self.lock_state()
                    .workspace(request.target.workspace_id)?
                    .open_in_ide(&request)?;
                Ok(AppResponse::WorkspaceIdeOpened)
            }
            AppCommand::GetPreflight => Ok(AppResponse::Preflight(self.runtime.snapshot())),
            AppCommand::RefreshPreflight => self.runtime.refresh().map(AppResponse::Preflight),
            AppCommand::SetRuntimePreference(preference) => self
                .runtime
                .set_preference(preference)
                .map(AppResponse::Preflight),
            AppCommand::InstallManagedRuntime { language } => {
                self.runtime.install(language).map(AppResponse::Preflight)
            }
            AppCommand::Shutdown => self.shutdown().map(AppResponse::Shutdown),
        }
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
        let (hooks, active_course, active_workspace, trial_workspace) = {
            let mut state = self.lock_state();
            if state.shut_down {
                return Ok(ShutdownReport {
                    already_shut_down: true,
                    hooks_executed: 0,
                });
            }
            state.shut_down = true;
            (
                std::mem::take(&mut state.cleanup_hooks),
                state.active_course.take(),
                state.active_workspace.take(),
                state.trial_workspace.take(),
            )
        };

        drop(active_workspace);
        drop(active_course);
        drop(trial_workspace);

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
        self.emit(AppEvent::ShutdownCompleted(report.clone()));

        if !failures.is_empty() {
            return Err(DomainError::new(ErrorCode::ShutdownFailed)
                .with_detail("failures", failures.join("; ")));
        }

        Ok(report)
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
            self.settings_store.save_atomic(&next)?;
            state.settings = next.clone();
            next
        };

        self.emit(AppEvent::SettingsUpdated(updated.clone()));
        Ok(AppResponse::Settings(updated))
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

        let mut state = self.lock_state();
        let updated = self.store_recent(&mut state, entry)?;
        Ok(AppResponse::Recents(updated))
    }

    fn remove_recent_workspace(&self, workspace_path: &str) -> Result<AppResponse, DomainError> {
        let normalized_path = normalize_absolute_path(workspace_path)
            .ok_or_else(|| DomainError::new(ErrorCode::InvalidWorkspacePath))?;
        let mut state = self.lock_state();
        self.mutate_recents(&mut state, |next| {
            next.retain(|item| item.workspace_path != normalized_path)
        })
        .map(AppResponse::Recents)
    }

    fn store_recent(
        &self,
        state: &mut AppState,
        mut entry: RecentWorkspace,
    ) -> Result<Vec<RecentWorkspace>, DomainError> {
        if entry.course_source.is_none() {
            entry.course_source = state
                .recents
                .iter()
                .find(|recent| {
                    recent.workspace_path == entry.workspace_path
                        && recent.course_id == entry.course_id
                })
                .and_then(|recent| recent.course_source.clone());
        }
        self.mutate_recents(state, |next| {
            next.retain(|item| item.workspace_path != entry.workspace_path);
            next.insert(0, entry);
            next.truncate(MAX_RECENT_WORKSPACES);
        })
    }

    fn mutate_recents(
        &self,
        state: &mut AppState,
        mutate: impl FnOnce(&mut Vec<RecentWorkspace>),
    ) -> Result<Vec<RecentWorkspace>, DomainError> {
        let mut next = state.recents.clone();
        mutate(&mut next);
        self.recents_store.save_atomic(&next)?;
        state.recents = next.clone();
        self.emit(AppEvent::RecentsUpdated(next.clone()));
        Ok(next)
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

        self.mutate_trust(|next| {
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
        let updated = self.mutate_trust(|next| {
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

    fn mutate_trust(
        &self,
        mutate: impl FnOnce(&mut Vec<TrustEntry>),
    ) -> Result<Vec<TrustEntry>, DomainError> {
        let mut state = self.lock_state();
        let mut next = state.trust_entries.clone();
        mutate(&mut next);
        self.trust_store.save_atomic(&next)?;
        state.trust_entries = next.clone();

        let trial_active = state.trial_workspace.is_some();
        let changed = state.active_course.as_mut().and_then(|active| {
            if trial_active {
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
        if let Some(changed) = changed {
            if let Some(workspace) = &state.active_workspace {
                workspace.set_trusted(changed.trusted);
            }
            self.emit(AppEvent::CourseTrustChanged(changed));
        }
        self.emit(AppEvent::TrustUpdated(next.clone()));
        Ok(next)
    }

    fn open_course(&self, request: OpenCourseRequest) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        self.load_course(&mut state, &request)
    }

    fn reload_active_course(&self) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let active = state.course()?;
        if state.trial_workspace.is_some() {
            return Ok(AppResponse::LoadedCourse(active.course.clone()));
        }
        let request = request_from_source(&active.course.source);
        self.load_course(&mut state, &request)
    }

    fn refresh_git_course(&self) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let CourseSource::GitPack { url } = &state.course()?.course.source else {
            return Err(DomainError::new(ErrorCode::NoActiveGitCourse));
        };
        let mut opened = git_pack::load(&self.data_dir, url, self.git.as_ref(), true)?;
        opened.course.trusted = is_course_trusted(&opened.course.source, &state.trust_entries);
        if let Some(workspace) = &state.active_workspace {
            let mut recent = workspace_recent(workspace, &opened.course);
            recent.completed_exercises = workspace.completed_for(&opened.course)?;
            self.store_recent(&mut state, recent)?;
        }
        Ok(AppResponse::LoadedCourse(
            self.activate_course(&mut state, opened),
        ))
    }

    fn load_course(
        &self,
        state: &mut AppState,
        request: &OpenCourseRequest,
    ) -> Result<AppResponse, DomainError> {
        let opened = self.prepare_course(state, request)?;
        state.close_trial();
        if state
            .active_course
            .as_ref()
            .is_some_and(|active| active.course.source != opened.course.source)
        {
            state.active_workspace = None;
        }
        Ok(AppResponse::LoadedCourse(
            self.activate_course(state, opened),
        ))
    }

    fn prepare_course(
        &self,
        state: &AppState,
        request: &OpenCourseRequest,
    ) -> Result<OpenedCourse, DomainError> {
        if state.shut_down {
            return Err(DomainError::new(ErrorCode::NoActiveCourse));
        }
        let mut opened = course::open_course(request, &self.data_dir, self.git.as_ref())?;
        opened.course.trusted = is_course_trusted(&opened.course.source, &state.trust_entries);
        Ok(opened)
    }

    fn activate_course(&self, state: &mut AppState, opened: OpenedCourse) -> LoadedCourse {
        let watcher = if opened.watched_files.is_empty() {
            None
        } else {
            let subs = Arc::clone(&self.subscribers);
            Some(CourseWatcher::start(
                opened.course.course_id.clone(),
                opened.watched_files,
                move |changed| emit_to_subscribers(&subs, AppEvent::CourseSourceChanged(changed)),
            ))
        };
        let course = opened.course;
        state.active_course = Some(ActiveCourseState {
            course: course.clone(),
            starter: opened.starter,
            references: opened.references,
            _watcher: watcher,
        });
        self.emit(AppEvent::CourseLoaded(course.clone()));
        course
    }

    fn close_active_course(&self) -> AppResponse {
        let mut state = self.lock_state();
        state.active_workspace = None;
        state.trial_workspace = None;
        state.active_course = None;
        AppResponse::CourseClosed
    }

    fn bind_workspace(&self, path: &str) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let active = state.trusted_course()?;
        if state.trial_workspace.is_some() {
            return Err(DomainError::new(ErrorCode::InvalidWorkspacePath));
        }
        state.protect_draft_path(path)?;
        let workspace = self.prepare_workspace(
            state.next_workspace_id,
            path,
            &active.course,
            active.starter.as_ref(),
        )?;
        let recent = workspace_recent(&workspace, &active.course);
        self.store_recent(&mut state, recent)?;
        let summary = self.activate_workspace(&mut state, workspace);
        Ok(AppResponse::WorkspaceBound(summary))
    }

    fn open_recent_workspace(&self, path: &str) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let normalized = normalize_absolute_path(path)
            .ok_or_else(|| DomainError::new(ErrorCode::InvalidWorkspacePath))?;
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
        let opened = self.prepare_course(&state, &request)?;
        let workspace = if opened.course.trusted {
            state.protect_draft_path(path)?;
            let workspace = self.prepare_workspace(
                state.next_workspace_id,
                path,
                &opened.course,
                opened.starter.as_ref(),
            )?;
            self.store_recent(&mut state, workspace_recent(&workspace, &opened.course))?;
            Some(self.activate_workspace(&mut state, workspace))
        } else {
            state.active_workspace = None;
            None
        };
        state.trial_workspace = None;
        let course = self.activate_course(&mut state, opened);
        Ok(AppResponse::RecentWorkspaceOpened(RecentWorkspaceOpened {
            course,
            workspace,
        }))
    }

    fn open_draft(
        &self,
        open: impl FnOnce(u64, Option<&Path>) -> Result<Draft, DomainError>,
    ) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        if state.shut_down {
            return Err(DomainError::new(ErrorCode::NoActiveDraft));
        }
        let workspace = state
            .active_workspace
            .as_ref()
            .map(|workspace| workspace.files.root.as_path());
        let draft = open(state.next_draft_id, workspace)?;
        let summary = draft.summary()?;
        state.next_draft_id += 1;
        state.close_trial();
        state.active_course = None;
        state.active_workspace = None;
        state.draft = Some(draft);
        Ok(AppResponse::CourseDraft(summary))
    }

    fn start_trial_workspace(&self, draft_id: u64) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let opened = state.draft(draft_id)?.trial_course()?;
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
        let workspace = self.activate_workspace(&mut state, workspace);
        state.trial_workspace = Some(trial);
        let course = self.activate_course(&mut state, opened);
        Ok(AppResponse::TrialWorkspaceOpened(TrialWorkspaceOpened {
            course,
            workspace,
        }))
    }

    fn prepare_workspace(
        &self,
        id: u64,
        path: &str,
        course: &LoadedCourse,
        starter: Option<&Starter>,
    ) -> Result<Workspace, DomainError> {
        let subs = Arc::clone(&self.subscribers);
        Workspace::bind(id, path, &self.data_dir, course, starter, move |event| {
            emit_to_subscribers(&subs, AppEvent::WorkspaceFilesChanged(event));
        })
    }

    fn activate_workspace(&self, state: &mut AppState, workspace: Workspace) -> WorkspaceSummary {
        state.next_workspace_id += 1;
        let summary = workspace.summary.clone();
        state.active_workspace = Some(workspace);
        summary
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, AppState> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn emit(&self, event: AppEvent) {
        emit_to_subscribers(&self.subscribers, event);
    }
}

impl AppState {
    fn close_trial(&mut self) {
        if self.trial_workspace.is_some() {
            self.active_workspace = None;
            self.trial_workspace = None;
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
        completed_exercises: workspace.completed_exercises,
        total_exercises: course.total_exercises,
        last_opened_at: Utc::now().to_rfc3339(),
    }
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
    is_valid_code_font_size(settings.code_font_size)
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

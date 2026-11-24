use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::course::{
    self, CourseSource, CourseSourceChangedEvent, CourseSummary, CourseWatcher, LoadedCourse,
    OpenCourseRequest,
};
use crate::error::{DomainError, ErrorCode};
use crate::model::{
    AppBootstrap, GlobalSettings, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE, RecentWorkspace,
    RecordRecentWorkspaceInput, ShutdownReport, StorageRecoveryNotice, StoreKind, TrustEntry,
    TrustSource, TrustSourceKind, TrustState, UpdateSettingsPatch,
};
use crate::storage::JsonStore;

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
    CloseActiveCourse,
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
    CourseClosed,
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
    source: CourseSource,
    watcher: Option<CourseWatcher>,
}

struct AppState {
    settings: GlobalSettings,
    recents: Vec<RecentWorkspace>,
    trust_entries: Vec<TrustEntry>,
    recovery_notices: Vec<StorageRecoveryNotice>,
    builtin_courses: Vec<CourseSummary>,
    active_course: Option<ActiveCourseState>,
    shut_down: bool,
    cleanup_hooks: Vec<(String, CleanupHook)>,
}

pub struct App {
    settings_store: JsonStore<GlobalSettings>,
    recents_store: JsonStore<Vec<RecentWorkspace>>,
    trust_store: JsonStore<Vec<TrustEntry>>,
    state: Mutex<AppState>,
    subscribers: Arc<Mutex<Vec<mpsc::Sender<AppEvent>>>>,
}

impl App {
    pub fn open(config: AppConfig) -> Result<Self, DomainError> {
        fs::create_dir_all(&config.data_dir).map_err(|err| {
            DomainError::new(ErrorCode::StorageWriteFailed)
                .with_detail("store", "dataDir")
                .with_detail("reason", err.to_string())
        })?;

        let settings_store = JsonStore::new(&config.data_dir, "settings.json", StoreKind::Settings);
        let recents_store = JsonStore::new(&config.data_dir, "recents.json", StoreKind::Recents);
        let trust_store = JsonStore::new(&config.data_dir, "trust.json", StoreKind::Trust);

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

        let recovery_notices = [
            settings_loaded.recovery,
            recents_loaded.recovery,
            trust_loaded.recovery,
        ]
        .into_iter()
        .flatten()
        .collect();

        let builtin_courses = course::list_builtin_courses()?;

        Ok(Self {
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
                shut_down: false,
                cleanup_hooks: Vec::new(),
            }),
            subscribers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn execute(&self, command: AppCommand) -> Result<AppResponse, DomainError> {
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
            AppCommand::CloseActiveCourse => self.close_active_course(),
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
        let (hooks, active_course) = {
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
            )
        };

        if let Some(active) = active_course
            && let Some(watcher) = active.watcher
        {
            watcher.stop();
        }

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
            language: input.language,
            completed_exercises: input.completed_exercises,
            total_exercises: input.total_exercises,
            last_opened_at: Utc::now().to_rfc3339(),
        };

        self.mutate_recents(|next| {
            next.retain(|item| item.workspace_path != entry.workspace_path);
            next.insert(0, entry);
            next.truncate(MAX_RECENT_WORKSPACES);
        })
    }

    fn remove_recent_workspace(&self, workspace_path: &str) -> Result<AppResponse, DomainError> {
        let normalized_path = normalize_absolute_path(workspace_path)
            .ok_or_else(|| DomainError::new(ErrorCode::InvalidWorkspacePath))?;
        self.mutate_recents(|next| {
            next.retain(|item| item.workspace_path != normalized_path);
        })
    }

    fn mutate_recents(
        &self,
        mutate: impl FnOnce(&mut Vec<RecentWorkspace>),
    ) -> Result<AppResponse, DomainError> {
        let updated = {
            let mut state = self.lock_state();
            let mut next = state.recents.clone();
            mutate(&mut next);
            self.recents_store.save_atomic(&next)?;
            state.recents = next.clone();
            next
        };
        self.emit(AppEvent::RecentsUpdated(updated.clone()));
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
            next.retain(|entry| !matches_trust_key(entry, source.source_kind, &normalized_source));
        })?;
        Ok(AppResponse::TrustEntries(updated))
    }

    fn mutate_trust(
        &self,
        mutate: impl FnOnce(&mut Vec<TrustEntry>),
    ) -> Result<Vec<TrustEntry>, DomainError> {
        let updated = {
            let mut state = self.lock_state();
            let mut next = state.trust_entries.clone();
            mutate(&mut next);
            self.trust_store.save_atomic(&next)?;
            state.trust_entries = next.clone();
            next
        };
        self.emit(AppEvent::TrustUpdated(updated.clone()));
        Ok(updated)
    }

    fn open_course(&self, request: OpenCourseRequest) -> Result<AppResponse, DomainError> {
        let opened = course::open_course(&request)?;
        let watcher = if opened.watched_files.is_empty() {
            None
        } else {
            let subs = Arc::clone(&self.subscribers);
            Some(CourseWatcher::start(
                opened.course.course_id.clone(),
                opened.watched_files,
                move |changed| {
                    emit_to_subscribers(&subs, AppEvent::CourseSourceChanged(changed));
                },
            ))
        };

        let previous = {
            let mut state = self.lock_state();
            state.active_course.replace(ActiveCourseState {
                source: opened.course.source.clone(),
                watcher,
            })
        };
        if let Some(prev) = previous
            && let Some(prev_watcher) = prev.watcher
        {
            prev_watcher.stop();
        }

        self.emit(AppEvent::CourseLoaded(opened.course.clone()));
        Ok(AppResponse::LoadedCourse(opened.course))
    }

    fn reload_active_course(&self) -> Result<AppResponse, DomainError> {
        let source = {
            let state = self.lock_state();
            state
                .active_course
                .as_ref()
                .map(|active| active.source.clone())
                .ok_or_else(|| DomainError::new(ErrorCode::NoActiveCourse))?
        };
        let request = match source {
            CourseSource::Builtin { course_id } => OpenCourseRequest::Builtin { course_id },
            CourseSource::LocalMarkdown { path } => OpenCourseRequest::LocalMarkdown { path },
            CourseSource::LocalPack { path } => OpenCourseRequest::LocalPack { path },
        };
        self.open_course(request)
    }

    fn close_active_course(&self) -> Result<AppResponse, DomainError> {
        let previous = {
            let mut state = self.lock_state();
            state.active_course.take()
        };
        if let Some(prev) = previous
            && let Some(watcher) = prev.watcher
        {
            watcher.stop();
        }
        Ok(AppResponse::CourseClosed)
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

fn emit_to_subscribers(subscribers: &Mutex<Vec<mpsc::Sender<AppEvent>>>, event: AppEvent) {
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

fn normalize_git_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
        return None;
    }

    if let Some((scheme, rest)) = trimmed.split_once("://") {
        let lower_scheme = scheme.to_ascii_lowercase();
        if !matches!(lower_scheme.as_str(), "https" | "http" | "ssh") {
            return None;
        }
        let (host, path) = split_and_clean_git_host_path(rest, '/')?;
        return Some(format!("{lower_scheme}://{host}/{path}"));
    }

    if let Some(after_user) = trimmed.strip_prefix("git@") {
        let (host, path) = split_and_clean_git_host_path(after_user, ':')?;
        return Some(format!("git@{host}:{path}"));
    }

    None
}

fn split_and_clean_git_host_path(input: &str, separator: char) -> Option<(String, String)> {
    let (host, raw_path) = input.split_once(separator)?;
    let lower_host = host.trim().to_ascii_lowercase();
    let trimmed_path = raw_path.trim_matches('/');
    let clean_path = trimmed_path
        .strip_suffix(".git")
        .unwrap_or(trimmed_path)
        .trim_end_matches('/');
    if lower_host.is_empty() || clean_path.is_empty() {
        return None;
    }
    Some((lower_host, clean_path.to_string()))
}

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::{DomainError, ErrorCode};
use crate::model::{
    AppBootstrap, GlobalSettings, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE, RecentWorkspace,
    RecordRecentWorkspaceInput, ShutdownReport, StorageRecoveryNotice, StoreKind, TrustEntry,
    TrustSourceKind, TrustStatus, UpdateSettingsPatch,
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
    CheckTrust {
        #[serde(rename = "sourceKind")]
        source_kind: TrustSourceKind,
        source: String,
    },
    GrantTrust {
        #[serde(rename = "sourceKind")]
        source_kind: TrustSourceKind,
        source: String,
    },
    RevokeTrust {
        #[serde(rename = "sourceKind")]
        source_kind: TrustSourceKind,
        source: String,
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
    TrustStatus(TrustStatus),
    Shutdown(ShutdownReport),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum AppEvent {
    SettingsUpdated(GlobalSettings),
    RecentsUpdated(Vec<RecentWorkspace>),
    TrustUpdated(Vec<TrustEntry>),
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

struct AppState {
    settings: GlobalSettings,
    recents: Vec<RecentWorkspace>,
    trust_entries: Vec<TrustEntry>,
    recovery_notices: Vec<StorageRecoveryNotice>,
    shut_down: bool,
    cleanup_hooks: Vec<(String, CleanupHook)>,
}

pub struct App {
    settings_store: JsonStore<GlobalSettings>,
    recents_store: JsonStore<Vec<RecentWorkspace>>,
    trust_store: JsonStore<Vec<TrustEntry>>,
    state: Mutex<AppState>,
    subscribers: Mutex<Vec<mpsc::Sender<AppEvent>>>,
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

        let mut recovery_notices = Vec::new();
        if let Some(notice) = settings_loaded.recovery {
            recovery_notices.push(notice);
        }
        if let Some(notice) = recents_loaded.recovery {
            recovery_notices.push(notice);
        }
        if let Some(notice) = trust_loaded.recovery {
            recovery_notices.push(notice);
        }

        Ok(Self {
            settings_store,
            recents_store,
            trust_store,
            state: Mutex::new(AppState {
                settings: settings_loaded.value,
                recents: recents_loaded.value,
                trust_entries: trust_loaded.value,
                recovery_notices,
                shut_down: false,
                cleanup_hooks: Vec::new(),
            }),
            subscribers: Mutex::new(Vec::new()),
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
                }))
            }
            AppCommand::GetSettings => {
                let state = self.lock_state();
                Ok(AppResponse::Settings(state.settings.clone()))
            }
            AppCommand::UpdateSettings(patch) => {
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
            AppCommand::ListRecents => {
                let state = self.lock_state();
                Ok(AppResponse::Recents(state.recents.clone()))
            }
            AppCommand::RecordRecentWorkspace(input) => {
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

                let updated = {
                    let mut state = self.lock_state();
                    let mut next = state.recents.clone();
                    next.retain(|item| item.workspace_path != entry.workspace_path);
                    next.insert(0, entry);
                    next.truncate(MAX_RECENT_WORKSPACES);
                    self.recents_store.save_atomic(&next)?;
                    state.recents = next.clone();
                    next
                };

                self.emit(AppEvent::RecentsUpdated(updated.clone()));
                Ok(AppResponse::Recents(updated))
            }
            AppCommand::RemoveRecentWorkspace { workspace_path } => {
                let normalized_path = normalize_absolute_path(&workspace_path)
                    .ok_or_else(|| DomainError::new(ErrorCode::InvalidWorkspacePath))?;

                let updated = {
                    let mut state = self.lock_state();
                    let mut next = state.recents.clone();
                    next.retain(|item| item.workspace_path != normalized_path);
                    self.recents_store.save_atomic(&next)?;
                    state.recents = next.clone();
                    next
                };

                self.emit(AppEvent::RecentsUpdated(updated.clone()));
                Ok(AppResponse::Recents(updated))
            }
            AppCommand::ListTrustEntries => {
                let state = self.lock_state();
                Ok(AppResponse::TrustEntries(state.trust_entries.clone()))
            }
            AppCommand::CheckTrust {
                source_kind,
                source,
            } => {
                let normalized_source = normalize_trust_source(source_kind, &source)
                    .ok_or_else(|| DomainError::new(ErrorCode::InvalidTrustSource))?;
                let state = self.lock_state();
                let matched = state.trust_entries.iter().find(|entry| {
                    entry.source_kind == source_kind && entry.normalized_source == normalized_source
                });
                Ok(AppResponse::TrustStatus(TrustStatus {
                    source_kind,
                    normalized_source,
                    trusted: matched.is_some(),
                    trusted_at: matched.map(|entry| entry.trusted_at.clone()),
                }))
            }
            AppCommand::GrantTrust {
                source_kind,
                source,
            } => {
                let normalized_source = normalize_trust_source(source_kind, &source)
                    .ok_or_else(|| DomainError::new(ErrorCode::InvalidTrustSource))?;
                let trusted_at = Utc::now().to_rfc3339();
                let (updated, status) = {
                    let mut state = self.lock_state();
                    let mut next = state.trust_entries.clone();
                    next.retain(|entry| {
                        !(entry.source_kind == source_kind
                            && entry.normalized_source == normalized_source)
                    });
                    next.insert(
                        0,
                        TrustEntry {
                            source_kind,
                            normalized_source: normalized_source.clone(),
                            trusted_at: trusted_at.clone(),
                        },
                    );
                    self.trust_store.save_atomic(&next)?;
                    state.trust_entries = next.clone();
                    (
                        next,
                        TrustStatus {
                            source_kind,
                            normalized_source,
                            trusted: true,
                            trusted_at: Some(trusted_at),
                        },
                    )
                };
                self.emit(AppEvent::TrustUpdated(updated));
                Ok(AppResponse::TrustStatus(status))
            }
            AppCommand::RevokeTrust {
                source_kind,
                source,
            } => {
                let normalized_source = normalize_trust_source(source_kind, &source)
                    .ok_or_else(|| DomainError::new(ErrorCode::InvalidTrustSource))?;
                let updated = {
                    let mut state = self.lock_state();
                    let mut next = state.trust_entries.clone();
                    next.retain(|entry| {
                        !(entry.source_kind == source_kind
                            && entry.normalized_source == normalized_source)
                    });
                    self.trust_store.save_atomic(&next)?;
                    state.trust_entries = next.clone();
                    next
                };
                self.emit(AppEvent::TrustUpdated(updated.clone()));
                Ok(AppResponse::TrustEntries(updated))
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
        let hooks = {
            let mut state = self.lock_state();
            if state.shut_down {
                return Ok(ShutdownReport {
                    already_shut_down: true,
                    hooks_executed: 0,
                });
            }
            state.shut_down = true;
            std::mem::take(&mut state.cleanup_hooks)
        };

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

    fn lock_state(&self) -> std::sync::MutexGuard<'_, AppState> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn emit(&self, event: AppEvent) {
        let mut subs = self
            .subscribers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        subs.retain(|tx| tx.send(event.clone()).is_ok());
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

const MAX_RECENT_WORKSPACES: usize = 20;

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
        let (host, path) = rest.split_once('/')?;
        let lower_host = host.trim().to_ascii_lowercase();
        let clean_path = strip_git_suffix(path.trim_matches('/'));
        if lower_host.is_empty() || clean_path.is_empty() {
            return None;
        }
        return Some(format!("{lower_scheme}://{lower_host}/{clean_path}"));
    }

    if let Some(after_user) = trimmed.strip_prefix("git@") {
        let (host, path) = after_user.split_once(':')?;
        let lower_host = host.trim().to_ascii_lowercase();
        let clean_path = strip_git_suffix(path.trim_matches('/'));
        if lower_host.is_empty() || clean_path.is_empty() {
            return None;
        }
        return Some(format!("git@{lower_host}:{clean_path}"));
    }

    None
}

fn strip_git_suffix(path: &str) -> &str {
    path.trim_end_matches('/')
        .strip_suffix(".git")
        .unwrap_or(path.trim_end_matches('/'))
        .trim_end_matches('/')
}

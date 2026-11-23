use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::Instant;

use chrono::Utc;

use crate::course::{list_builtin_courses, resolve_default_builtin_courses_dir};
use crate::domain::error::{AppError, ErrorCode};
use crate::domain::ids::SessionEpoch;
use crate::domain::metrics::MetricEvent;
use crate::domain::workspace::{
    BootstrapState, ChapterStatus, GlobalConfigPatch, GlobalConfigView, Locale, RecentWorkspaceItem,
};
use crate::platform::CodelabHomePaths;
use crate::session::workspace_session::{
    SessionOpenParams, SessionShutdownReport, WorkspaceSession,
};
use crate::storage::GlobalStore;

pub const PENDING_METRICS_CAPACITY: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionLifecycleKind {
    Idle,
    TrustPending {
        source_key: String,
        fingerprint: String,
    },
    Opening {
        workspace_root: PathBuf,
    },
    Active {
        epoch: SessionEpoch,
        workspace_root: PathBuf,
    },
    Closing {
        epoch: SessionEpoch,
    },
}

#[derive(Debug)]
enum SessionSlot {
    Idle,
    TrustPending {
        source_key: String,
        fingerprint: String,
    },
    Opening {
        workspace_root: PathBuf,
    },
    Active(Arc<WorkspaceSession>),
    Closing {
        epoch: SessionEpoch,
    },
}

pub struct AppState {
    started_at: Instant,
    global_store: GlobalStore,
    builtin_courses_dir: RwLock<Option<PathBuf>>,
    fallback_locale: RwLock<Locale>,
    current_epoch: AtomicU64,
    app_launched_recorded: AtomicBool,
    shutting_down: AtomicBool,
    pending_metrics: Mutex<VecDeque<MetricEvent>>,
    lifecycle_lock: tokio::sync::Mutex<()>,
    slot: RwLock<SessionSlot>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("current_epoch", &self.current_epoch())
            .field("lifecycle", &self.lifecycle_state())
            .finish()
    }
}

impl AppState {
    pub fn try_new() -> Result<Self, AppError> {
        let home_paths = CodelabHomePaths::resolve_default().map_err(|err| {
            AppError::new(ErrorCode::PathOutsideSandbox, "error.unknown")
                .with_param("detail", err.to_string())
                .with_detail(err.to_string())
        })?;
        let state = Self::with_home_paths(home_paths);
        state.set_builtin_courses_dir(resolve_default_builtin_courses_dir(None));
        Ok(state)
    }

    pub fn with_home_paths(home_paths: CodelabHomePaths) -> Self {
        Self {
            started_at: Instant::now(),
            global_store: GlobalStore::new(home_paths),
            builtin_courses_dir: RwLock::new(None),
            fallback_locale: RwLock::new(Locale::EnUs),
            current_epoch: AtomicU64::new(0),
            app_launched_recorded: AtomicBool::new(false),
            shutting_down: AtomicBool::new(false),
            pending_metrics: Mutex::new(VecDeque::with_capacity(PENDING_METRICS_CAPACITY)),
            lifecycle_lock: tokio::sync::Mutex::new(()),
            slot: RwLock::new(SessionSlot::Idle),
        }
    }

    pub fn global_store(&self) -> &GlobalStore {
        &self.global_store
    }

    pub fn set_builtin_courses_dir(&self, dir: Option<PathBuf>) {
        let mut guard = self
            .builtin_courses_dir
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        *guard = dir;
    }

    pub fn builtin_courses_dir(&self) -> Option<PathBuf> {
        self.builtin_courses_dir
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn elapsed_startup_ms(&self) -> u128 {
        self.started_at.elapsed().as_millis()
    }

    pub fn current_epoch(&self) -> SessionEpoch {
        SessionEpoch(self.current_epoch.load(Ordering::Acquire))
    }

    pub fn lifecycle_state(&self) -> SessionLifecycleKind {
        let slot = self.slot.read().unwrap_or_else(PoisonError::into_inner);
        match &*slot {
            SessionSlot::Idle => SessionLifecycleKind::Idle,
            SessionSlot::TrustPending {
                source_key,
                fingerprint,
            } => SessionLifecycleKind::TrustPending {
                source_key: source_key.clone(),
                fingerprint: fingerprint.clone(),
            },
            SessionSlot::Opening { workspace_root } => SessionLifecycleKind::Opening {
                workspace_root: workspace_root.clone(),
            },
            SessionSlot::Active(session) => SessionLifecycleKind::Active {
                epoch: session.epoch(),
                workspace_root: session.paths().root().to_path_buf(),
            },
            SessionSlot::Closing { epoch } => SessionLifecycleKind::Closing { epoch: *epoch },
        }
    }

    pub fn push_pending_metric(&self, event: MetricEvent) {
        let mut queue = self
            .pending_metrics
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if queue.len() >= PENDING_METRICS_CAPACITY {
            queue.pop_front();
        }
        queue.push_back(event);
    }

    pub fn drain_pending_metrics(&self) -> Vec<MetricEvent> {
        let mut queue = self
            .pending_metrics
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        queue.drain(..).collect()
    }

    pub fn build_bootstrap_state(
        &self,
        detected_locale: Locale,
    ) -> Result<BootstrapState, AppError> {
        {
            let mut fallback = self
                .fallback_locale
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            *fallback = detected_locale;
        }

        let config_file = self
            .global_store
            .load_or_init_config(detected_locale)
            .map_err(AppError::from)?
            .into_value();

        if !self.app_launched_recorded.swap(true, Ordering::AcqRel) {
            self.push_pending_metric(MetricEvent::AppLaunched {
                locale: config_file.locale,
            });
        }

        let config = config_file.to_view();
        let builtin_dir = self.builtin_courses_dir();
        let builtin_courses = list_builtin_courses(builtin_dir.as_deref());
        Ok(BootstrapState {
            epoch: self.current_epoch(),
            config,
            recent_workspaces: config_file.recent_workspaces,
            builtin_courses,
        })
    }

    pub fn update_settings(&self, patch: GlobalConfigPatch) -> Result<GlobalConfigView, AppError> {
        let fallback = *self
            .fallback_locale
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        let (view, _corruption) = self
            .global_store
            .update_config(fallback, patch)
            .map_err(AppError::from)?;
        Ok(view)
    }

    pub async fn begin_trust_pending(
        &self,
        source_key: impl Into<String>,
        fingerprint: impl Into<String>,
    ) -> Result<(), AppError> {
        let _op = self.lifecycle_lock.lock().await;
        let mut slot = self.slot.write().unwrap_or_else(PoisonError::into_inner);
        match &*slot {
            SessionSlot::Idle | SessionSlot::TrustPending { .. } => {
                *slot = SessionSlot::TrustPending {
                    source_key: source_key.into(),
                    fingerprint: fingerprint.into(),
                };
                Ok(())
            }
            SessionSlot::Active(session) => Err(AppError::new(
                ErrorCode::SessionEpochStale,
                "error.notReady",
            )
            .with_detail(format!(
                "cannot enter TrustPending while epoch {} is active",
                session.epoch().0
            ))),
            SessionSlot::Opening { .. } | SessionSlot::Closing { .. } => Err(AppError::new(
                ErrorCode::SessionEpochStale,
                "error.notReady",
            )
            .with_detail("session transition already in progress")),
        }
    }

    pub async fn open_workspace(
        &self,
        params: SessionOpenParams,
    ) -> Result<Arc<WorkspaceSession>, AppError> {
        if self.shutting_down.load(Ordering::Acquire) {
            return Err(AppError::new(ErrorCode::Cancelled, "error.notReady")
                .with_detail("application is shutting down"));
        }
        let _op = self.lifecycle_lock.lock().await;
        if self.shutting_down.load(Ordering::Acquire) {
            return Err(AppError::new(ErrorCode::Cancelled, "error.notReady")
                .with_detail("application is shutting down"));
        }

        let previous = {
            let mut slot = self.slot.write().unwrap_or_else(PoisonError::into_inner);
            match std::mem::replace(&mut *slot, SessionSlot::Idle) {
                SessionSlot::Active(prev) => {
                    *slot = SessionSlot::Closing {
                        epoch: prev.epoch(),
                    };
                    Some(prev)
                }
                SessionSlot::Idle | SessionSlot::TrustPending { .. } => None,
                other => {
                    *slot = other;
                    return Err(
                        AppError::new(ErrorCode::SessionEpochStale, "error.notReady")
                            .with_detail("workspace transition already in progress"),
                    );
                }
            }
        };

        if let Some(prev) = previous {
            let _ = prev.drain_and_close().await;
        }

        {
            let mut slot = self.slot.write().unwrap_or_else(PoisonError::into_inner);
            *slot = SessionSlot::Opening {
                workspace_root: params.workspace_root.clone(),
            };
        }

        let next_epoch = SessionEpoch(self.current_epoch.load(Ordering::Acquire) + 1);
        let course_id = params.init_spec.course_id.clone();
        let course_title = params.course_title.clone();
        let language = params.language;
        let total_chapters = params.total_chapters;

        let session =
            match tokio::task::spawn_blocking(move || WorkspaceSession::open(next_epoch, params))
                .await
            {
                Ok(Ok(session)) => Arc::new(session),
                Ok(Err(err)) => {
                    let mut slot = self.slot.write().unwrap_or_else(PoisonError::into_inner);
                    *slot = SessionSlot::Idle;
                    return Err(err);
                }
                Err(join_err) => {
                    let mut slot = self.slot.write().unwrap_or_else(PoisonError::into_inner);
                    *slot = SessionSlot::Idle;
                    return Err(AppError::new(ErrorCode::Cancelled, "error.unknown")
                        .with_param("detail", join_err.to_string())
                        .with_detail(join_err.to_string()));
                }
            };

        if let (Ok(meta), Ok(progress)) =
            (session.store().read_meta(), session.store().read_progress())
        {
            let completed_chapters = progress
                .chapters
                .values()
                .filter(|ch| ch.status == ChapterStatus::Completed)
                .count() as u32;
            let fallback = *self
                .fallback_locale
                .read()
                .unwrap_or_else(PoisonError::into_inner);
            if let Err(err) = self.global_store.record_recent_workspace(
                fallback,
                RecentWorkspaceItem {
                    workspace_id: meta.workspace_id,
                    workspace_root: meta.workspace_root,
                    course_id,
                    course_title,
                    language,
                    completed_chapters,
                    total_chapters,
                    last_opened_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                },
            ) {
                eprintln!("codelab warning: failed to update recent workspaces: {err}");
            }
        }

        let pending_or_cancelled = {
            let mut slot = self.slot.write().unwrap_or_else(PoisonError::into_inner);
            if self.shutting_down.load(Ordering::Acquire) {
                *slot = SessionSlot::Idle;
                None
            } else {
                let pending = self.drain_pending_metrics();
                self.current_epoch.store(next_epoch.0, Ordering::Release);
                *slot = SessionSlot::Active(Arc::clone(&session));
                Some(pending)
            }
        };

        let Some(pending) = pending_or_cancelled else {
            let _ = session.drain_and_close().await;
            return Err(AppError::new(ErrorCode::Cancelled, "error.notReady")
                .with_detail("application is shutting down"));
        };

        if !pending.is_empty() {
            if let Err(err) = session.metrics().record_batch(pending).await {
                eprintln!("codelab warning: failed to flush pending startup metrics: {err}");
            }
        }

        Ok(session)
    }

    pub fn require_active_session(
        &self,
        expected_epoch: SessionEpoch,
    ) -> Result<Arc<WorkspaceSession>, AppError> {
        let slot = self.slot.read().unwrap_or_else(PoisonError::into_inner);
        match &*slot {
            SessionSlot::Active(session) if session.epoch() == expected_epoch => {
                Ok(Arc::clone(session))
            }
            SessionSlot::Active(session) => Err(AppError::new(
                ErrorCode::SessionEpochStale,
                "error.notReady",
            )
            .with_param("expectedEpoch", expected_epoch.0.to_string())
            .with_param("activeEpoch", session.epoch().0.to_string())),
            SessionSlot::Closing { epoch } => Err(AppError::new(
                ErrorCode::SessionEpochStale,
                "error.notReady",
            )
            .with_param("expectedEpoch", expected_epoch.0.to_string())
            .with_param("closingEpoch", epoch.0.to_string())),
            SessionSlot::Idle | SessionSlot::TrustPending { .. } | SessionSlot::Opening { .. } => {
                Err(AppError::new(ErrorCode::WorkspaceNotOpen, "error.notReady"))
            }
        }
    }

    pub fn active_session(&self) -> Option<Arc<WorkspaceSession>> {
        let slot = self.slot.read().unwrap_or_else(PoisonError::into_inner);
        match &*slot {
            SessionSlot::Active(session) => Some(Arc::clone(session)),
            _ => None,
        }
    }

    pub async fn record_metric(
        &self,
        epoch: Option<SessionEpoch>,
        event: MetricEvent,
    ) -> Result<(), AppError> {
        match epoch {
            Some(expected_epoch) => {
                let session = self.require_active_session(expected_epoch)?;
                session.record_metric(event).await
            }
            None => {
                let active = {
                    let slot = self.slot.read().unwrap_or_else(PoisonError::into_inner);
                    match &*slot {
                        SessionSlot::Active(session) => Some(Arc::clone(session)),
                        _ => {
                            self.push_pending_metric(event.clone());
                            None
                        }
                    }
                };
                if let Some(session) = active {
                    session.record_metric(event).await
                } else {
                    Ok(())
                }
            }
        }
    }

    pub async fn close_workspace(
        &self,
        expected_epoch: SessionEpoch,
    ) -> Result<SessionShutdownReport, AppError> {
        let _op = self.lifecycle_lock.lock().await;

        let session = {
            let mut slot = self.slot.write().unwrap_or_else(PoisonError::into_inner);
            match &*slot {
                SessionSlot::Active(active) if active.epoch() == expected_epoch => {
                    let active = Arc::clone(active);
                    *slot = SessionSlot::Closing {
                        epoch: expected_epoch,
                    };
                    active
                }
                SessionSlot::Active(active) => {
                    return Err(
                        AppError::new(ErrorCode::SessionEpochStale, "error.notReady")
                            .with_param("expectedEpoch", expected_epoch.0.to_string())
                            .with_param("activeEpoch", active.epoch().0.to_string()),
                    );
                }
                SessionSlot::Closing { epoch } => {
                    return Err(
                        AppError::new(ErrorCode::SessionEpochStale, "error.notReady")
                            .with_param("closingEpoch", epoch.0.to_string()),
                    );
                }
                SessionSlot::Idle
                | SessionSlot::TrustPending { .. }
                | SessionSlot::Opening { .. } => {
                    return Err(AppError::new(ErrorCode::WorkspaceNotOpen, "error.notReady"));
                }
            }
        };

        let report = session.drain_and_close().await;

        {
            let mut slot = self.slot.write().unwrap_or_else(PoisonError::into_inner);
            *slot = SessionSlot::Idle;
        }

        Ok(report)
    }

    pub async fn shutdown(&self) -> Option<SessionShutdownReport> {
        self.shutting_down.store(true, Ordering::Release);
        let _op = self.lifecycle_lock.lock().await;

        let session = {
            let mut slot = self.slot.write().unwrap_or_else(PoisonError::into_inner);
            match std::mem::replace(&mut *slot, SessionSlot::Idle) {
                SessionSlot::Active(active) => {
                    *slot = SessionSlot::Closing {
                        epoch: active.epoch(),
                    };
                    Some(active)
                }
                _ => None,
            }
        };

        let report = match session {
            Some(active) => Some(active.drain_and_close().await),
            None => None,
        };

        let mut slot = self.slot.write().unwrap_or_else(PoisonError::into_inner);
        *slot = SessionSlot::Idle;
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::domain::ai::TeachingMode;
    use crate::domain::ids::RelWorkspacePath;
    use crate::domain::runtime::Language;
    use crate::domain::workspace::{
        AccentPreset, CourseSourceRef, ExternalIdeKind, StorageSchemaVersion, ThemeMode,
    };
    use crate::storage::WorkspaceInitSpec;

    fn isolated_state() -> (tempfile::TempDir, AppState) {
        let home = tempfile::tempdir().expect("temp home");
        let state =
            AppState::with_home_paths(CodelabHomePaths::from_root(home.path().to_path_buf()));
        (home, state)
    }

    fn open_params(ws: &std::path::Path) -> SessionOpenParams {
        SessionOpenParams::new(
            ws.to_path_buf(),
            true,
            WorkspaceInitSpec {
                course_id: "go-goframe-practice".to_owned(),
                course_source: CourseSourceRef::Builtin {
                    course_id: "go-goframe-practice".to_owned(),
                    entry_file: RelWorkspacePath::new("README.md").unwrap(),
                },
                pack_fingerprint: "sha256:test".to_owned(),
            },
            "Go & GoFrame",
            Language::Go,
            10,
        )
    }

    #[test]
    fn initial_epoch_is_zero_and_lifecycle_is_idle() {
        let (_home, state) = isolated_state();
        assert_eq!(state.current_epoch(), SessionEpoch(0));
        assert_eq!(state.lifecycle_state(), SessionLifecycleKind::Idle);
    }

    #[test]
    fn build_bootstrap_state_persists_config_and_records_app_launched_once() {
        let (_home, state) = isolated_state();

        let first = state.build_bootstrap_state(Locale::ZhCn).unwrap();
        let second = state.build_bootstrap_state(Locale::EnUs).unwrap();

        assert_eq!(first.epoch, SessionEpoch(0));
        assert_eq!(first.config.schema_version, StorageSchemaVersion::V1);
        assert_eq!(first.config.locale, Locale::ZhCn);
        assert_eq!(first.config.theme, ThemeMode::Light);
        assert_eq!(first.config.accent, AccentPreset::Neutral);
        assert_eq!(first.config.external_ide, ExternalIdeKind::Vscode);
        assert_eq!(first.config.default_teaching_mode, TeachingMode::Socratic);
        assert!(!first.config.opt_in_anonymous_telemetry);

        assert_eq!(second.epoch, SessionEpoch(0));
        assert_eq!(second.config.locale, Locale::ZhCn);

        let drained = state.drain_pending_metrics();
        assert_eq!(
            drained,
            vec![MetricEvent::AppLaunched {
                locale: Locale::ZhCn
            }]
        );
        assert!(state.drain_pending_metrics().is_empty());
    }

    #[test]
    fn pending_metrics_queue_is_bounded_and_drops_oldest() {
        let (_home, state) = isolated_state();
        let total = (PENDING_METRICS_CAPACITY + 44) as u32;

        for chapter_index in 0..total {
            state.push_pending_metric(MetricEvent::VerifyChapterClicked { chapter_index });
        }

        let drained = state.drain_pending_metrics();
        assert_eq!(drained.len(), PENDING_METRICS_CAPACITY);
        assert_eq!(
            drained.first(),
            Some(&MetricEvent::VerifyChapterClicked { chapter_index: 44 })
        );
        assert_eq!(
            drained.last(),
            Some(&MetricEvent::VerifyChapterClicked {
                chapter_index: total - 1
            })
        );
        assert!(state.drain_pending_metrics().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn session_lifecycle_enforces_single_active_epoch_and_shutdown_drain() {
        let (_home, state) = isolated_state();
        let ws1 = tempfile::tempdir().unwrap();
        let ws2 = tempfile::tempdir().unwrap();

        let _ = state.build_bootstrap_state(Locale::ZhCn).unwrap();
        state
            .begin_trust_pending("local:demo", "sha256:test")
            .await
            .unwrap();
        assert_eq!(
            state.lifecycle_state(),
            SessionLifecycleKind::TrustPending {
                source_key: "local:demo".to_owned(),
                fingerprint: "sha256:test".to_owned(),
            }
        );

        let s1 = state.open_workspace(open_params(ws1.path())).await.unwrap();
        assert_eq!(s1.epoch(), SessionEpoch(1));
        assert_eq!(state.current_epoch(), SessionEpoch(1));
        assert!(state.drain_pending_metrics().is_empty());

        assert!(state.require_active_session(SessionEpoch(1)).is_ok());
        assert_eq!(
            state
                .require_active_session(SessionEpoch(99))
                .unwrap_err()
                .code,
            ErrorCode::SessionEpochStale
        );

        let s2 = state.open_workspace(open_params(ws2.path())).await.unwrap();
        assert_eq!(s2.epoch(), SessionEpoch(2));
        assert!(s1.cancel_token().is_cancelled());
        assert!(!s1.paths().lock_file().exists());
        assert!(s2.paths().lock_file().exists());

        assert_eq!(
            state
                .require_active_session(SessionEpoch(1))
                .unwrap_err()
                .code,
            ErrorCode::SessionEpochStale
        );

        let report = state.shutdown().await.unwrap();
        assert_eq!(report.epoch, SessionEpoch(2));
        assert_eq!(state.lifecycle_state(), SessionLifecycleKind::Idle);
        assert!(!s2.paths().lock_file().exists());
        assert_eq!(
            state
                .open_workspace(open_params(ws1.path()))
                .await
                .unwrap_err()
                .code,
            ErrorCode::Cancelled
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shutdown_barrier_bounds_hung_hooks_and_flushes_metrics() {
        let (_home, state) = isolated_state();
        let ws = tempfile::tempdir().unwrap();
        let mut params = open_params(ws.path());
        params.shutdown_timeout = Duration::from_millis(120);

        let session = state.open_workspace(params).await.unwrap();
        let finished = Arc::new(AtomicBool::new(false));
        let finished_clone = Arc::clone(&finished);

        session
            .register_shutdown_hook(Box::new(move || {
                Box::pin(async move {
                    finished_clone.store(true, Ordering::SeqCst);
                })
            }))
            .await;
        session
            .register_shutdown_hook(Box::new(|| {
                Box::pin(async {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                })
            }))
            .await;

        state
            .record_metric(
                Some(session.epoch()),
                MetricEvent::FileTreeOpen { opened: true },
            )
            .await
            .unwrap();

        let started = Instant::now();
        let report = state.close_workspace(session.epoch()).await.unwrap();
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(report.hooks_ran, 2);
        assert!(report.hooks_timed_out);
        assert!(finished.load(Ordering::SeqCst));
        assert_eq!(report.metrics.total_persisted_events, 1);
        assert!(!session.paths().lock_file().exists());
        assert_eq!(state.lifecycle_state(), SessionLifecycleKind::Idle);
    }

    #[tokio::test]
    async fn shutdown_completes_without_deadlock_on_current_thread_runtime() {
        let (_home, state) = isolated_state();
        let ws = tempfile::tempdir().unwrap();
        let session = state.open_workspace(open_params(ws.path())).await.unwrap();
        state
            .record_metric(
                Some(session.epoch()),
                MetricEvent::FileTreeOpen { opened: false },
            )
            .await
            .unwrap();

        let report = state.shutdown().await.unwrap();
        assert_eq!(report.epoch, session.epoch());
        assert_eq!(report.metrics.total_persisted_events, 1);
        assert_eq!(state.lifecycle_state(), SessionLifecycleKind::Idle);
    }
}

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::domain::error::{AppError, ErrorCode};
use crate::domain::events::{SessionEventEnvelope, StorageCorruptedEvent};
use crate::domain::ids::SessionEpoch;
use crate::domain::metrics::MetricEvent;
use crate::domain::runtime::Language;
use crate::platform::{WorkspaceLock, WorkspaceLockError, WorkspacePaths};
use crate::storage::{
    LoadOutcome, MetricsShutdownStats, MetricsWriter, MetricsWriterConfig, MetricsWriterHandle,
    WorkspaceInitSpec, WorkspaceOpenReport, WorkspaceStore,
};

pub const DEFAULT_SHUTDOWN_BARRIER_TIMEOUT: Duration = Duration::from_secs(3);

pub type ShutdownHook = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

#[derive(Debug, Clone)]
pub struct SessionOpenParams {
    pub workspace_root: PathBuf,
    pub create_if_missing: bool,
    pub init_spec: WorkspaceInitSpec,
    pub course_title: String,
    pub language: Language,
    pub total_chapters: u32,
    pub metrics_config: MetricsWriterConfig,
    pub shutdown_timeout: Duration,
}

impl SessionOpenParams {
    pub fn new(
        workspace_root: PathBuf,
        create_if_missing: bool,
        init_spec: WorkspaceInitSpec,
        course_title: impl Into<String>,
        language: Language,
        total_chapters: u32,
    ) -> Self {
        Self {
            workspace_root,
            create_if_missing,
            init_spec,
            course_title: course_title.into(),
            language,
            total_chapters,
            metrics_config: MetricsWriterConfig::default(),
            shutdown_timeout: DEFAULT_SHUTDOWN_BARRIER_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionShutdownReport {
    pub epoch: SessionEpoch,
    pub hooks_ran: usize,
    pub hooks_timed_out: bool,
    pub metrics: MetricsShutdownStats,
}

pub struct WorkspaceSession {
    epoch: SessionEpoch,
    cancel_token: CancellationToken,
    store: WorkspaceStore,
    metrics: MetricsWriterHandle,
    lock: Mutex<Option<WorkspaceLock>>,
    shutdown_hooks: tokio::sync::Mutex<Vec<ShutdownHook>>,
    shutdown_timeout: Duration,
    open_report: WorkspaceOpenReport,
    corruption_events: Mutex<Vec<SessionEventEnvelope<StorageCorruptedEvent>>>,
}

impl std::fmt::Debug for WorkspaceSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceSession")
            .field("epoch", &self.epoch)
            .field("workspace_root", &self.store.paths().root())
            .field("cancelled", &self.cancel_token.is_cancelled())
            .finish()
    }
}

impl WorkspaceSession {
    pub fn open(epoch: SessionEpoch, params: SessionOpenParams) -> Result<Self, AppError> {
        let paths = WorkspacePaths::from_root(&params.workspace_root, params.create_if_missing)
            .map_err(|err| {
                AppError::new(ErrorCode::PathOutsideSandbox, "error.invalidInput")
                    .with_param("field", "workspaceRoot")
                    .with_detail(err.to_string())
            })?;

        let lock = WorkspaceLock::acquire(&paths).map_err(lock_error_to_app_error)?;

        let (store, mut open_report) =
            WorkspaceStore::open_or_create(paths.clone(), params.init_spec)
                .map_err(AppError::from)?;

        let (metrics, metrics_outcome) =
            MetricsWriter::start(paths.metrics_json(), params.metrics_config)
                .map_err(AppError::from)?;

        match &metrics_outcome {
            LoadOutcome::RecoveredFromBackup { .. } => {
                open_report
                    .recovered_from_backup
                    .push("metrics.json".to_owned());
            }
            LoadOutcome::ResetAfterCorruption { event, .. } => {
                open_report.corruption_events.push(event.clone());
            }
            LoadOutcome::CreatedDefault(_)
            | LoadOutcome::Loaded(_)
            | LoadOutcome::Migrated { .. } => {}
        }

        let envelopes: Vec<SessionEventEnvelope<StorageCorruptedEvent>> = open_report
            .corruption_events
            .iter()
            .cloned()
            .map(|payload| SessionEventEnvelope { epoch, payload })
            .collect();

        Ok(Self {
            epoch,
            cancel_token: CancellationToken::new(),
            store,
            metrics,
            lock: Mutex::new(Some(lock)),
            shutdown_hooks: tokio::sync::Mutex::new(Vec::new()),
            shutdown_timeout: params.shutdown_timeout,
            open_report,
            corruption_events: Mutex::new(envelopes),
        })
    }

    pub fn epoch(&self) -> SessionEpoch {
        self.epoch
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub fn paths(&self) -> &WorkspacePaths {
        self.store.paths()
    }

    pub fn store(&self) -> &WorkspaceStore {
        &self.store
    }

    pub fn metrics(&self) -> &MetricsWriterHandle {
        &self.metrics
    }

    pub fn open_report(&self) -> &WorkspaceOpenReport {
        &self.open_report
    }

    pub fn take_corruption_events(&self) -> Vec<SessionEventEnvelope<StorageCorruptedEvent>> {
        let mut guard = self
            .corruption_events
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *guard)
    }

    pub async fn register_shutdown_hook(&self, hook: ShutdownHook) {
        let mut hooks = self.shutdown_hooks.lock().await;
        hooks.push(hook);
    }

    pub async fn record_metric(&self, event: MetricEvent) -> Result<(), AppError> {
        self.metrics.record(event).await
    }

    pub async fn drain_and_close(&self) -> SessionShutdownReport {
        self.cancel_token.cancel();

        let hooks = {
            let mut guard = self.shutdown_hooks.lock().await;
            std::mem::take(&mut *guard)
        };
        let hooks_ran = hooks.len();
        let mut hooks_timed_out = false;

        if !hooks.is_empty() {
            let mut join_set = JoinSet::new();
            for hook in hooks {
                join_set.spawn(hook());
            }

            let drain_future = async { while join_set.join_next().await.is_some() {} };

            if tokio::time::timeout(self.shutdown_timeout, drain_future)
                .await
                .is_err()
            {
                hooks_timed_out = true;
                join_set.abort_all();
                while join_set.join_next().await.is_some() {}
            }
        }

        let metrics = self.metrics.shutdown().await.unwrap_or_default();

        {
            let mut lock_guard = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
            let _ = lock_guard.take();
        }

        SessionShutdownReport {
            epoch: self.epoch,
            hooks_ran,
            hooks_timed_out,
            metrics,
        }
    }
}

impl Drop for WorkspaceSession {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

fn lock_error_to_app_error(err: WorkspaceLockError) -> AppError {
    match err {
        WorkspaceLockError::AlreadyLocked {
            pid,
            acquired_at,
            workspace_root,
        } => AppError::new(ErrorCode::ExternalEditConflict, "error.notReady")
            .with_param("pid", pid.to_string())
            .with_param("acquiredAt", acquired_at)
            .with_param("workspaceRoot", workspace_root)
            .with_detail(format!("workspace is locked by live process {pid}"))
            .with_retryable(true),
        WorkspaceLockError::Io(io_err) => {
            AppError::new(ErrorCode::StorageCorruptedRecovered, "error.unknown")
                .with_param("detail", io_err.to_string())
                .with_detail(io_err.to_string())
                .with_retryable(true)
        }
    }
}

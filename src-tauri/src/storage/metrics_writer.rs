use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

use crate::domain::ai::{AcpOperationKind, SelectionSource, TeachingMode};
use crate::domain::error::{AppError, ErrorCode};
use crate::domain::metrics::{MetricEvent, MetricRecord, MetricValue, MetricsLog};
use crate::domain::runtime::{Language, ManagedRuntimeTarget};
use crate::domain::workspace::{Locale, StorageSchemaVersion};
use crate::storage::atomic_json::{AtomicJsonStore, LoadOutcome, StorageError};

pub const DEFAULT_METRICS_FLUSH_INTERVAL: Duration = Duration::from_secs(2);
pub const DEFAULT_METRICS_BATCH_SIZE: usize = 20;
pub const MAX_METRICS_EVENTS: usize = 5_000;
pub const DEFAULT_METRICS_CHANNEL_CAPACITY: usize = 1024;

static NEXT_EVENT_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetricsWriterConfig {
    pub flush_interval: Duration,
    pub batch_size: usize,
    pub max_events: usize,
    pub channel_capacity: usize,
}

impl Default for MetricsWriterConfig {
    fn default() -> Self {
        Self {
            flush_interval: DEFAULT_METRICS_FLUSH_INTERVAL,
            batch_size: DEFAULT_METRICS_BATCH_SIZE,
            max_events: MAX_METRICS_EVENTS,
            channel_capacity: DEFAULT_METRICS_CHANNEL_CAPACITY,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MetricsShutdownStats {
    pub total_persisted_events: usize,
    pub flush_count: u64,
}

type FlushReply = oneshot::Sender<Result<u64, StorageError>>;
type ShutdownReply = oneshot::Sender<Result<MetricsShutdownStats, StorageError>>;

struct DrainedReplies {
    flushes: Vec<FlushReply>,
    shutdowns: Vec<ShutdownReply>,
}

enum MetricsCommand {
    Record(MetricEvent),
    RecordBatch(Vec<MetricEvent>),
    Flush(FlushReply),
    Shutdown(ShutdownReply),
}

struct WriterLifecycle {
    join_handle: Option<JoinHandle<()>>,
    completed_stats: Option<MetricsShutdownStats>,
}

#[derive(Clone)]
pub struct MetricsWriterHandle {
    tx: mpsc::Sender<MetricsCommand>,
    lifecycle: Arc<Mutex<WriterLifecycle>>,
}

impl std::fmt::Debug for MetricsWriterHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsWriterHandle")
            .field("is_closed", &self.tx.is_closed())
            .finish()
    }
}

pub struct MetricsWriter;

impl MetricsWriter {
    pub fn start(
        path: PathBuf,
        config: MetricsWriterConfig,
    ) -> Result<(MetricsWriterHandle, LoadOutcome<MetricsLog>), StorageError> {
        let path_for_err = path.clone();
        let store = AtomicJsonStore::new(path);
        let outcome = store.load_or_init(|| MetricsLog {
            schema_version: StorageSchemaVersion::V1,
            events: Vec::new(),
        })?;

        let mut initial_log = outcome.value().clone();
        enforce_rolling_cap(&mut initial_log.events, config.max_events);

        let capacity = config.channel_capacity.max(1);
        let (tx, rx) = mpsc::channel(capacity);
        let join_handle = thread::Builder::new()
            .name("codelab-metrics-writer".to_owned())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(err) => {
                        eprintln!("codelab metrics writer runtime init failed: {err}");
                        return;
                    }
                };
                rt.block_on(run_writer_loop(store, initial_log, config, rx));
            })
            .map_err(|source| StorageError::Io {
                path: path_for_err,
                source,
            })?;

        let handle = MetricsWriterHandle {
            tx,
            lifecycle: Arc::new(Mutex::new(WriterLifecycle {
                join_handle: Some(join_handle),
                completed_stats: None,
            })),
        };

        Ok((handle, outcome))
    }
}

impl MetricsWriterHandle {
    pub async fn record(&self, event: MetricEvent) -> Result<(), AppError> {
        self.tx
            .send(MetricsCommand::Record(event))
            .await
            .map_err(|_| closed_writer_error())
    }

    pub async fn record_batch(&self, events: Vec<MetricEvent>) -> Result<(), AppError> {
        if events.is_empty() {
            return Ok(());
        }
        self.tx
            .send(MetricsCommand::RecordBatch(events))
            .await
            .map_err(|_| closed_writer_error())
    }

    pub async fn flush(&self) -> Result<u64, AppError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(MetricsCommand::Flush(reply_tx))
            .await
            .map_err(|_| closed_writer_error())?;
        let flush_count = reply_rx
            .await
            .map_err(|_| closed_writer_error())?
            .map_err(AppError::from)?;
        Ok(flush_count)
    }

    pub async fn shutdown(&self) -> Result<MetricsShutdownStats, AppError> {
        let mut lifecycle = self.lifecycle.lock().await;
        if let Some(stats) = lifecycle.completed_stats {
            return Ok(stats);
        }

        let (reply_tx, reply_rx) = oneshot::channel();
        let send_ok = self
            .tx
            .send(MetricsCommand::Shutdown(reply_tx))
            .await
            .is_ok();

        let stats = if send_ok {
            match reply_rx.await {
                Ok(result) => result.map_err(AppError::from)?,
                Err(_) => MetricsShutdownStats::default(),
            }
        } else {
            MetricsShutdownStats::default()
        };

        if let Some(handle) = lifecycle.join_handle.take() {
            let _ = tokio::task::spawn_blocking(move || {
                let _ = handle.join();
            })
            .await;
        }
        lifecycle.completed_stats = Some(stats);
        Ok(stats)
    }
}

fn try_flush_background(
    store: &AtomicJsonStore,
    log: &MetricsLog,
    pending_since_flush: &mut usize,
    flush_count: &mut u64,
) {
    match store.write_atomic(log) {
        Ok(()) => {
            *pending_since_flush = 0;
            *flush_count += 1;
        }
        Err(err) => {
            eprintln!(
                "codelab metrics flush failed at '{}': {err}",
                store.path().display()
            );
        }
    }
}

async fn run_writer_loop(
    store: AtomicJsonStore,
    mut log: MetricsLog,
    config: MetricsWriterConfig,
    mut rx: mpsc::Receiver<MetricsCommand>,
) {
    let batch_size = config.batch_size.max(1);
    let max_events = config.max_events.max(1);

    let mut ticker = tokio::time::interval(config.flush_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    ticker.tick().await;

    let mut pending_since_flush: usize = 0;
    let mut flush_count: u64 = 0;

    loop {
        tokio::select! {
            biased;
            cmd = rx.recv() => {
                match cmd {
                    Some(MetricsCommand::Record(event)) => {
                        let flush_now = matches!(event, MetricEvent::ChapterMarkedDone { .. });
                        push_event(&mut log.events, event, max_events);
                        pending_since_flush += 1;
                        if flush_now || pending_since_flush >= batch_size {
                            try_flush_background(
                                &store,
                                &log,
                                &mut pending_since_flush,
                                &mut flush_count,
                            );
                        }
                    }
                    Some(MetricsCommand::RecordBatch(events)) => {
                        for event in events {
                            push_event(&mut log.events, event, max_events);
                            pending_since_flush += 1;
                        }
                        if pending_since_flush > 0 {
                            try_flush_background(
                                &store,
                                &log,
                                &mut pending_since_flush,
                                &mut flush_count,
                            );
                        }
                    }
                    Some(MetricsCommand::Flush(reply)) => {
                        let result = if pending_since_flush > 0 {
                            store.write_atomic(&log).map(|()| {
                                pending_since_flush = 0;
                                flush_count += 1;
                                flush_count
                            })
                        } else {
                            Ok(flush_count)
                        };
                        let _ = reply.send(result);
                    }
                    Some(MetricsCommand::Shutdown(reply)) => {
                        let drained = drain_remaining_commands(
                            &mut rx,
                            &mut log.events,
                            &mut pending_since_flush,
                            max_events,
                        );
                        let write_res = if pending_since_flush > 0 {
                            store.write_atomic(&log).map(|()| {
                                pending_since_flush = 0;
                                flush_count += 1;
                            })
                        } else {
                            Ok(())
                        };
                        match write_res {
                            Ok(()) => {
                                for flush_reply in drained.flushes {
                                    let _ = flush_reply.send(Ok(flush_count));
                                }
                                let stats = MetricsShutdownStats {
                                    total_persisted_events: log.events.len(),
                                    flush_count,
                                };
                                for shutdown_reply in drained.shutdowns {
                                    let _ = shutdown_reply.send(Ok(stats));
                                }
                                let _ = reply.send(Ok(stats));
                            }
                            Err(err) => {
                                let msg = err.to_string();
                                for flush_reply in drained.flushes {
                                    let _ = flush_reply.send(Err(StorageError::Domain(
                                        AppError::new(
                                            ErrorCode::StorageCorruptedRecovered,
                                            "error.unknown",
                                        )
                                        .with_param("detail", msg.clone())
                                        .with_detail(msg.clone()),
                                    )));
                                }
                                for shutdown_reply in drained.shutdowns {
                                    let _ = shutdown_reply.send(Err(StorageError::Domain(
                                        AppError::new(
                                            ErrorCode::StorageCorruptedRecovered,
                                            "error.unknown",
                                        )
                                        .with_param("detail", msg.clone())
                                        .with_detail(msg.clone()),
                                    )));
                                }
                                let _ = reply.send(Err(err));
                            }
                        }
                        break;
                    }
                    None => {
                        if pending_since_flush > 0 {
                            try_flush_background(
                                &store,
                                &log,
                                &mut pending_since_flush,
                                &mut flush_count,
                            );
                        }
                        break;
                    }
                }
            }
            _ = ticker.tick() => {
                if pending_since_flush > 0 {
                    try_flush_background(
                        &store,
                        &log,
                        &mut pending_since_flush,
                        &mut flush_count,
                    );
                }
            }
        }
    }
}

fn drain_remaining_commands(
    rx: &mut mpsc::Receiver<MetricsCommand>,
    events: &mut Vec<MetricRecord>,
    pending_since_flush: &mut usize,
    max_events: usize,
) -> DrainedReplies {
    let mut flushes = Vec::new();
    let mut shutdowns = Vec::new();

    while let Ok(cmd) = rx.try_recv() {
        match cmd {
            MetricsCommand::Record(event) => {
                push_event(events, event, max_events);
                *pending_since_flush += 1;
            }
            MetricsCommand::RecordBatch(batch) => {
                for event in batch {
                    push_event(events, event, max_events);
                    *pending_since_flush += 1;
                }
            }
            MetricsCommand::Flush(reply) => {
                flushes.push(reply);
            }
            MetricsCommand::Shutdown(reply) => {
                shutdowns.push(reply);
            }
        }
    }

    DrainedReplies { flushes, shutdowns }
}

fn push_event(events: &mut Vec<MetricRecord>, event: MetricEvent, max_events: usize) {
    events.push(metric_event_to_record(event));
    enforce_rolling_cap(events, max_events);
}

fn enforce_rolling_cap(events: &mut Vec<MetricRecord>, max_events: usize) {
    if events.len() > max_events {
        let excess = events.len() - max_events;
        events.drain(0..excess);
    }
}

pub fn metric_event_to_record(event: MetricEvent) -> MetricRecord {
    let name = event.event_name();
    let seq = NEXT_EVENT_SEQ.fetch_add(1, Ordering::Relaxed);
    let id = format!("evt_{}_{seq}", Uuid::new_v4().simple());
    let ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut payload = BTreeMap::new();

    match event {
        MetricEvent::AppLaunched { locale } => {
            payload.insert(
                "locale".to_owned(),
                MetricValue::String(locale_str(locale).to_owned()),
            );
        }
        MetricEvent::CourseOpened {
            source_kind,
            language,
        } => {
            payload.insert("sourceKind".to_owned(), MetricValue::String(source_kind));
            if let Some(lang) = language {
                payload.insert(
                    "language".to_owned(),
                    MetricValue::String(language_str(lang).to_owned()),
                );
            }
        }
        MetricEvent::PreflightExecuted {
            ok_count,
            warn_count,
            missing_count,
        } => {
            payload.insert("okCount".to_owned(), MetricValue::Int(clamp_u32(ok_count)));
            payload.insert(
                "warnCount".to_owned(),
                MetricValue::Int(clamp_u32(warn_count)),
            );
            payload.insert(
                "missingCount".to_owned(),
                MetricValue::Int(clamp_u32(missing_count)),
            );
        }
        MetricEvent::RuntimeInstalled { target, success } => {
            payload.insert(
                "target".to_owned(),
                MetricValue::String(runtime_target_str(target).to_owned()),
            );
            payload.insert("success".to_owned(), MetricValue::Bool(success));
        }
        MetricEvent::CmdBlockClicked { role_kind } => {
            payload.insert("roleKind".to_owned(), MetricValue::String(role_kind));
        }
        MetricEvent::CmdRiskModalAction { confirmed } => {
            payload.insert("confirmed".to_owned(), MetricValue::Bool(confirmed));
        }
        MetricEvent::CmdFinished {
            exit_code,
            duration_ms,
        } => {
            if let Some(code) = exit_code {
                payload.insert("exitCode".to_owned(), MetricValue::Int(code));
            }
            payload.insert(
                "durationMs".to_owned(),
                MetricValue::Int(clamp_u64(duration_ms)),
            );
        }
        MetricEvent::FileTreeOpen { opened } => {
            payload.insert("opened".to_owned(), MetricValue::Bool(opened));
        }
        MetricEvent::SelectionAiClicked { source } => {
            payload.insert(
                "source".to_owned(),
                MetricValue::String(selection_source_str(source).to_owned()),
            );
        }
        MetricEvent::AiModeSwitched { mode } => {
            payload.insert(
                "mode".to_owned(),
                MetricValue::String(teaching_mode_str(mode).to_owned()),
            );
        }
        MetricEvent::AcpPermissionDecision { operation, allowed } => {
            payload.insert(
                "operation".to_owned(),
                MetricValue::String(acp_operation_str(operation).to_owned()),
            );
            payload.insert("allowed".to_owned(), MetricValue::Bool(allowed));
        }
        MetricEvent::VerifyChapterClicked { chapter_index } => {
            payload.insert(
                "chapterIndex".to_owned(),
                MetricValue::Int(clamp_u32(chapter_index)),
            );
        }
        MetricEvent::ChapterMarkedDone {
            chapter_index,
            auto_passed,
        } => {
            payload.insert(
                "chapterIndex".to_owned(),
                MetricValue::Int(clamp_u32(chapter_index)),
            );
            payload.insert("autoPassed".to_owned(), MetricValue::Bool(auto_passed));
        }
    }

    MetricRecord {
        id,
        name,
        ts,
        payload,
    }
}

fn clamp_u32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn clamp_u64(value: u64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn locale_str(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "zh-CN",
        Locale::EnUs => "en-US",
    }
}

fn language_str(lang: Language) -> &'static str {
    match lang {
        Language::Js => "js",
        Language::Ts => "ts",
        Language::Go => "go",
        Language::Java => "java",
        Language::Python => "python",
    }
}

fn runtime_target_str(target: ManagedRuntimeTarget) -> &'static str {
    match target {
        ManagedRuntimeTarget::Bun => "bun",
        ManagedRuntimeTarget::Go => "go",
        ManagedRuntimeTarget::TemurinJdk21 => "temurin_jdk21",
        ManagedRuntimeTarget::UvPython => "uv_python",
    }
}

fn selection_source_str(source: SelectionSource) -> &'static str {
    match source {
        SelectionSource::Manual => "manual",
        SelectionSource::Editor => "editor",
        SelectionSource::Terminal => "terminal",
    }
}

fn teaching_mode_str(mode: TeachingMode) -> &'static str {
    match mode {
        TeachingMode::Socratic => "socratic",
        TeachingMode::Direct => "direct",
    }
}

fn acp_operation_str(op: AcpOperationKind) -> &'static str {
    match op {
        AcpOperationKind::Read => "read",
        AcpOperationKind::Write => "write",
        AcpOperationKind::Exec => "exec",
        AcpOperationKind::Delete => "delete",
    }
}

fn closed_writer_error() -> AppError {
    AppError::new(ErrorCode::WorkspaceNotOpen, "error.notReady")
        .with_detail("metrics writer channel is closed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::domain::metrics::MetricEventName;

    #[test]
    fn all_13_metric_events_convert_to_desensitized_records() {
        let events = vec![
            MetricEvent::AppLaunched {
                locale: Locale::ZhCn,
            },
            MetricEvent::CourseOpened {
                source_kind: "builtin".to_owned(),
                language: Some(Language::Go),
            },
            MetricEvent::PreflightExecuted {
                ok_count: 4,
                warn_count: 1,
                missing_count: 0,
            },
            MetricEvent::RuntimeInstalled {
                target: ManagedRuntimeTarget::Go,
                success: true,
            },
            MetricEvent::CmdBlockClicked {
                role_kind: "daemon".to_owned(),
            },
            MetricEvent::CmdRiskModalAction { confirmed: false },
            MetricEvent::CmdFinished {
                exit_code: Some(0),
                duration_ms: 350,
            },
            MetricEvent::FileTreeOpen { opened: true },
            MetricEvent::SelectionAiClicked {
                source: SelectionSource::Terminal,
            },
            MetricEvent::AiModeSwitched {
                mode: TeachingMode::Direct,
            },
            MetricEvent::AcpPermissionDecision {
                operation: AcpOperationKind::Write,
                allowed: true,
            },
            MetricEvent::VerifyChapterClicked { chapter_index: 2 },
            MetricEvent::ChapterMarkedDone {
                chapter_index: 2,
                auto_passed: true,
            },
        ];

        assert_eq!(events.len(), MetricEventName::ALL.len());
        let records: Vec<MetricRecord> = events.into_iter().map(metric_event_to_record).collect();
        for (record, expected_name) in records.iter().zip(MetricEventName::ALL.iter()) {
            assert_eq!(record.name, *expected_name);
            assert!(record.id.starts_with("evt_"));
            assert!(!record.ts.is_empty());
        }

        let log = MetricsLog {
            schema_version: StorageSchemaVersion::V1,
            events: records,
        };
        let encoded = serde_json::to_string(&log).expect("serialize metrics log");
        let decoded: MetricsLog = serde_json::from_str(&encoded).expect("round-trip metrics log");
        assert_eq!(decoded.events.len(), 13);
    }

    #[tokio::test]
    async fn concurrent_metric_writers_persist_all_events_without_loss_or_corruption() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("metrics.json");
        let (handle, outcome) =
            MetricsWriter::start(path.clone(), MetricsWriterConfig::default()).expect("start");
        assert!(matches!(outcome, LoadOutcome::CreatedDefault(_)));

        let mut tasks = Vec::new();
        for worker in 0..16u32 {
            let handle = handle.clone();
            tasks.push(tokio::spawn(async move {
                for step in 0..50u32 {
                    handle
                        .record(MetricEvent::VerifyChapterClicked {
                            chapter_index: worker * 100 + step,
                        })
                        .await
                        .expect("record metric");
                }
            }));
        }

        for task in tasks {
            task.await.expect("join metric task");
        }

        let stats = handle.shutdown().await.expect("shutdown metrics writer");
        assert_eq!(stats.total_persisted_events, 800);
        assert!(stats.flush_count >= 40);

        let stats_again = handle.shutdown().await.expect("idempotent second shutdown");
        assert_eq!(stats_again, stats);

        let log: MetricsLog = AtomicJsonStore::new(path)
            .read_valid()
            .expect("read metrics");
        assert_eq!(log.events.len(), 800);

        let tmp_files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(tmp_files.is_empty());
    }

    #[tokio::test]
    async fn chapter_marked_done_and_timer_trigger_flushes_and_cap_enforces_5000() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("metrics.json");
        let config = MetricsWriterConfig {
            flush_interval: Duration::from_millis(60),
            batch_size: 20,
            max_events: MAX_METRICS_EVENTS,
            channel_capacity: 512,
        };
        let (handle, _) = MetricsWriter::start(path.clone(), config).expect("start");

        handle
            .record(MetricEvent::ChapterMarkedDone {
                chapter_index: 1,
                auto_passed: true,
            })
            .await
            .unwrap();
        let _ = handle.flush().await.unwrap();
        let after_done: MetricsLog = AtomicJsonStore::new(path.clone()).read_valid().unwrap();
        assert_eq!(after_done.events.len(), 1);
        assert_eq!(
            after_done.events[0].name,
            MetricEventName::ChapterMarkedDone
        );

        handle
            .record(MetricEvent::FileTreeOpen { opened: true })
            .await
            .unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let mut after_timer = AtomicJsonStore::new(path.clone())
            .read_valid::<MetricsLog>()
            .unwrap();
        while after_timer.events.len() < 2 && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(25)).await;
            after_timer = AtomicJsonStore::new(path.clone())
                .read_valid::<MetricsLog>()
                .unwrap();
        }
        assert_eq!(after_timer.events.len(), 2);

        let batch: Vec<MetricEvent> = (0..5_200u32)
            .map(|idx| MetricEvent::VerifyChapterClicked { chapter_index: idx })
            .collect();
        handle.record_batch(batch).await.unwrap();
        let stats = handle.shutdown().await.unwrap();
        assert_eq!(stats.total_persisted_events, MAX_METRICS_EVENTS);

        let final_log: MetricsLog = AtomicJsonStore::new(path).read_valid().unwrap();
        assert_eq!(final_log.events.len(), MAX_METRICS_EVENTS);
        assert_eq!(
            final_log
                .events
                .first()
                .unwrap()
                .payload
                .get("chapterIndex"),
            Some(&MetricValue::Int(200))
        );
        assert_eq!(
            final_log.events.last().unwrap().payload.get("chapterIndex"),
            Some(&MetricValue::Int(5_199))
        );
    }

    #[tokio::test]
    async fn metrics_writer_recovers_corrupt_metrics_file_on_start() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("metrics.json");
        fs::write(&path, b"{truncated-metrics-json").expect("corrupt metrics");

        let (handle, outcome) =
            MetricsWriter::start(path.clone(), MetricsWriterConfig::default()).expect("start");
        assert!(matches!(
            outcome,
            LoadOutcome::ResetAfterCorruption { ref event, .. }
                if event.file_name == "metrics.json"
                    && event.backup_name.starts_with("metrics.json.corrupt.")
        ));

        handle
            .record(MetricEvent::AppLaunched {
                locale: Locale::EnUs,
            })
            .await
            .unwrap();
        let stats = handle.shutdown().await.unwrap();
        assert_eq!(stats.total_persisted_events, 1);

        let log: MetricsLog = AtomicJsonStore::new(path).read_valid().unwrap();
        assert_eq!(log.events.len(), 1);
    }
}

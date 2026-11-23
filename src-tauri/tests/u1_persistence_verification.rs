use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use codelab_studio_lib::domain::ai::{AcpOperationKind, SelectionSource, TeachingMode};
use codelab_studio_lib::domain::error::ErrorCode;
use codelab_studio_lib::domain::ids::{ChapterId, RelWorkspacePath, RunId, SessionEpoch};
use codelab_studio_lib::domain::metrics::{MetricEvent, MetricEventName, MetricsLog};
use codelab_studio_lib::domain::runtime::{Language, ManagedRuntimeTarget};
use codelab_studio_lib::domain::workspace::{
    AccentPreset, AutoCheckRecord, ChapterProgress, ChapterStatus, ChatMessage, ChatRole,
    CourseSourceRef, GlobalConfigPatch, Locale, StorageSchemaVersion, ThemeMode, WorkspaceMeta,
};
use codelab_studio_lib::platform::{CodelabHomePaths, WorkspaceLockRecord, WorkspacePaths};
use codelab_studio_lib::session::{AppState, SessionLifecycleKind, SessionOpenParams};
use codelab_studio_lib::storage::{
    AtomicJsonStore, LoadOutcome, MetricsWriterConfig, WorkspaceInitSpec, MAX_METRICS_EVENTS,
};

fn make_init_spec(course_id: &str) -> WorkspaceInitSpec {
    WorkspaceInitSpec {
        course_id: course_id.to_owned(),
        course_source: CourseSourceRef::Builtin {
            course_id: course_id.to_owned(),
            entry_file: RelWorkspacePath::new("README.md").unwrap(),
        },
        pack_fingerprint: "sha256:u1-verification".to_owned(),
    }
}

fn make_open_params(ws_root: &Path, course_id: &str) -> SessionOpenParams {
    SessionOpenParams::new(
        ws_root.to_path_buf(),
        true,
        make_init_spec(course_id),
        "Go & GoFrame Handbook",
        Language::Go,
        10,
    )
}

fn list_codelab_entries(ws_root: &Path) -> Vec<String> {
    let mut entries: Vec<String> = fs::read_dir(ws_root.join(".codelab"))
        .expect("read .codelab dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    entries
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn u1_verify_creation_binding_pending_metrics_and_schema_migration() {
    let home_dir = tempfile::tempdir().expect("temp home");
    let ws1_dir = tempfile::tempdir().expect("temp ws1");
    let ws2_dir = tempfile::tempdir().expect("temp ws2");

    let learner_main = ws1_dir.path().join("task-api").join("main.go");
    fs::create_dir_all(learner_main.parent().unwrap()).unwrap();
    fs::write(
        &learner_main,
        "package main\nfunc main() { println(\"ok\") }\n",
    )
    .unwrap();

    let state =
        AppState::with_home_paths(CodelabHomePaths::from_root(home_dir.path().to_path_buf()));

    let bootstrap = state.build_bootstrap_state(Locale::ZhCn).unwrap();
    assert_eq!(bootstrap.epoch, SessionEpoch(0));
    assert_eq!(bootstrap.config.locale, Locale::ZhCn);

    state
        .record_metric(
            None,
            MetricEvent::PreflightExecuted {
                ok_count: 3,
                warn_count: 1,
                missing_count: 0,
            },
        )
        .await
        .unwrap();

    let updated_cfg = state
        .update_settings(GlobalConfigPatch {
            theme: Some(ThemeMode::Dark),
            accent: Some(AccentPreset::Blue),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(updated_cfg.theme, ThemeMode::Dark);
    assert_eq!(updated_cfg.accent, AccentPreset::Blue);

    let s1 = state
        .open_workspace(make_open_params(ws1_dir.path(), "go-goframe-practice"))
        .await
        .unwrap();
    assert_eq!(s1.epoch(), SessionEpoch(1));
    assert!(s1.open_report().created_new);
    let created_ws_id = s1.store().read_meta().unwrap().workspace_id;

    s1.store()
        .update_meta(|meta| {
            meta.active_chapter_id = Some(ChapterId("ex-1-healthz".to_owned()));
            meta.open_tabs = vec![RelWorkspacePath::new("task-api/main.go").unwrap()];
            meta.active_tab = Some(RelWorkspacePath::new("task-api/main.go").unwrap());
            Ok(())
        })
        .unwrap();

    s1.store()
        .update_progress(|progress| {
            progress.chapters.insert(
                ChapterId("ex-1-healthz".to_owned()),
                ChapterProgress {
                    status: ChapterStatus::Completed,
                    completed_at: Some("2026-11-23T03:30:00Z".to_owned()),
                    last_run_id: Some(RunId("run_u1_1".to_owned())),
                    auto_checks: BTreeMap::from([(
                        "v1".to_owned(),
                        AutoCheckRecord {
                            passed: true,
                            latency_ms: 5,
                            checked_at: "2026-11-23T03:30:00Z".to_owned(),
                        },
                    )]),
                    self_checks: BTreeMap::from([("s1".to_owned(), true)]),
                },
            );
            Ok(())
        })
        .unwrap();

    s1.store()
        .append_chat_message(
            ChapterId("ex-1-healthz".to_owned()),
            ChatMessage {
                id: "msg_1".to_owned(),
                role: ChatRole::User,
                content: "How does ServeMux match GET /healthz?".to_owned(),
                teaching_mode: Some(TeachingMode::Socratic),
                created_at: "2026-11-23T03:30:00Z".to_owned(),
            },
        )
        .unwrap();

    let close1 = state.close_workspace(SessionEpoch(1)).await.unwrap();
    assert_eq!(close1.metrics.total_persisted_events, 2);
    assert_eq!(state.lifecycle_state(), SessionLifecycleKind::Idle);

    let ws1_metrics: MetricsLog = AtomicJsonStore::new(s1.paths().metrics_json())
        .read_valid()
        .unwrap();
    assert_eq!(
        ws1_metrics
            .events
            .iter()
            .map(|e| e.name)
            .collect::<Vec<_>>(),
        vec![
            MetricEventName::AppLaunched,
            MetricEventName::PreflightExecuted
        ]
    );

    let s2 = state
        .open_workspace(make_open_params(ws2_dir.path(), "ts-data-pipeline"))
        .await
        .unwrap();
    assert_eq!(s2.epoch(), SessionEpoch(2));
    let close2 = state.close_workspace(SessionEpoch(2)).await.unwrap();
    assert_eq!(
        close2.metrics.total_persisted_events, 0,
        "PendingAppMetrics must flush once and never duplicate into a second workspace"
    );

    let s3 = state
        .open_workspace(make_open_params(ws1_dir.path(), "go-goframe-practice"))
        .await
        .unwrap();
    assert_eq!(s3.epoch(), SessionEpoch(3));
    assert!(!s3.open_report().created_new);
    assert_eq!(s3.store().read_meta().unwrap().workspace_id, created_ws_id);
    assert_eq!(
        s3.store().read_meta().unwrap().active_chapter_id,
        Some(ChapterId("ex-1-healthz".to_owned()))
    );
    assert_eq!(
        s3.store()
            .read_progress()
            .unwrap()
            .chapters
            .get(&ChapterId("ex-1-healthz".to_owned()))
            .map(|c| c.status),
        Some(ChapterStatus::Completed)
    );
    assert_eq!(
        fs::read_to_string(&learner_main).unwrap(),
        "package main\nfunc main() { println(\"ok\") }\n"
    );

    let reloaded_bootstrap = state.build_bootstrap_state(Locale::EnUs).unwrap();
    assert_eq!(reloaded_bootstrap.config.locale, Locale::ZhCn);
    assert_eq!(reloaded_bootstrap.config.theme, ThemeMode::Dark);
    assert_eq!(reloaded_bootstrap.recent_workspaces.len(), 2);
    assert_eq!(
        reloaded_bootstrap.recent_workspaces[0].workspace_id,
        created_ws_id
    );

    state.close_workspace(SessionEpoch(3)).await.unwrap();

    let migrate_dir = tempfile::tempdir().unwrap();
    let migrate_path = migrate_dir.path().join("workspace.json");
    let legacy_v0 = serde_json::json!({
        "schemaVersion": 0,
        "workspaceId": "ws_legacy_001",
        "workspaceRoot": migrate_dir.path().to_string_lossy(),
        "courseId": "go-goframe-practice"
    });
    fs::write(
        &migrate_path,
        serde_json::to_vec_pretty(&legacy_v0).unwrap(),
    )
    .unwrap();

    let migrated = AtomicJsonStore::new(migrate_path.clone())
        .load_or_init_with_migrator(
            || panic!("migration should succeed without falling back to default"),
            |ver, raw| {
                if ver != 0 {
                    return None;
                }
                let ws_id = raw.get("workspaceId")?.as_str()?.to_owned();
                let ws_root = raw.get("workspaceRoot")?.as_str()?.to_owned();
                let course_id = raw.get("courseId")?.as_str()?.to_owned();
                Some(Ok(WorkspaceMeta {
                    schema_version: StorageSchemaVersion::V1,
                    workspace_id: ws_id,
                    workspace_root: ws_root,
                    course_source: CourseSourceRef::Builtin {
                        course_id,
                        entry_file: RelWorkspacePath::new("README.md").unwrap(),
                    },
                    pack_fingerprint: "sha256:migrated".to_owned(),
                    hydrated_files: Vec::new(),
                    active_chapter_id: None,
                    open_tabs: Vec::new(),
                    active_tab: None,
                    layout: codelab_studio_lib::domain::workspace::LayoutSnapshot {
                        manual_width_pct: 35,
                        bottom_height_px: 256,
                        focus_mode: false,
                    },
                    updated_at: "2026-11-23T03:30:00Z".to_owned(),
                }))
            },
        )
        .unwrap();

    assert!(matches!(
        migrated,
        LoadOutcome::Migrated {
            from_version: 0,
            ref value,
            ..
        } if value.workspace_id == "ws_legacy_001"
    ));

    println!(
        "[U1 VERIFY: CREATE/BIND/MIGRATE] created_ws_id={} epochs=1->2->3 pending_metrics_flushed_once=2/0 recent_workspaces={} schema_v0_to_v1_migrated=true",
        created_ws_id,
        reloaded_bootstrap.recent_workspaces.len()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn u1_verify_corruption_recovery_bak_corrupt_tmp_and_stale_lock() {
    let home_dir = tempfile::tempdir().expect("temp home");
    let ws_dir = tempfile::tempdir().expect("temp ws");
    let state =
        AppState::with_home_paths(CodelabHomePaths::from_root(home_dir.path().to_path_buf()));

    let s1 = state
        .open_workspace(make_open_params(ws_dir.path(), "go-goframe-practice"))
        .await
        .unwrap();
    let original_ws_id = s1.store().read_meta().unwrap().workspace_id;

    s1.store()
        .update_meta(|meta| {
            meta.active_chapter_id = Some(ChapterId("ex-1-stable-bak".to_owned()));
            Ok(())
        })
        .unwrap();
    s1.store()
        .update_meta(|meta| {
            meta.active_chapter_id = Some(ChapterId("ex-2-latest".to_owned()));
            Ok(())
        })
        .unwrap();

    s1.store()
        .update_progress(|p| {
            p.chapters.insert(
                ChapterId("ex-1-stable-bak".to_owned()),
                ChapterProgress {
                    status: ChapterStatus::Completed,
                    completed_at: Some("2026-11-23T03:31:00Z".to_owned()),
                    last_run_id: None,
                    auto_checks: BTreeMap::new(),
                    self_checks: BTreeMap::new(),
                },
            );
            Ok(())
        })
        .unwrap();
    s1.store()
        .update_progress(|p| {
            p.chapters.insert(
                ChapterId("ex-2-latest".to_owned()),
                ChapterProgress {
                    status: ChapterStatus::InProgress,
                    completed_at: None,
                    last_run_id: None,
                    auto_checks: BTreeMap::new(),
                    self_checks: BTreeMap::new(),
                },
            );
            Ok(())
        })
        .unwrap();

    s1.store()
        .append_chat_message(
            ChapterId("ex-1".to_owned()),
            ChatMessage {
                id: "m1".to_owned(),
                role: ChatRole::User,
                content: "turn 1".to_owned(),
                teaching_mode: Some(TeachingMode::Socratic),
                created_at: "2026-11-23T03:31:00Z".to_owned(),
            },
        )
        .unwrap();

    s1.record_metric(MetricEvent::CourseOpened {
        source_kind: "builtin".to_owned(),
        language: Some(Language::Go),
    })
    .await
    .unwrap();

    let paths = s1.paths().clone();
    state.close_workspace(SessionEpoch(1)).await.unwrap();

    fs::write(
        paths.workspace_json(),
        br#"{"schemaVersion": 1, "workspaceId": "TRUNCATED_POWER_LOSS"#,
    )
    .unwrap();

    fs::write(
        paths.progress_json(),
        br#"{"schemaVersion": 99, "courseId": "future-incompatible", "chapters": {}}"#,
    )
    .unwrap();

    fs::write(paths.chat_json(), b"<<<CORRUPT_CHAT_PRIMARY>>>").unwrap();
    fs::write(
        paths.meta_dir().join("chat.json.bak"),
        b"<<<CORRUPT_CHAT_BACKUP>>>",
    )
    .unwrap();

    fs::write(paths.metrics_json(), b"{truncated-metrics-without-bak").unwrap();
    let _ = fs::remove_file(paths.meta_dir().join("metrics.json.bak"));

    fs::write(
        paths.meta_dir().join("workspace.json.tmp.99999.1"),
        b"stale temp residue",
    )
    .unwrap();
    let stale_lock = WorkspaceLockRecord {
        pid: 0,
        process_started_at_ms: 1_700_000_000_000,
        nonce: 7,
        acquired_at: "2026-01-01T00:00:00Z".to_owned(),
        workspace_root: paths.root_string(),
    };
    fs::write(
        paths.lock_file(),
        serde_json::to_vec_pretty(&stale_lock).unwrap(),
    )
    .unwrap();

    let recovered_session = state
        .open_workspace(make_open_params(ws_dir.path(), "go-goframe-practice"))
        .await
        .unwrap();

    let report = recovered_session.open_report();
    assert_eq!(
        report.cleaned_tmp_files,
        vec!["workspace.json.tmp.99999.1".to_owned()]
    );
    assert!(report
        .recovered_from_backup
        .contains(&"workspace.json".to_owned()));
    assert!(report
        .recovered_from_backup
        .contains(&"progress.json".to_owned()));

    let recovered_meta = recovered_session.store().read_meta().unwrap();
    assert_eq!(recovered_meta.workspace_id, original_ws_id);
    assert_eq!(
        recovered_meta.active_chapter_id,
        Some(ChapterId("ex-1-stable-bak".to_owned()))
    );

    let recovered_progress = recovered_session.store().read_progress().unwrap();
    assert_eq!(
        recovered_progress
            .chapters
            .get(&ChapterId("ex-1-stable-bak".to_owned()))
            .map(|c| c.status),
        Some(ChapterStatus::Completed)
    );
    assert!(!recovered_progress
        .chapters
        .contains_key(&ChapterId("ex-2-latest".to_owned())));

    let events = recovered_session.take_corruption_events();
    assert_eq!(events.len(), 2);
    for env in &events {
        assert_eq!(env.epoch, SessionEpoch(2));
        assert!(paths.meta_dir().join(&env.payload.backup_name).exists());
    }
    let corrupted_files: Vec<&str> = events
        .iter()
        .map(|e| e.payload.file_name.as_str())
        .collect();
    assert!(corrupted_files.contains(&"chat.json"));
    assert!(corrupted_files.contains(&"metrics.json"));

    let reset_chat = recovered_session.store().read_chat().unwrap();
    assert!(reset_chat.chapters.is_empty());

    let entries = list_codelab_entries(ws_dir.path());
    assert!(!entries.iter().any(|name| name.contains(".tmp.")));

    println!(
        "[U1 VERIFY: CORRUPTION RECOVERY] epoch={} cleaned_tmp={:?} recovered_from_bak={:?} quarantined_events={:?} workspace_id_preserved={} active_chapter_restored={:?} codelab_entries={:?}",
        recovered_session.epoch().0,
        report.cleaned_tmp_files,
        report.recovered_from_backup,
        events
            .iter()
            .map(|e| format!("{} -> {}", e.payload.file_name, e.payload.backup_name))
            .collect::<Vec<_>>(),
        recovered_meta.workspace_id == original_ws_id,
        recovered_meta.active_chapter_id,
        entries
    );

    let duplicate_open = WorkspacePaths::from_root(ws_dir.path(), false).unwrap();
    let lock_err = codelab_studio_lib::platform::WorkspaceLock::acquire(&duplicate_open);
    assert!(lock_err.is_err());

    state.close_workspace(SessionEpoch(2)).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn u1_verify_concurrent_writes_metrics_4_files_and_rolling_cap() {
    let home_dir = tempfile::tempdir().expect("temp home");
    let ws_dir = tempfile::tempdir().expect("temp ws");
    let state = Arc::new(AppState::with_home_paths(CodelabHomePaths::from_root(
        home_dir.path().to_path_buf(),
    )));

    let mut params = make_open_params(ws_dir.path(), "go-goframe-practice");
    params.metrics_config = MetricsWriterConfig {
        flush_interval: Duration::from_secs(2),
        batch_size: 20,
        max_events: MAX_METRICS_EVENTS,
        channel_capacity: 1024,
    };

    let session = state.open_workspace(params).await.unwrap();
    let epoch = session.epoch();
    let started = Instant::now();

    let metric_workers = 20u32;
    let events_per_worker = 60u32;
    let expected_metric_total = (metric_workers * events_per_worker) as usize;

    let mut handles = Vec::new();
    for worker in 0..metric_workers {
        let state = Arc::clone(&state);
        handles.push(tokio::spawn(async move {
            for seq in 0..events_per_worker {
                let event = match (worker + seq) % 13 {
                    0 => MetricEvent::AppLaunched {
                        locale: Locale::ZhCn,
                    },
                    1 => MetricEvent::CourseOpened {
                        source_kind: "builtin".to_owned(),
                        language: Some(Language::Go),
                    },
                    2 => MetricEvent::PreflightExecuted {
                        ok_count: 4,
                        warn_count: 0,
                        missing_count: 0,
                    },
                    3 => MetricEvent::RuntimeInstalled {
                        target: ManagedRuntimeTarget::Go,
                        success: true,
                    },
                    4 => MetricEvent::CmdBlockClicked {
                        role_kind: "oneshot".to_owned(),
                    },
                    5 => MetricEvent::CmdRiskModalAction { confirmed: false },
                    6 => MetricEvent::CmdFinished {
                        exit_code: Some(0),
                        duration_ms: u64::from(seq * 10),
                    },
                    7 => MetricEvent::FileTreeOpen { opened: true },
                    8 => MetricEvent::SelectionAiClicked {
                        source: SelectionSource::Editor,
                    },
                    9 => MetricEvent::AiModeSwitched {
                        mode: TeachingMode::Socratic,
                    },
                    10 => MetricEvent::AcpPermissionDecision {
                        operation: AcpOperationKind::Read,
                        allowed: true,
                    },
                    11 => MetricEvent::VerifyChapterClicked {
                        chapter_index: worker * 100 + seq,
                    },
                    _ => MetricEvent::ChapterMarkedDone {
                        chapter_index: worker * 100 + seq,
                        auto_passed: true,
                    },
                };
                state
                    .record_metric(Some(epoch), event)
                    .await
                    .expect("concurrent metric record");
            }
        }));
    }

    let file_workers = 8usize;
    let ops_per_file_worker = 5usize;
    for worker in 0..file_workers {
        let session = Arc::clone(&session);
        handles.push(tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                for step in 0..ops_per_file_worker {
                    let chapter = ChapterId(format!("concurrent_w{worker}_s{step}"));
                    session
                        .store()
                        .update_progress(|progress| {
                            progress.chapters.insert(
                                chapter.clone(),
                                ChapterProgress {
                                    status: ChapterStatus::Completed,
                                    completed_at: Some("2026-11-23T03:32:00Z".to_owned()),
                                    last_run_id: Some(RunId(format!("run_{worker}_{step}"))),
                                    auto_checks: BTreeMap::new(),
                                    self_checks: BTreeMap::new(),
                                },
                            );
                            Ok(())
                        })
                        .expect("concurrent progress write");

                    session
                        .store()
                        .append_chat_message(
                            chapter.clone(),
                            ChatMessage {
                                id: format!("msg_{worker}_{step}"),
                                role: ChatRole::User,
                                content: format!("concurrent question {worker}-{step}"),
                                teaching_mode: Some(TeachingMode::Direct),
                                created_at: "2026-11-23T03:32:00Z".to_owned(),
                            },
                        )
                        .expect("concurrent chat write");

                    session
                        .store()
                        .update_meta(|meta| {
                            meta.active_chapter_id = Some(chapter);
                            Ok(())
                        })
                        .expect("concurrent meta write");
                }
            })
            .await
            .expect("join blocking file worker");
        }));
    }

    for handle in handles {
        handle.await.expect("join worker");
    }

    let paths = session.paths().clone();
    let shutdown_report = state.close_workspace(epoch).await.unwrap();
    let elapsed_ms = started.elapsed().as_millis();

    assert_eq!(
        shutdown_report.metrics.total_persisted_events,
        expected_metric_total
    );

    let persisted_metrics: MetricsLog = AtomicJsonStore::new(paths.metrics_json())
        .read_valid()
        .unwrap();
    assert_eq!(persisted_metrics.events.len(), expected_metric_total);

    let persisted_progress: codelab_studio_lib::domain::workspace::ProgressState =
        AtomicJsonStore::new(paths.progress_json())
            .read_valid()
            .unwrap();
    assert_eq!(
        persisted_progress.chapters.len(),
        file_workers * ops_per_file_worker
    );

    let persisted_chat: codelab_studio_lib::domain::workspace::ChatHistory =
        AtomicJsonStore::new(paths.chat_json())
            .read_valid()
            .unwrap();
    assert_eq!(
        persisted_chat.chapters.len(),
        file_workers * ops_per_file_worker
    );

    let persisted_meta: WorkspaceMeta = AtomicJsonStore::new(paths.workspace_json())
        .read_valid()
        .unwrap();
    assert!(persisted_meta.active_chapter_id.is_some());

    let leftover_tmp: Vec<String> = list_codelab_entries(ws_dir.path())
        .into_iter()
        .filter(|name| name.contains(".tmp."))
        .collect();
    assert!(leftover_tmp.is_empty());

    assert_eq!(
        state
            .record_metric(Some(epoch), MetricEvent::FileTreeOpen { opened: false })
            .await
            .unwrap_err()
            .code,
        ErrorCode::WorkspaceNotOpen
    );

    let cap_ws = tempfile::tempdir().unwrap();
    let mut cap_params = make_open_params(cap_ws.path(), "go-goframe-practice");
    cap_params.metrics_config = MetricsWriterConfig {
        flush_interval: Duration::from_secs(2),
        batch_size: 500,
        max_events: MAX_METRICS_EVENTS,
        channel_capacity: 2048,
    };
    let cap_session = state.open_workspace(cap_params).await.unwrap();
    let cap_epoch = cap_session.epoch();

    let mut cap_tasks = Vec::new();
    for worker in 0..11u32 {
        let state = Arc::clone(&state);
        cap_tasks.push(tokio::spawn(async move {
            for seq in 0..500u32 {
                state
                    .record_metric(
                        Some(cap_epoch),
                        MetricEvent::VerifyChapterClicked {
                            chapter_index: worker * 1_000 + seq,
                        },
                    )
                    .await
                    .unwrap();
            }
        }));
    }
    for task in cap_tasks {
        task.await.unwrap();
    }
    let cap_paths = cap_session.paths().clone();
    let cap_shutdown = state.close_workspace(cap_epoch).await.unwrap();
    assert_eq!(
        cap_shutdown.metrics.total_persisted_events,
        MAX_METRICS_EVENTS
    );
    let cap_log: MetricsLog = AtomicJsonStore::new(cap_paths.metrics_json())
        .read_valid()
        .unwrap();
    assert_eq!(cap_log.events.len(), MAX_METRICS_EVENTS);

    println!(
        "[U1 VERIFY: CONCURRENT WRITES] metric_tasks=20x60 persisted_metrics={}/{} flushes={} progress_chapters={}/40 chat_chapters={}/40 elapsed_ms={} tmp_residue=0 cap_burst=11x500(5500)->retained={}",
        persisted_metrics.events.len(),
        expected_metric_total,
        shutdown_report.metrics.flush_count,
        persisted_progress.chapters.len(),
        persisted_chat.chapters.len(),
        elapsed_ms,
        cap_log.events.len()
    );
}

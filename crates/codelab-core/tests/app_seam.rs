use std::sync::{Arc, Mutex};

use codelab_core::{
    AccentPreset, App, AppCommand, AppConfig, AppEvent, AppResponse, CourseLanguage, ErrorCode,
    GlobalSettings, Locale, RecordRecentWorkspaceInput, ShutdownReport, StoreKind, ThemeMode,
    TrustSource, TrustSourceKind, UpdateSettingsPatch,
};
use tempfile::TempDir;

#[test]
fn cold_start_bootstrap_returns_default_settings_and_empty_collections() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");

    let response = app.execute(AppCommand::Bootstrap).expect("bootstrap");

    match response {
        AppResponse::Bootstrap(bootstrap) => {
            assert_eq!(
                bootstrap.settings,
                GlobalSettings {
                    theme: ThemeMode::Light,
                    accent: AccentPreset::Neutral,
                    code_font_size: 13.5,
                    locale: Locale::ZhCn,
                    tutor: Default::default(),
                }
            );
            assert!(bootstrap.recents.is_empty());
            assert!(bootstrap.trust_entries.is_empty());
            assert!(bootstrap.recovery_notices.is_empty());
        }
        other => panic!("expected Bootstrap response, got {other:?}"),
    }
}

#[test]
fn updating_settings_emits_event_and_persists_across_restart() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");
    let events = app.subscribe();

    let updated = app
        .execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
            theme: Some(ThemeMode::Dark),
            accent: Some(AccentPreset::Violet),
            code_font_size: Some(15.0),
            locale: Some(Locale::EnUs),
            ..Default::default()
        }))
        .expect("update settings");

    let expected = GlobalSettings {
        theme: ThemeMode::Dark,
        accent: AccentPreset::Violet,
        code_font_size: 15.0,
        locale: Locale::EnUs,
        tutor: Default::default(),
    };
    assert_eq!(updated, AppResponse::Settings(expected.clone()));
    assert_eq!(
        events.try_recv().expect("event"),
        AppEvent::SettingsUpdated(expected.clone())
    );

    // No leftover temp files after atomic rename
    let leftover_tmps: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
        .collect();
    assert!(
        leftover_tmps.is_empty(),
        "found temp files: {leftover_tmps:?}"
    );

    drop(app);

    let restarted = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("reopen app");
    assert_eq!(
        restarted
            .execute(AppCommand::GetSettings)
            .expect("get settings"),
        AppResponse::Settings(expected)
    );
}

#[test]
fn updating_settings_rejects_invalid_code_font_size_with_domain_error() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");

    for invalid_size in [10.0_f32, 24.0_f32, f32::NAN, f32::INFINITY] {
        let err = app
            .execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
                code_font_size: Some(invalid_size),
                ..Default::default()
            }))
            .expect_err("expected invalid font size error");
        assert_eq!(err.code, ErrorCode::InvalidCodeFontSize);
    }

    assert_eq!(
        app.execute(AppCommand::GetSettings).expect("get settings"),
        AppResponse::Settings(GlobalSettings::default())
    );
}

#[test]
fn recent_workspaces_deduplicate_order_persist_and_remove() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");
    let events = app.subscribe();

    let first_ws = dir.path().join("ws-go");
    let second_ws = dir.path().join("ws-ts");

    app.execute(AppCommand::RecordRecentWorkspace(
        RecordRecentWorkspaceInput {
            workspace_path: first_ws.to_string_lossy().into_owned(),
            course_id: "go-goframe".into(),
            course_title: "Go 与 GoFrame 开发实践".into(),
            language: CourseLanguage::Go,
            completed_exercises: 4,
            total_exercises: 10,
        },
    ))
    .expect("record first");

    let _ = events.try_recv().expect("first recents event");

    app.execute(AppCommand::RecordRecentWorkspace(
        RecordRecentWorkspaceInput {
            workspace_path: second_ws.to_string_lossy().into_owned(),
            course_id: "ts-pipeline".into(),
            course_title: "TypeScript 数据流水线实战".into(),
            language: CourseLanguage::JsTs,
            completed_exercises: 2,
            total_exercises: 8,
        },
    ))
    .expect("record second");

    let _ = events.try_recv().expect("second recents event");

    // Re-recording the first workspace with updated progress promotes it to index 0
    app.execute(AppCommand::RecordRecentWorkspace(
        RecordRecentWorkspaceInput {
            workspace_path: first_ws.to_string_lossy().into_owned(),
            course_id: "go-goframe".into(),
            course_title: "Go 与 GoFrame 开发实践".into(),
            language: CourseLanguage::Go,
            completed_exercises: 5,
            total_exercises: 10,
        },
    ))
    .expect("promote first");

    match events.try_recv().expect("third recents event") {
        AppEvent::RecentsUpdated(recents) => {
            assert_eq!(recents.len(), 2);
            assert_eq!(recents[0].course_id, "go-goframe");
            assert_eq!(recents[0].completed_exercises, 5);
            assert_eq!(recents[1].course_id, "ts-pipeline");
        }
        other => panic!("expected RecentsUpdated, got {other:?}"),
    }

    drop(app);

    let restarted = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("reopen");

    match restarted
        .execute(AppCommand::ListRecents)
        .expect("list recents")
    {
        AppResponse::Recents(recents) => {
            assert_eq!(recents.len(), 2);
            assert_eq!(recents[0].course_id, "go-goframe");
            assert_eq!(recents[0].completed_exercises, 5);
        }
        other => panic!("expected Recents, got {other:?}"),
    }

    match restarted
        .execute(AppCommand::RemoveRecentWorkspace {
            workspace_path: first_ws.to_string_lossy().into_owned(),
        })
        .expect("remove recent")
    {
        AppResponse::Recents(recents) => {
            assert_eq!(recents.len(), 1);
            assert_eq!(recents[0].course_id, "ts-pipeline");
        }
        other => panic!("expected Recents, got {other:?}"),
    }
}

#[test]
fn recording_invalid_recent_workspace_returns_structured_domain_errors() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");

    let rel_err = app
        .execute(AppCommand::RecordRecentWorkspace(
            RecordRecentWorkspaceInput {
                workspace_path: "relative/path".into(),
                course_id: "go-1".into(),
                course_title: "Go".into(),
                language: CourseLanguage::Go,
                completed_exercises: 0,
                total_exercises: 5,
            },
        ))
        .expect_err("relative path must fail");
    assert_eq!(rel_err.code, ErrorCode::InvalidWorkspacePath);

    let meta_err = app
        .execute(AppCommand::RecordRecentWorkspace(
            RecordRecentWorkspaceInput {
                workspace_path: dir.path().join("ws").to_string_lossy().into_owned(),
                course_id: "   ".into(),
                course_title: "Go".into(),
                language: CourseLanguage::Go,
                completed_exercises: 0,
                total_exercises: 5,
            },
        ))
        .expect_err("blank course id must fail");
    assert_eq!(meta_err.code, ErrorCode::InvalidCourseMetadata);

    let count_err = app
        .execute(AppCommand::RecordRecentWorkspace(
            RecordRecentWorkspaceInput {
                workspace_path: dir.path().join("ws").to_string_lossy().into_owned(),
                course_id: "go-1".into(),
                course_title: "Go".into(),
                language: CourseLanguage::Go,
                completed_exercises: 6,
                total_exercises: 5,
            },
        ))
        .expect_err("completed > total must fail");
    assert_eq!(count_err.code, ErrorCode::InvalidProgressCount);
}

#[test]
fn trust_registry_normalizes_sources_persists_and_revokes() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");
    let events = app.subscribe();

    let course_dir = dir.path().join("external-course");
    std::fs::create_dir_all(&course_dir).expect("create course dir");

    // Untrusted by default
    match app
        .execute(AppCommand::CheckTrust(TrustSource {
            source_kind: TrustSourceKind::GitRepo,
            source: "https://GitHub.com/faiz-gear/sample-pack.git/".into(),
        }))
        .expect("check untrusted")
    {
        AppResponse::TrustState(state) => {
            assert!(!state.trusted);
            assert_eq!(
                state.normalized_source,
                "https://github.com/faiz-gear/sample-pack"
            );
        }
        other => panic!("expected TrustState, got {other:?}"),
    }

    // Grant git trust with uppercase host + .git suffix
    app.execute(AppCommand::GrantTrust(TrustSource {
        source_kind: TrustSourceKind::GitRepo,
        source: "https://GitHub.com/faiz-gear/sample-pack.git".into(),
    }))
    .expect("grant git trust");
    assert!(matches!(
        events.try_recv().expect("trust event"),
        AppEvent::TrustUpdated(entries) if entries.len() == 1
    ));

    // Grant local path trust with unnormalized segment
    let messy_local = format!("{}/./", course_dir.display());
    app.execute(AppCommand::GrantTrust(TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: messy_local,
    }))
    .expect("grant local trust");
    let _ = events.try_recv().expect("second trust event");

    drop(app);

    let restarted = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("reopen app");

    match restarted
        .execute(AppCommand::CheckTrust(TrustSource {
            source_kind: TrustSourceKind::GitRepo,
            source: "https://github.com/faiz-gear/sample-pack/".into(),
        }))
        .expect("check git trust after restart")
    {
        AppResponse::TrustState(state) => assert!(state.trusted),
        other => panic!("expected TrustState, got {other:?}"),
    }

    match restarted
        .execute(AppCommand::CheckTrust(TrustSource {
            source_kind: TrustSourceKind::LocalPath,
            source: course_dir.to_string_lossy().into_owned(),
        }))
        .expect("check local trust after restart")
    {
        AppResponse::TrustState(state) => assert!(state.trusted),
        other => panic!("expected TrustState, got {other:?}"),
    }

    restarted
        .execute(AppCommand::RevokeTrust(TrustSource {
            source_kind: TrustSourceKind::GitRepo,
            source: "https://github.com/faiz-gear/sample-pack".into(),
        }))
        .expect("revoke trust");

    match restarted
        .execute(AppCommand::ListTrustEntries)
        .expect("list trust")
    {
        AppResponse::TrustEntries(entries) => {
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].source_kind, TrustSourceKind::LocalPath);
        }
        other => panic!("expected TrustEntries, got {other:?}"),
    }
}

#[test]
fn trust_commands_reject_invalid_sources_with_domain_error() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");

    let bad_local = app
        .execute(AppCommand::GrantTrust(TrustSource {
            source_kind: TrustSourceKind::LocalPath,
            source: "relative/course".into(),
        }))
        .expect_err("relative local path must fail");
    assert_eq!(bad_local.code, ErrorCode::InvalidTrustSource);

    let bad_git = app
        .execute(AppCommand::GrantTrust(TrustSource {
            source_kind: TrustSourceKind::GitRepo,
            source: "not-a-git-url".into(),
        }))
        .expect_err("invalid git url must fail");
    assert_eq!(bad_git.code, ErrorCode::InvalidTrustSource);
}

#[test]
fn corrupted_global_stores_recover_including_invalid_utf8_bytes() {
    let dir = TempDir::new().expect("temp dir");

    // 1. Non-UTF-8 binary corruption for settings
    std::fs::write(
        dir.path().join("settings.json"),
        [0xff_u8, 0xfe, 0x00, 0x80],
    )
    .expect("write non-utf8 settings");

    // 2. Truncated / broken JSON for recents
    std::fs::write(dir.path().join("recents.json"), "{broken json").expect("write corrupt recents");

    // 3. Unsupported schema version for trust
    std::fs::write(
        dir.path().join("trust.json"),
        r#"{"schemaVersion":999,"data":[]}"#,
    )
    .expect("write bad version trust");

    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open with recovery");

    match app.execute(AppCommand::Bootstrap).expect("bootstrap") {
        AppResponse::Bootstrap(bootstrap) => {
            assert_eq!(bootstrap.settings, GlobalSettings::default());
            assert!(bootstrap.recents.is_empty());
            assert!(bootstrap.trust_entries.is_empty());
            assert_eq!(bootstrap.recovery_notices.len(), 3);

            let recovered_stores: Vec<StoreKind> =
                bootstrap.recovery_notices.iter().map(|n| n.store).collect();
            assert_eq!(
                recovered_stores,
                vec![StoreKind::Settings, StoreKind::Recents, StoreKind::Trust]
            );

            for notice in &bootstrap.recovery_notices {
                assert_eq!(notice.error_code, ErrorCode::StorageCorrupted);
                let backup = notice.backup_path.as_deref().expect("backup path");
                assert!(
                    std::path::Path::new(backup).exists(),
                    "backup missing: {backup}"
                );
            }
        }
        other => panic!("expected Bootstrap, got {other:?}"),
    }

    drop(app);

    // Out-of-bounds domain value inside valid JSON envelope also recovers
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"schemaVersion":1,"data":{"theme":"dark","accent":"blue","codeFontSize":99.0,"locale":"en-US"}}"#,
    )
    .expect("write out-of-range settings");

    let recovered_again = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("reopen with domain validation recovery");

    match recovered_again
        .execute(AppCommand::Bootstrap)
        .expect("bootstrap")
    {
        AppResponse::Bootstrap(bootstrap) => {
            assert_eq!(bootstrap.settings, GlobalSettings::default());
            assert_eq!(bootstrap.recovery_notices.len(), 1);
            assert_eq!(bootstrap.recovery_notices[0].store, StoreKind::Settings);
        }
        other => panic!("expected Bootstrap, got {other:?}"),
    }

    drop(recovered_again);

    // Subsequent clean restart yields zero recovery notices
    let clean = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("reopen clean");

    match clean.execute(AppCommand::Bootstrap).expect("bootstrap") {
        AppResponse::Bootstrap(bootstrap) => {
            assert!(bootstrap.recovery_notices.is_empty());
        }
        other => panic!("expected Bootstrap, got {other:?}"),
    }
}

#[test]
fn shutdown_executes_cleanup_hooks_once_and_emits_event() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");
    let events = app.subscribe();

    let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let order1 = Arc::clone(&order);
    app.register_cleanup_hook(
        "pty",
        Box::new(move || {
            order1.lock().unwrap().push("pty");
            Ok(())
        }),
    );
    let order2 = Arc::clone(&order);
    app.register_cleanup_hook(
        "daemon",
        Box::new(move || {
            order2.lock().unwrap().push("daemon");
            Ok(())
        }),
    );

    let first = app.execute(AppCommand::Shutdown).expect("shutdown");
    assert_eq!(
        first,
        AppResponse::Shutdown(ShutdownReport {
            already_shut_down: false,
            hooks_executed: 2,
        })
    );
    assert_eq!(*order.lock().unwrap(), vec!["daemon", "pty"]);
    assert_eq!(
        events.try_recv().expect("shutdown event"),
        AppEvent::ShutdownCompleted(ShutdownReport {
            already_shut_down: false,
            hooks_executed: 2,
        })
    );

    let second = app.shutdown().expect("idempotent shutdown");
    assert_eq!(
        second,
        ShutdownReport {
            already_shut_down: true,
            hooks_executed: 0,
        }
    );
    assert_eq!(*order.lock().unwrap(), vec!["daemon", "pty"]);
}

#[test]
fn shutdown_runs_all_hooks_even_if_one_fails_and_returns_domain_error() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");

    let ran_first = Arc::new(Mutex::new(false));
    let ran_first_clone = Arc::clone(&ran_first);
    app.register_cleanup_hook(
        "first",
        Box::new(move || {
            *ran_first_clone.lock().unwrap() = true;
            Ok(())
        }),
    );
    app.register_cleanup_hook(
        "failing",
        Box::new(|| Err("simulated cleanup failure".into())),
    );

    let err = app.shutdown().expect_err("expected ShutdownFailed");
    assert_eq!(err.code, ErrorCode::ShutdownFailed);
    assert!(*ran_first.lock().unwrap(), "remaining hook must still run");
}

#[test]
fn ipc_wire_json_roundtrips_commands_responses_and_domain_errors() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");

    let raw_cmd = r#"{"type":"updateSettings","payload":{"theme":"dark","accent":"blue","codeFontSize":12.5,"locale":"en-US"}}"#;
    let cmd: AppCommand = serde_json::from_str(raw_cmd).expect("parse command JSON");
    let response = app.execute(cmd).expect("execute");
    let serialized = serde_json::to_value(&response).expect("serialize response");

    assert_eq!(serialized["type"], "settings");
    assert_eq!(serialized["payload"]["theme"], "dark");
    assert_eq!(serialized["payload"]["accent"], "blue");
    assert_eq!(serialized["payload"]["codeFontSize"], 12.5);
    assert_eq!(serialized["payload"]["locale"], "en-US");

    let bad_cmd: AppCommand =
        serde_json::from_str(r#"{"type":"updateSettings","payload":{"codeFontSize":4.0}}"#)
            .expect("parse invalid font command");
    let err = app.execute(bad_cmd).expect_err("expected error");
    let err_json = serde_json::to_value(&err).expect("serialize DomainError");
    assert_eq!(err_json["code"], "INVALID_CODE_FONT_SIZE");
    assert_eq!(err_json["details"]["min"], "11");
    assert_eq!(err_json["details"]["max"], "20");
}

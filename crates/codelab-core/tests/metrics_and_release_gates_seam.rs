use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use codelab_core::{
    App, AppCommand, AppConfig, AppEvent, AppResponse, BackendEvent, BackendEventSink,
    BackendPermissionPrompt, CourseLanguage, DomainError, ErrorCode, EventSubscription,
    ExerciseTarget, ExternalIde, HostProbeResult, HostTool, HostToolchainProbe, LocalMetricEvent,
    MetricEventName, MetricRiskSource, MetricSelectionAction, MetricSelectionZone,
    OpenCourseRequest, PermissionPolicy, PreflightReport, RecordLocalMetric,
    RespondTutorPermission, RunChecks, RunCommandBlock, RuntimeAdapters, RuntimeArtifact,
    RuntimeDownloadAdapter, RuntimePreference, SendTutorPrompt, TeachingStyle, TerminalRead,
    TrustSource, TrustSourceKind, TutorBackend, TutorBackendSession, TutorHandshakeResult,
    TutorLaunchSpec, TutorPermissionKind, TutorSettingsPatch, TutorTurnInput, UpdateSettingsPatch,
    WorkspaceTerminalRequest,
};
use tempfile::TempDir;

struct AllMissingHost;

impl HostToolchainProbe for AllMissingHost {
    fn probe(&self, _tool: HostTool, _timeout: Duration) -> HostProbeResult {
        HostProbeResult::Missing
    }
}

struct FailingDownloader;

impl RuntimeDownloadAdapter for FailingDownloader {
    fn download(
        &self,
        _artifact: &RuntimeArtifact,
        _destination: &mut dyn Write,
        _progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), DomainError> {
        Err(DomainError::new(ErrorCode::RuntimeDownloadFailed))
    }
}

struct MetricFakeTutorBackend;

struct MetricFakeTutorSession {
    sink: BackendEventSink,
}

impl TutorBackend for MetricFakeTutorBackend {
    fn handshake(&self, spec: &TutorLaunchSpec) -> Result<TutorHandshakeResult, DomainError> {
        Ok(TutorHandshakeResult {
            preset: spec.preset,
            command: spec.command.clone(),
            protocol_version: Some("1".into()),
            agent_name: Some("Metric Fake Agent".into()),
            agent_version: Some("1.0.0".into()),
        })
    }

    fn start_session(
        &self,
        _spec: &TutorLaunchSpec,
        sink: BackendEventSink,
    ) -> Result<Box<dyn TutorBackendSession>, DomainError> {
        Ok(Box::new(MetricFakeTutorSession { sink }))
    }
}

impl TutorBackendSession for MetricFakeTutorSession {
    fn agent_name(&self) -> Option<String> {
        Some("Metric Fake Agent".into())
    }

    fn send_turn(&self, input: TutorTurnInput) -> Result<(), DomainError> {
        (self.sink)(BackendEvent::PermissionRequested {
            turn_id: input.turn_id.clone(),
            request: BackendPermissionPrompt {
                request_id: "perm-write-canary".into(),
                kind: TutorPermissionKind::Write,
                title: "Write CANARY_FILE_PATH".into(),
                target_path: Some("CANARY_FILE_PATH.txt".into()),
                target_paths: vec!["CANARY_FILE_PATH.txt".into()],
                command: None,
                raw_input: None,
                is_fs_method: false,
                write_content: Some("CANARY_SOURCE_CONTENT".into()),
            },
        });
        Ok(())
    }

    fn cancel_turn(&self, turn_id: &str) -> Result<(), DomainError> {
        (self.sink)(BackendEvent::TurnCancelled {
            turn_id: turn_id.to_owned(),
        });
        Ok(())
    }

    fn respond_permission(
        &self,
        _request_id: &str,
        _allowed: bool,
        _fs_read_result: Option<Result<String, String>>,
        _fs_write_result: Option<Result<(), String>>,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    fn shutdown(&self) {}
}

fn wait_for_preflight(events: &EventSubscription, matches: impl Fn(&PreflightReport) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "timed out waiting for preflight");
        if let AppEvent::PreflightUpdated(report) =
            events.recv_timeout(remaining).expect("preflight event")
            && matches(&report)
        {
            return;
        }
    }
}

fn wait_for_verification_done(events: &EventSubscription, workspace_id: u64) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "timed out waiting for verification");
        if let AppEvent::ProgressUpdated(progress) =
            events.recv_timeout(remaining).expect("progress event")
            && progress.workspace_id == workspace_id
            && progress.running_chapter_id.is_none()
        {
            return;
        }
    }
}

fn write_canary_course_pack(pack_dir: &Path) {
    fs::create_dir_all(pack_dir.join("starter")).unwrap();
    fs::write(
        pack_dir.join("codelab.json"),
        r#"{
  "schemaVersion": "1.0",
  "courseId": "metrics-canary-pack",
  "title": "Metrics Canary Pack",
  "description": "Local pack verifying all 13 metrics and sanitization",
  "language": "Go",
  "entryMarkdown": "README.md",
  "starterDir": "starter",
  "checksFile": ".codelab-checks.json"
}"#,
    )
    .unwrap();
    fs::write(
        pack_dir.join("starter/CANARY_FILE_PATH.txt"),
        "CANARY_SOURCE_CONTENT\n",
    )
    .unwrap();
    fs::write(
        pack_dir.join("README.md"),
        r#"# Metrics Canary Pack

## 练习 1：Safe & Risky Commands {#ex-1}

### 1. 运行命令

```bash mode="oneshot"
echo CANARY_RAW_CMD_PAYLOAD
```

```bash mode="oneshot"
rm -rf /tmp/CANARY_HIGH_RISK_CMD_PAYLOAD
```

### 2. 修改代码

修改 `CANARY_FILE_PATH.txt`。

### 3. 验收清单

- 运行验收检查。
"#,
    )
    .unwrap();
    fs::write(
        pack_dir.join(".codelab-checks.json"),
        r#"{
  "schemaVersion": "1.0",
  "chapters": [
    {
      "chapterId": "ex-1",
      "checks": [
        {
          "id": "c1",
          "title": "Verify echo succeeds",
          "type": "command_test",
          "command": "echo CANARY_CHECK_CMD_PAYLOAD",
          "expectExitCode": 0
        }
      ]
    }
  ]
}"#,
    )
    .unwrap();
}

#[test]
fn preferred_ide_persists_in_global_settings_and_emits_update_event() {
    let dir = TempDir::new().unwrap();
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .unwrap();
    let events = app.subscribe();

    let AppResponse::Settings(initial) = app.execute(AppCommand::GetSettings).unwrap() else {
        panic!("expected settings");
    };
    assert_eq!(initial.preferred_ide, ExternalIde::VsCode);

    let AppResponse::Settings(updated) = app
        .execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
            preferred_ide: Some(ExternalIde::Cursor),
            ..Default::default()
        }))
        .unwrap()
    else {
        panic!("expected updated settings");
    };
    assert_eq!(updated.preferred_ide, ExternalIde::Cursor);
    assert!(matches!(
        events.try_recv().unwrap(),
        AppEvent::SettingsUpdated(settings) if settings.preferred_ide == ExternalIde::Cursor
    ));

    drop(app);
    let reopened = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .unwrap();
    let AppResponse::Settings(persisted) = reopened.execute(AppCommand::GetSettings).unwrap()
    else {
        panic!("expected persisted settings");
    };
    assert_eq!(persisted.preferred_ide, ExternalIde::Cursor);
}

#[test]
fn all_13_local_metrics_persist_atomically_to_workspace_with_strict_sanitization() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");
    let pack_dir = temp.path().join("CANARY_COURSE_PATH");
    let workspace_dir = temp.path().join("CANARY_WORKSPACE_PATH");
    fs::create_dir_all(&workspace_dir).unwrap();
    write_canary_course_pack(&pack_dir);

    let app = App::open_with_test_adapters(
        AppConfig {
            data_dir: data_dir.clone(),
        },
        RuntimeAdapters {
            host: Arc::new(AllMissingHost),
            downloads: Arc::new(FailingDownloader),
        },
        Arc::new(MetricFakeTutorBackend),
    )
    .unwrap();
    let events = app.subscribe();

    // 1. app_launched queued on App::open, flushed into bound workspace.
    app.execute(AppCommand::Bootstrap).unwrap();

    // 2. preflight_executed + 3. runtime_installed (fails deterministically via FailingDownloader).
    app.execute(AppCommand::RefreshPreflight).unwrap();
    wait_for_preflight(&events, |report| report.checked && !report.probing);
    app.execute(AppCommand::SetRuntimePreference(RuntimePreference {
        language: CourseLanguage::Go,
        prefer_managed: true,
    }))
    .unwrap();
    app.execute(AppCommand::InstallManagedRuntime {
        language: CourseLanguage::Go,
    })
    .unwrap();
    wait_for_preflight(&events, |report| {
        report
            .languages
            .iter()
            .find(|row| row.language == CourseLanguage::Go)
            .is_some_and(|row| {
                matches!(
                    row.install,
                    Some(codelab_core::InstallProgress::Failed { .. })
                )
            })
    });

    // Open untrusted course -> metrics commands must be blocked by Trust Gate.
    let pack_str = pack_dir.to_string_lossy().into_owned();
    app.execute(AppCommand::OpenCourse(OpenCourseRequest::LocalPack {
        path: pack_str.clone(),
    }))
    .unwrap();
    let err = app
        .execute(AppCommand::RecordLocalMetric(RecordLocalMetric {
            workspace_id: 1,
            event: LocalMetricEvent::FileTreeOpen,
        }))
        .expect_err("untrusted course must block local metrics");
    assert_eq!(err.code, ErrorCode::CourseUntrusted);

    // Grant trust & bind workspace -> flushes app_launched, preflight_executed, runtime_installed, course_opened.
    app.execute(AppCommand::GrantTrust(TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: pack_str,
    }))
    .unwrap();
    let AppResponse::WorkspaceBound(summary) = app
        .execute(AppCommand::BindWorkspace {
            path: workspace_dir.to_string_lossy().into_owned(),
        })
        .unwrap()
    else {
        panic!("expected WorkspaceBound");
    };
    let ws_id = summary.id;

    // 4. cmd_block_clicked + 5. cmd_finished (safe command).
    let AppResponse::CommandStarted(started) = app
        .execute(AppCommand::RunCommandBlock(RunCommandBlock {
            workspace_id: ws_id,
            command_id: "ex-1:cmd-1".into(),
            restart_terminal_id: None,
            confirmation: None,
            drafts: Vec::new(),
        }))
        .unwrap()
    else {
        panic!("expected CommandStarted");
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let AppResponse::Terminals(updates) = app
            .execute(AppCommand::ReadTerminals(TerminalRead {
                workspace_id: ws_id,
                terminal_id: Some(started.terminal.id),
                cursor: 0,
                include_recent_errors: Some(false),
            }))
            .unwrap()
        else {
            panic!("expected Terminals");
        };
        if updates
            .terminals
            .iter()
            .find(|t| t.id == started.terminal.id)
            .is_some_and(|t| !t.running)
        {
            break;
        }
        assert!(Instant::now() < deadline, "safe command did not finish");
        std::thread::sleep(Duration::from_millis(20));
    }

    // 6. cmd_risk_modal_action (cancel + confirm).
    let AppResponse::CommandConfirmationRequired(_prompt) = app
        .execute(AppCommand::RunCommandBlock(RunCommandBlock {
            workspace_id: ws_id,
            command_id: "ex-1:cmd-2".into(),
            restart_terminal_id: None,
            confirmation: None,
            drafts: Vec::new(),
        }))
        .unwrap()
    else {
        panic!("expected CommandConfirmationRequired");
    };
    app.execute(AppCommand::RecordLocalMetric(RecordLocalMetric {
        workspace_id: ws_id,
        event: LocalMetricEvent::CmdRiskModalCancelled {
            source: MetricRiskSource::CommandBlock,
        },
    }))
    .unwrap();

    // 7. file_tree_open + 8. selection_ai_clicked.
    app.execute(AppCommand::RecordLocalMetric(RecordLocalMetric {
        workspace_id: ws_id,
        event: LocalMetricEvent::FileTreeOpen,
    }))
    .unwrap();
    app.execute(AppCommand::RecordLocalMetric(RecordLocalMetric {
        workspace_id: ws_id,
        event: LocalMetricEvent::SelectionAiClicked {
            zone: MetricSelectionZone::Editor,
            action: MetricSelectionAction::ExplainCode,
        },
    }))
    .unwrap();

    // 9. ai_mode_switched.
    app.execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
        tutor: Some(TutorSettingsPatch {
            teaching_style: Some(TeachingStyle::Direct),
            permission_policy: Some(PermissionPolicy::GuidedReadonly),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .unwrap();

    // 10. acp_permission_decision.
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id: ws_id,
        prompt: "CANARY_CHAT_TRANSCRIPT_SECRET please inspect".into(),
        teaching_style: Some(TeachingStyle::Direct),
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let AppResponse::TutorSession(session) = app
        .execute(AppCommand::GetTutorSession(
            codelab_core::WorkspaceTutorRequest {
                workspace_id: ws_id,
            },
        ))
        .unwrap()
    else {
        panic!("expected TutorSession");
    };
    let perm_id = session
        .messages
        .iter()
        .flat_map(|m| &m.permissions)
        .find(|p| p.id == "perm-write-canary")
        .expect("pending permission")
        .id
        .clone();
    app.execute(AppCommand::RespondTutorPermission(RespondTutorPermission {
        workspace_id: ws_id,
        request_id: perm_id,
        allow: false,
    }))
    .unwrap();

    // 11. verify_chapter_clicked + 12. chapter_marked_done.
    app.execute(AppCommand::RunChecks(RunChecks {
        target: ExerciseTarget {
            workspace_id: ws_id,
            chapter_id: "ex-1".into(),
        },
        drafts: Vec::new(),
        confirmation: None,
    }))
    .unwrap();
    wait_for_verification_done(&events, ws_id);

    // Fetch metrics via App interface and verify all 13 event types exist.
    let AppResponse::WorkspaceMetrics(metrics) = app
        .execute(AppCommand::GetWorkspaceMetrics(WorkspaceTerminalRequest {
            workspace_id: ws_id,
        }))
        .unwrap()
    else {
        panic!("expected WorkspaceMetrics");
    };

    let all_13 = [
        MetricEventName::AppLaunched,
        MetricEventName::CourseOpened,
        MetricEventName::PreflightExecuted,
        MetricEventName::RuntimeInstalled,
        MetricEventName::CmdBlockClicked,
        MetricEventName::CmdRiskModalAction,
        MetricEventName::CmdFinished,
        MetricEventName::FileTreeOpen,
        MetricEventName::SelectionAiClicked,
        MetricEventName::AiModeSwitched,
        MetricEventName::AcpPermissionDecision,
        MetricEventName::VerifyChapterClicked,
        MetricEventName::ChapterMarkedDone,
    ];
    for name in all_13 {
        assert!(
            metrics.counts.get(&name).copied().unwrap_or(0) >= 1,
            "missing metric count for {name:?}; counts={:?}",
            metrics.counts
        );
    }
    let recorded_names: BTreeSet<_> = metrics.events.iter().map(|e| e.event).collect();
    for name in all_13 {
        assert!(
            recorded_names.contains(&name),
            "missing metric record for {name:?}"
        );
    }

    // Verify atomic file on disk under <workspace>/.codelab/metrics.json (no global metrics file, no leftover tmp).
    assert!(!data_dir.join("metrics.json").exists());
    let metrics_path = workspace_dir.join(".codelab/metrics.json");
    let raw_disk = fs::read_to_string(&metrics_path).expect("read metrics.json");
    for expected_wire_name in [
        "app_launched",
        "course_opened",
        "preflight_executed",
        "runtime_installed",
        "cmd_block_clicked",
        "cmd_risk_modal_action",
        "cmd_finished",
        "file_tree_open",
        "selection_ai_clicked",
        "ai_mode_switched",
        "acp_permission_decision",
        "verify_chapter_clicked",
        "chapter_marked_done",
    ] {
        assert!(
            raw_disk.contains(expected_wire_name),
            "metrics.json missing wire event {expected_wire_name}"
        );
    }

    // Strict sanitization check: ZERO source code, raw command text, file paths, or chat transcripts!
    for forbidden in [
        "CANARY_SOURCE_CONTENT",
        "CANARY_RAW_CMD_PAYLOAD",
        "CANARY_HIGH_RISK_CMD_PAYLOAD",
        "CANARY_CHECK_CMD_PAYLOAD",
        "CANARY_FILE_PATH",
        "CANARY_COURSE_PATH",
        "CANARY_WORKSPACE_PATH",
        "CANARY_CHAT_TRANSCRIPT_SECRET",
    ] {
        assert!(
            !raw_disk.contains(forbidden),
            "metrics.json leaked sensitive canary {forbidden}:\n{raw_disk}"
        );
    }
}

#[test]
fn performance_budgets_hold_for_cold_start_1000_line_markdown_and_workspace_watcher_sync() {
    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");

    // 1. Cold start (App::open + Bootstrap) <= 2.0s
    let cold_start = Instant::now();
    let app = App::open(AppConfig { data_dir }).unwrap();
    let _ = app.execute(AppCommand::Bootstrap).unwrap();
    let cold_elapsed = cold_start.elapsed();
    assert!(
        cold_elapsed <= Duration::from_secs(2),
        "cold start took {:?} (> 2.0s budget)",
        cold_elapsed
    );

    // 2. 1,000-line Markdown parsing <= 150ms
    let mut md = String::from("# Performance Benchmark Course\n\nIntro overview paragraph.\n\n");
    for chapter_idx in 1..=20 {
        md.push_str(&format!(
            "## 练习 {chapter_idx}：第 {chapter_idx} 节实践目标\n\n"
        ));
        md.push_str("### 1. 启动服务\n\n");
        md.push_str("```bash cwd=\"task-api\" mode=\"oneshot\"\ngo test ./...\n```\n\n");
        md.push_str("### 2. 修改代码\n\n");
        for line_idx in 0..35 {
            md.push_str(&format!(
                "- Step detail {chapter_idx}.{line_idx}: update `handler.go` and verify edge case #{line_idx}.\n"
            ));
        }
        md.push_str("\n### 3. 验收清单\n\n- 确认输出符合预期并无竞态。\n\n");
    }
    let line_count = md.lines().count();
    assert!(
        line_count >= 1000,
        "expected >= 1000 markdown lines, got {line_count}"
    );
    let md_path = temp.path().join("large-course.md");
    fs::write(&md_path, &md).unwrap();

    let parse_start = Instant::now();
    let AppResponse::LoadedCourse(course) = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: md_path.to_string_lossy().into_owned(),
        }))
        .unwrap()
    else {
        panic!("expected LoadedCourse");
    };
    let parse_elapsed = parse_start.elapsed();
    assert_eq!(course.chapters.len(), 20);
    assert!(
        parse_elapsed <= Duration::from_millis(150),
        "1,000-line Markdown parsing took {:?} (> 150ms budget)",
        parse_elapsed
    );

    // 3. Workspace file tree sync <= 500ms
    app.execute(AppCommand::GrantTrust(TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: md_path.to_string_lossy().into_owned(),
    }))
    .unwrap();
    let ws_dir = temp.path().join("ws-sync");
    fs::create_dir_all(&ws_dir).unwrap();
    let events = app.subscribe();
    let AppResponse::WorkspaceBound(summary) = app
        .execute(AppCommand::BindWorkspace {
            path: ws_dir.to_string_lossy().into_owned(),
        })
        .unwrap()
    else {
        panic!("expected WorkspaceBound");
    };
    // Let watcher register before writing test file.
    std::thread::sleep(Duration::from_millis(80));
    let sync_start = Instant::now();
    fs::write(ws_dir.join("synced.go"), "package main\n").unwrap();
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut synced = false;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if let Ok(AppEvent::WorkspaceFilesChanged(changed)) = events.recv_timeout(remaining)
            && changed.workspace_id == summary.id
            && changed.paths.iter().any(|p| p.contains("synced.go"))
        {
            synced = true;
            break;
        }
    }
    let sync_elapsed = sync_start.elapsed();
    assert!(
        synced && sync_elapsed <= Duration::from_millis(500),
        "workspace file tree sync exceeded 500ms budget (synced={synced}, elapsed={sync_elapsed:?})"
    );

    // 4. Resident memory (excl. LSP) <= 350 MB
    #[cfg(unix)]
    {
        let output = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .expect("query resident memory via ps");
        assert!(output.status.success(), "ps -o rss= failed");
        let rss_kb: u64 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .expect("parse RSS kB");
        let rss_mb = rss_kb as f64 / 1024.0;
        assert!(
            rss_kb <= 350 * 1024,
            "resident memory {rss_mb:.1} MB exceeded 350 MB budget"
        );
    }
}

#[cfg(unix)]
#[test]
fn cross_module_workspace_switch_and_shutdown_leave_zero_leaked_child_processes() {
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    fn pid_alive(pid: i32) -> bool {
        if pid <= 1 {
            return false;
        }
        let res = unsafe { libc::kill(pid, 0) };
        res == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    fn wait_pid_dead(pid: i32, label: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if !pid_alive(pid) {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!("leaked {label} process pid={pid} still alive after teardown");
    }

    fn wait_file_text(path: &Path) -> String {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            if let Ok(text) = fs::read_to_string(path)
                && !text.trim().is_empty()
            {
                return text;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!("timed out waiting for marker file {}", path.display());
    }

    let temp = TempDir::new().unwrap();
    let data_dir = temp.path().join("data");

    // Fake gopls LSP script that spawns a sleeping grandchild and records both PIDs.
    let lsp_bin = temp.path().join("fake-gopls");
    fs::write(
        &lsp_bin,
        r#"#!/usr/bin/env python3
import json, os, subprocess, sys
from pathlib import Path
child = subprocess.Popen(['/bin/sleep', '120'], start_new_session=True)
Path('lsp-pids.txt').write_text(f'{os.getpid()} {child.pid}')
def send(msg):
    body = json.dumps({'jsonrpc': '2.0', **msg}).encode()
    sys.stdout.buffer.write(f'Content-Length: {len(body)}\r\n\r\n'.encode() + body)
    sys.stdout.buffer.flush()
while header := sys.stdin.buffer.readline():
    size = int(header.split(b':')[1])
    sys.stdin.buffer.readline()
    msg = json.loads(sys.stdin.buffer.read(size))
    if msg.get('method') == 'initialize':
        send({'id': msg['id'], 'result': {'capabilities': {'textDocumentSync': 1}}})
"#,
    )
    .unwrap();
    fs::set_permissions(&lsp_bin, fs::Permissions::from_mode(0o755)).unwrap();

    // Fake ACP stdio script that spawns a sleeping grandchild and records both PIDs.
    let acp_script = temp.path().join("fake-acp.py");
    fs::write(
        &acp_script,
        r#"#!/usr/bin/env python3
import json, os, subprocess, sys
from pathlib import Path
child = subprocess.Popen(['/bin/sleep', '120'])
Path('acp-pids.txt').write_text(f'{os.getpid()} {child.pid}')
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    msg = json.loads(line)
    method = msg.get('method')
    if method == 'initialize':
        print(json.dumps({'jsonrpc': '2.0', 'id': msg['id'], 'result': {'protocolVersion': 1, 'agentInfo': {'name': 'LeakCheck', 'version': '1.0'}}}), flush=True)
    elif method == 'session/new':
        print(json.dumps({'jsonrpc': '2.0', 'id': msg['id'], 'result': {'sessionId': 'sess-1'}}), flush=True)
    elif method == 'session/prompt':
        print(json.dumps({'jsonrpc': '2.0', 'id': msg['id'], 'result': {'stopReason': 'end_turn'}}), flush=True)
"#,
    )
    .unwrap();
    fs::set_permissions(&acp_script, fs::Permissions::from_mode(0o755)).unwrap();

    struct LspHost(PathBuf);
    impl HostToolchainProbe for LspHost {
        fn probe(&self, _tool: HostTool, _timeout: Duration) -> HostProbeResult {
            HostProbeResult::Missing
        }
        fn locate_language_server(&self, tool: HostTool) -> Option<PathBuf> {
            (tool == HostTool::Gopls).then(|| self.0.clone())
        }
    }

    // Course pack with daemon command, oneshot command, and long-running verification check.
    let pack_dir = temp.path().join("leak-pack");
    fs::create_dir_all(pack_dir.join("starter")).unwrap();
    fs::write(
        pack_dir.join("codelab.json"),
        r#"{
  "schemaVersion": "1.0",
  "courseId": "leak-check-pack",
  "title": "Leak Check Pack",
  "description": "Cross-module subprocess teardown verification",
  "language": "Go",
  "entryMarkdown": "README.md",
  "starterDir": "starter",
  "checksFile": ".codelab-checks.json"
}"#,
    )
    .unwrap();
    fs::write(
        pack_dir.join("starter/go.mod"),
        "module leakcheck\n\ngo 1.22\n",
    )
    .unwrap();
    fs::write(
        pack_dir.join("starter/main.go"),
        "package main\nfunc main() {}\n",
    )
    .unwrap();
    fs::write(
        pack_dir.join("README.md"),
        r#"# Leak Check

## 练习 1：Concurrency & Processes {#ex-1}

```bash mode="daemon" port=18991
sleep 120 & echo "$$ $!" > daemon-pids.txt; wait
```

```bash mode="oneshot"
sleep 120 & echo "$$ $!" > oneshot-pids.txt; wait
```
"#,
    )
    .unwrap();
    fs::write(
        pack_dir.join(".codelab-checks.json"),
        r#"{
  "schemaVersion": "1.0",
  "chapters": [
    {
      "chapterId": "ex-1",
      "checks": [
        {
          "id": "slow",
          "title": "Long check",
          "type": "command_test",
          "command": "sleep 120 & echo \"$$ $!\" > verify-pids.txt; wait",
          "timeoutSec": 60
        }
      ]
    }
  ]
}"#,
    )
    .unwrap();

    let app = App::open_with_runtime_adapters(
        AppConfig { data_dir },
        RuntimeAdapters {
            host: Arc::new(LspHost(lsp_bin)),
            ..RuntimeAdapters::default()
        },
    )
    .unwrap();
    let events = app.subscribe();

    app.execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
        tutor: Some(TutorSettingsPatch {
            preset: Some(codelab_core::AcpAgentPreset::Custom),
            command: Some(format!("python3 {}", acp_script.display())),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .unwrap();

    let pack_str = pack_dir.to_string_lossy().into_owned();
    app.execute(AppCommand::GrantTrust(TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: pack_str.clone(),
    }))
    .unwrap();
    app.execute(AppCommand::OpenCourse(OpenCourseRequest::LocalPack {
        path: pack_str,
    }))
    .unwrap();

    // Helper to spin up all 5 child-process families in a workspace and return their PIDs.
    let spawn_all = |ws_dir: &Path| -> Vec<(String, i32)> {
        fs::create_dir_all(ws_dir).unwrap();
        let AppResponse::WorkspaceBound(ws) = app
            .execute(AppCommand::BindWorkspace {
                path: ws_dir.to_string_lossy().into_owned(),
            })
            .unwrap()
        else {
            panic!("expected WorkspaceBound");
        };
        let ws_id = ws.id;

        // 1. Managed Daemon PTY + grandchild
        let r1 = app
            .execute(AppCommand::RunCommandBlock(RunCommandBlock {
                workspace_id: ws_id,
                command_id: "ex-1:cmd-1".into(),
                restart_terminal_id: None,
                confirmation: None,
                drafts: Vec::new(),
            }))
            .unwrap();
        assert!(
            matches!(r1, AppResponse::CommandStarted(_)),
            "daemon command failed to start: {r1:?}"
        );

        // 2. Oneshot PTY + grandchild
        let r2 = app
            .execute(AppCommand::RunCommandBlock(RunCommandBlock {
                workspace_id: ws_id,
                command_id: "ex-1:cmd-2".into(),
                restart_terminal_id: None,
                confirmation: None,
                drafts: Vec::new(),
            }))
            .unwrap();
        assert!(
            matches!(r2, AppResponse::CommandStarted(_)),
            "oneshot command failed to start: {r2:?}"
        );

        // 3. LSP worker + detached grandchild
        app.execute(AppCommand::SyncLanguageDocument(
            codelab_core::LanguageDocumentUpdate {
                workspace_id: ws_id,
                version: 1,
                document: Some(codelab_core::LanguageDocument {
                    path: "main.go".into(),
                    text: "package main\nfunc main() {}\n".into(),
                }),
            },
        ))
        .unwrap();

        // 4. ACP stdio agent + grandchild
        app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
            workspace_id: ws_id,
            prompt: "hello".into(),
            teaching_style: None,
            context_preamble: None,
            context: None,
        }))
        .unwrap();

        // 5. Headless verification process + grandchild
        app.execute(AppCommand::RunChecks(RunChecks {
            target: ExerciseTarget {
                workspace_id: ws_id,
                chapter_id: "ex-1".into(),
            },
            drafts: Vec::new(),
            confirmation: None,
        }))
        .unwrap();

        let mut pids = Vec::new();
        for (label, file) in [
            ("daemon", "daemon-pids.txt"),
            ("oneshot", "oneshot-pids.txt"),
            ("lsp", "lsp-pids.txt"),
            ("acp", "acp-pids.txt"),
            ("verify", "verify-pids.txt"),
        ] {
            let text = wait_file_text(&ws_dir.join(file));
            for (idx, token) in text.split_whitespace().enumerate() {
                let pid: i32 = token.parse().expect("valid pid");
                pids.push((format!("{label}-{idx}"), pid));
            }
        }
        // Drain any events before switching
        while events.try_recv().is_ok() {}
        pids
    };

    // Phase A: Workspace 1 -> switch to Workspace 2 -> ALL Workspace 1 processes must die!
    let ws1 = temp.path().join("ws-1");
    let ws1_pids = spawn_all(&ws1);
    let ws2 = temp.path().join("ws-2");
    let ws2_pids = spawn_all(&ws2);
    for (label, pid) in ws1_pids {
        wait_pid_dead(pid, &format!("ws1-{label}"));
    }

    // Phase B: Shutdown -> ALL Workspace 2 processes must die!
    app.execute(AppCommand::Shutdown).unwrap();
    for (label, pid) in ws2_pids {
        wait_pid_dead(pid, &format!("ws2-{label}"));
    }
}

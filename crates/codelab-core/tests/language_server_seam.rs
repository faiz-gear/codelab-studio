#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use codelab_core::{
    App, AppCommand, AppConfig, AppEvent, AppResponse, EventSubscription, HostProbeResult,
    HostTool, HostToolchainProbe, LanguageDocument, LanguageDocumentUpdate, LanguageStatus,
    OpenCourseRequest, RuntimeAdapters,
};
use tempfile::TempDir;

struct Host(Option<PathBuf>);

impl HostToolchainProbe for Host {
    fn probe(&self, _: HostTool, _: Duration) -> HostProbeResult {
        HostProbeResult::Missing
    }

    fn locate_language_server(&self, tool: HostTool) -> Option<PathBuf> {
        (tool == HostTool::Gopls).then(|| self.0.clone()).flatten()
    }
}

fn open(tmp: &TempDir, executable: Option<PathBuf>) -> App {
    App::open_with_runtime_adapters(
        AppConfig {
            data_dir: tmp.path().join("data"),
        },
        RuntimeAdapters {
            host: Arc::new(Host(executable)),
            ..RuntimeAdapters::default()
        },
    )
    .unwrap()
}

fn bind(app: &App, root: &Path) -> u64 {
    let AppResponse::WorkspaceBound(workspace) = app
        .execute(AppCommand::BindWorkspace {
            path: root.display().to_string(),
        })
        .unwrap()
    else {
        panic!("expected Workspace");
    };
    workspace.id
}

fn builtin(app: &App) {
    app.execute(AppCommand::OpenCourse(OpenCourseRequest::Builtin {
        course_id: "go-goframe-practice".into(),
    }))
    .unwrap();
}

fn activate(app: &App, workspace_id: u64, path: &str, version: i32) {
    app.execute(AppCommand::SyncLanguageDocument(LanguageDocumentUpdate {
        workspace_id,
        version,
        document: Some(LanguageDocument {
            path: path.into(),
            text: "package main\n".into(),
        }),
    }))
    .unwrap();
}

fn wait_status(events: &EventSubscription, version: i32, status: LanguageStatus) {
    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    loop {
        let event = events
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .expect("language status event");
        if matches!(event, AppEvent::LanguageStatusChanged(state) if state.document.version == version && state.status == status)
        {
            return;
        }
    }
}

#[test]
fn missing_language_server_preserves_workspace_editing() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("workspace");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("main.go"), "package main\n").unwrap();
    let app = open(&tmp, None);
    let events = app.subscribe();
    builtin(&app);
    let id = bind(&app, &root);
    assert!(
        !std::iter::from_fn(|| events.try_recv().ok())
            .any(|event| matches!(event, AppEvent::LanguageStatusChanged(_)))
    );
    activate(&app, id, "main.go", 1);
    wait_status(&events, 1, LanguageStatus::Missing);
    assert!(matches!(
        app.execute(AppCommand::ReadWorkspaceFile(codelab_core::WorkspacePath {
            workspace_id: id,
            path: "main.go".into(),
        }))
        .unwrap(),
        AppResponse::WorkspaceFile(_)
    ));
}

fn fake_server(tmp: &TempDir, behavior: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = tmp.path().join("gopls");
    let script = r#"#!/usr/bin/env python3
import json, os, subprocess, sys
from pathlib import Path

mode = '__BEHAVIOR__'
duration = '6' if mode == 'detached-crash' else '120'
child = subprocess.Popen(['/bin/sleep', duration], start_new_session=mode != 'crash')
Path('server-pids').write_text(f'{os.getpid()} {child.pid}')
def diagnostic(text):
    return {'message': text, 'range': {'start': {'line': 0, 'character': 0}, 'end': {'line': 0, 'character': 1}}}
def send(message):
    body = json.dumps({'jsonrpc': '2.0', **message}).encode()
    sys.stdout.buffer.write(f'Content-Length: {len(body)}\r\n\r\n'.encode() + body)
    sys.stdout.buffer.flush()
while header := sys.stdin.buffer.readline():
    size = int(header.split(b':')[1])
    sys.stdin.buffer.readline()
    message = json.loads(sys.stdin.buffer.read(size))
    method = message.get('method')
    params = message.get('params', {})
    if method == 'initialize':
        Path('server-root').write_text(params['rootUri'])
        if mode != 'timeout':
            send({'id': message['id'], 'result': {'capabilities': {
                'textDocumentSync': 2, 'completionProvider': {}, 'hoverProvider': True
            }}})
    if method == 'textDocument/didOpen':
        doc = params['textDocument']
        send({'method': 'textDocument/publishDiagnostics', 'params': {
            'uri': doc['uri'], 'version': doc['version'], 'diagnostics': [diagnostic(Path.cwd().name)]
        }})
    if method == 'textDocument/didChange':
        if mode in ('crash', 'detached-crash'):
            os._exit(1)
        doc = params['textDocument']
        text = params['contentChanges'][0]['text']
        assert 'range' in params['contentChanges'][0]
        send({'method': 'textDocument/publishDiagnostics', 'params': {
            'uri': doc['uri'], 'version': doc['version'] - 1, 'diagnostics': [diagnostic('stale')]
        }})
        send({'method': 'textDocument/publishDiagnostics', 'params': {
            'uri': doc['uri'], 'version': doc['version'], 'diagnostics': [diagnostic(text)]
        }})
    if method == 'textDocument/completion':
        send({'id': message['id'], 'result': [{'label': Path.cwd().name}]})
    if method == 'textDocument/hover':
        send({'id': message['id'], 'result': {'contents': 'Go documentation'}})
"#;
    fs::write(&path, script.replace("__BEHAVIOR__", behavior)).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn make_module(root: &Path, module: &str) -> PathBuf {
    let path = root.join(module);
    fs::create_dir_all(path.join("pkg")).unwrap();
    fs::write(path.join("go.mod"), "module example.org/demo\n").unwrap();
    fs::write(path.join("pkg/code.go"), "package demo\n").unwrap();
    path
}

fn pids(module: &Path) -> String {
    fs::read_to_string(module.join("server-pids")).unwrap()
}

fn assert_stopped(pids: &str) {
    use std::process::Command;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let all_stopped = pids.split_whitespace().all(|pid| {
            let status = Command::new("ps")
                .args(["-o", "stat=", "-p", pid])
                .output()
                .unwrap();
            !status.status.success()
                || String::from_utf8_lossy(&status.stdout)
                    .trim_start()
                    .starts_with('Z')
        });
        if all_stopped {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "still running: {pids}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn diagnostic(events: &EventSubscription, version: i32) -> String {
    loop {
        let event = events.recv_timeout(Duration::from_secs(3)).unwrap();
        if let AppEvent::LanguageDiagnostics(event) = event
            && event.document.version == version
        {
            return event.diagnostics[0].message.clone();
        }
    }
}

fn feature(
    app: &App,
    workspace_id: u64,
    version: i32,
    feature: codelab_core::LanguageFeature,
) -> Option<codelab_core::LanguageFeatureResult> {
    let AppResponse::LanguageFeature(value) = app
        .execute(AppCommand::RequestLanguageFeature(
            codelab_core::LanguageFeatureRequest {
                workspace_id,
                version,
                feature,
                line: 0,
                character: 8,
            },
        ))
        .unwrap()
    else {
        panic!("expected language feature");
    };
    value
}

fn assert_completion(app: &App, workspace_id: u64, version: i32, label: &str) {
    use codelab_core::{LanguageFeature::Completion, LanguageFeatureResult};
    let Some(LanguageFeatureResult::Completion(result)) =
        feature(app, workspace_id, version, Completion)
    else {
        panic!("expected completion")
    };
    assert_eq!(result.items[0].label, label);
}

#[test]
fn opens_only_the_active_module_and_evicts_the_least_recently_used_server() {
    use codelab_core::LanguageFeature::Hover;
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("workspace");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("go.mod"), "module example.org/workspace\n").unwrap();
    let modules =
        ["task-api", "task-api-gf", "tools", "checks"].map(|name| make_module(&root, name));
    let app = open(&tmp, Some(fake_server(&tmp, "")));
    let events = app.subscribe();
    builtin(&app);
    let id = bind(&app, &root);
    app.execute(AppCommand::RefreshPreflight).unwrap();
    assert!(
        modules
            .iter()
            .all(|module| !module.join("server-pids").exists())
    );

    activate(&app, id, "task-api/pkg/code.go", 1);
    wait_status(&events, 1, LanguageStatus::Ready);
    assert_eq!(diagnostic(&events, 1), "task-api");
    assert_completion(&app, id, 1, "task-api");
    let Some(codelab_core::LanguageFeatureResult::Hover(hover)) = feature(&app, id, 1, Hover)
    else {
        panic!("expected hover")
    };
    assert_eq!(
        hover.contents,
        [codelab_core::LanguageMarkup::Text(
            "Go documentation".into()
        )]
    );
    assert!(
        fs::read_to_string(modules[0].join("server-root"))
            .unwrap()
            .ends_with("/task-api")
    );
    assert!(!modules[1].join("server-pids").exists());
    activate(&app, id, "task-api/pkg/code.go", 2);
    wait_status(&events, 2, LanguageStatus::Ready);
    assert_eq!(diagnostic(&events, 2), "package main\n");

    for (index, name) in ["task-api-gf", "tools", "task-api", "checks"]
        .into_iter()
        .enumerate()
    {
        let version = index as i32 + 3;
        activate(&app, id, &format!("{name}/pkg/code.go"), version);
        wait_status(&events, version, LanguageStatus::Ready);
        assert_eq!(diagnostic(&events, version), name);
        assert_completion(&app, id, version, name);
    }

    assert_stopped(&pids(&modules[1]));
    for index in [0, 2, 3] {
        let pid = pids(&modules[index])
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<i32>()
            .unwrap();
        assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
    }
    let other = tmp.path().join("another");
    make_module(&other, "next");
    let next_id = bind(&app, &other);
    for module in &modules {
        assert_stopped(&pids(module));
    }
    activate(&app, next_id, "next/pkg/code.go", 1);
    wait_status(&events, 1, LanguageStatus::Ready);
    let next_pids = pids(&other.join("next"));
    app.shutdown().unwrap();
    assert_stopped(&next_pids);
}

#[test]
fn trust_gates_launch_and_revocation_reaps_the_server() {
    use codelab_core::{ErrorCode, TrustSource, TrustSourceKind};
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("workspace");
    let module = make_module(&root, "demo");
    let guide = tmp.path().join("manual.md");
    fs::write(&guide, "# Local Course\n").unwrap();
    let source = TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: guide.display().to_string(),
    };
    let app = open(&tmp, Some(fake_server(&tmp, "")));
    let events = app.subscribe();
    app.execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
        path: source.source.clone(),
    }))
    .unwrap();
    let sync = AppCommand::SyncLanguageDocument(LanguageDocumentUpdate {
        workspace_id: 1,
        version: 1,
        document: Some(LanguageDocument {
            path: "demo/pkg/code.go".into(),
            text: "package demo".into(),
        }),
    });
    assert_eq!(
        app.execute(sync.clone()).unwrap_err().code,
        ErrorCode::CourseUntrusted
    );
    app.execute(AppCommand::GrantTrust(source.clone())).unwrap();
    bind(&app, &root);
    assert!(!module.join("server-pids").exists());
    app.execute(sync).unwrap();
    wait_status(&events, 1, LanguageStatus::Ready);
    app.execute(AppCommand::RevokeTrust(source)).unwrap();
    assert_stopped(&pids(&module));
}

#[test]
fn startup_timeout_and_crash_each_degrade_and_reap_descendants() {
    for behavior in ["timeout", "crash"] {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("workspace");
        let module = make_module(&root, "demo");
        let app = open(&tmp, Some(fake_server(&tmp, behavior)));
        let events = app.subscribe();
        builtin(&app);
        let id = bind(&app, &root);
        activate(&app, id, "demo/pkg/code.go", 1);
        let version = if behavior == "crash" {
            wait_status(&events, 1, LanguageStatus::Ready);
            activate(&app, id, "demo/pkg/code.go", 2);
            2
        } else {
            1
        };
        wait_status(&events, version, LanguageStatus::Degraded);
        assert_stopped(&pids(&module));
        assert!(feature(&app, id, version, codelab_core::LanguageFeature::Hover).is_none());
    }
}

#[test]
fn workspace_switch_and_shutdown_reap_detached_helpers_after_a_server_crashes() {
    let tmp = TempDir::new().unwrap();
    let first = tmp.path().join("first");
    let second = tmp.path().join("second");
    let first_module = make_module(&first, "demo");
    let second_module = make_module(&second, "demo");
    let app = open(&tmp, Some(fake_server(&tmp, "detached-crash")));
    let events = app.subscribe();
    builtin(&app);

    let crash_server = |workspace_id, module| {
        activate(&app, workspace_id, "demo/pkg/code.go", 1);
        wait_status(&events, 1, LanguageStatus::Ready);
        let processes = pids(module);
        activate(&app, workspace_id, "demo/pkg/code.go", 2);
        wait_status(&events, 2, LanguageStatus::Degraded);
        processes
    };
    let first_processes = crash_server(bind(&app, &first), &first_module);
    let started = std::time::Instant::now();
    let second_id = bind(&app, &second);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_stopped(&first_processes);

    let second_processes = crash_server(second_id, &second_module);
    let started = std::time::Instant::now();
    app.shutdown().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_stopped(&second_processes);
}

use std::{fs, path::Path};

use codelab_core::{App, AppCommand, AppConfig, AppResponse, ErrorCode, OpenCourseRequest};
use tempfile::TempDir;

fn wire(command: serde_json::Value) -> AppCommand {
    serde_json::from_value(command).expect("execution command is part of the App contract")
}

fn fixture(manual: &str) -> (TempDir, App, String) {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("manual.md");
    fs::write(&source, manual).unwrap();
    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let AppResponse::LoadedCourse(course) = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: path(&source),
        }))
        .unwrap()
    else {
        panic!("expected Course")
    };
    let id = course
        .chapters
        .iter()
        .flat_map(|chapter| &chapter.commands)
        .next()
        .unwrap()
        .id
        .clone();
    (temp, app, id)
}

fn path(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

#[test]
fn trust_gate_applies_to_command_blocks_and_manual_terminals() {
    let (_temp, app, id) = fixture("# Local\n\n```sh\nprintf OK\n```\n");
    for command in [
        serde_json::json!({"type":"openTerminal","payload":{"workspaceId":1}}),
        serde_json::json!({"type":"runCommandBlock","payload":{"workspaceId":1,"commandId":id}}),
    ] {
        assert_eq!(
            app.execute(wire(command)).unwrap_err().code,
            ErrorCode::CourseUntrusted
        );
    }
}

fn bind(temp: &TempDir, app: &App, name: &str) -> u64 {
    let root = temp.path().join(name);
    fs::create_dir_all(&root).unwrap();
    let AppResponse::WorkspaceBound(workspace) = app
        .execute(AppCommand::BindWorkspace { path: path(&root) })
        .unwrap()
    else {
        panic!("expected Workspace")
    };
    workspace.id
}

fn trust(temp: &TempDir, app: &App) {
    app.execute(wire(serde_json::json!({"type":"grantTrust","payload":{
        "sourceKind":"localPath", "source": path(&temp.path().join("manual.md"))
    }})))
    .unwrap();
}

fn send(app: &App, name: &str, payload: serde_json::Value) -> serde_json::Value {
    serde_json::to_value(
        app.execute(wire(serde_json::json!({
            "type": name, "payload": payload
        })))
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn structured_high_risk_commands_require_exact_single_use_confirmation() {
    let command = "printf '%s' 'safe fixture' | /bin/sh\n# keep the entire block in the dialog";
    let (temp, app, id) = fixture(&format!("# Local\n\n```sh\n{command}\n```\n"));
    trust(&temp, &app);
    let workspace = bind(&temp, &app, "work");
    let draft_path = temp.path().join("work/draft.txt");
    fs::write(&draft_path, "original").unwrap();
    let draft = send(
        &app,
        "readWorkspaceFile",
        serde_json::json!({"workspaceId":workspace,"path":"draft.txt"}),
    )["payload"]
        .clone();
    let request = serde_json::json!({
        "workspaceId":workspace,
        "commandId":id,
        "drafts":[{"path":"draft.txt","text":"confirmed draft","expectedRevision":draft["revision"]}]
    });
    let pending = send(&app, "runCommandBlock", request.clone());
    assert_eq!(pending["type"], "commandConfirmationRequired");
    assert_eq!(pending["payload"]["command"], command);
    assert_eq!(
        pending["payload"]["cwd"],
        path(&temp.path().join("work").canonicalize().unwrap())
    );
    assert_eq!(fs::read_to_string(&draft_path).unwrap(), "original");
    assert_eq!(
        send(
            &app,
            "readTerminals",
            serde_json::json!({"workspaceId":workspace})
        )["payload"]["terminals"],
        serde_json::json!([])
    );
    let mut confirmed = request;
    confirmed["confirmation"] = pending["payload"]["token"].clone();
    let started = send(&app, "runCommandBlock", confirmed.clone());
    assert_eq!(started["type"], "commandStarted");
    assert_eq!(started["payload"]["saved"][0]["text"], "confirmed draft");
    assert_eq!(fs::read_to_string(&draft_path).unwrap(), "confirmed draft");
    assert_eq!(
        send(&app, "runCommandBlock", confirmed)["type"],
        "commandConfirmationRequired"
    );
}

fn wait_terminal(
    app: &App,
    workspace_id: u64,
    terminal_id: u64,
    until: impl Fn(&serde_json::Value, &str) -> bool,
) -> (serde_json::Value, String) {
    use base64::Engine as _;
    let mut output = Vec::new();
    let mut cursor = 0;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
    loop {
        let updates = send(
            app,
            "readTerminals",
            serde_json::json!({
                "workspaceId":workspace_id,"terminalId":terminal_id,"cursor":cursor
            }),
        )["payload"]
            .clone();
        let chunk = &updates["output"];
        cursor = chunk["cursor"].as_u64().unwrap();
        if chunk["reset"] == true {
            output.clear();
        }
        output.extend(
            base64::engine::general_purpose::STANDARD
                .decode(chunk["data"].as_str().unwrap())
                .unwrap(),
        );
        let decoded = String::from_utf8_lossy(&output);
        if until(&updates, &decoded) {
            return (updates, decoded.into_owned());
        }
        assert!(
            std::time::Instant::now() < deadline,
            "terminal timed out with {} and output {decoded:?}",
            updates["terminals"]
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[cfg(unix)]
#[test]
fn saves_all_drafts_before_launch_and_blocks_on_external_conflict() {
    let (temp, app, id) =
        fixture("# Local\n\n```sh\ntest -t 0 && test -t 1 && cat main.txt\n```\n");
    trust(&temp, &app);
    let workspace = bind(&temp, &app, "work");
    let original = temp.path().join("work/main.txt");
    fs::write(&original, "original").unwrap();
    let before = send(
        &app,
        "readWorkspaceFile",
        serde_json::json!({"workspaceId":workspace,"path":"main.txt"}),
    );
    let run = || {
        send(
            &app,
            "runCommandBlock",
            serde_json::json!({
                "workspaceId":workspace, "commandId":id, "drafts":[{"path":"main.txt","text":"saved draft","expectedRevision":before["payload"]["revision"]}]
            }),
        )
    };
    fs::write(&original, "external edit").unwrap();
    let conflict = run();
    assert_eq!(conflict["type"], "commandSaveConflict");
    assert_eq!(conflict["payload"]["file"]["text"], "external edit");
    assert_eq!(fs::read_to_string(&original).unwrap(), "external edit");
    assert_eq!(
        send(
            &app,
            "readTerminals",
            serde_json::json!({"workspaceId":workspace})
        )["payload"]["terminals"],
        serde_json::json!([])
    );

    fs::write(&original, "original").unwrap();
    let started = run();
    assert_eq!(started["type"], "commandStarted");
    let terminal = started["payload"]["terminal"]["id"].as_u64().unwrap();
    wait_terminal(&app, workspace, terminal, |_, output| {
        output.contains("saved draft")
    });
}

#[test]
fn returns_saved_revisions_when_a_later_draft_cannot_be_written() {
    let (temp, app, id) = fixture("# Local\n\n```sh\nprintf OK\n```\n");
    trust(&temp, &app);
    let workspace = bind(&temp, &app, "work");
    let root = temp.path().join("work");
    fs::write(root.join("main.txt"), "original").unwrap();
    fs::write(root.join("binary.txt"), [0, 255]).unwrap();
    let read = |file| {
        send(
            &app,
            "readWorkspaceFile",
            serde_json::json!({"workspaceId": workspace, "path": file}),
        )
    };
    let response = send(
        &app,
        "runCommandBlock",
        serde_json::json!({
            "workspaceId": workspace, "commandId": id,
            "drafts": [
                {"path": "main.txt", "text": "updated", "expectedRevision": read("main.txt")["payload"]["revision"]},
                {"path": "binary.txt", "text": "invalid", "expectedRevision": read("binary.txt")["payload"]["revision"]}
            ]
        }),
    );
    assert_eq!(response["type"], "commandFailed");
    assert_eq!(
        response["payload"]["error"]["code"],
        "WORKSPACE_FILE_NOT_TEXT"
    );
    assert_eq!(
        response["payload"]["saved"],
        serde_json::json!([read("main.txt")["payload"]])
    );
    assert_eq!(
        fs::read_to_string(root.join("main.txt")).unwrap(),
        "updated"
    );
    assert_eq!(fs::read(root.join("binary.txt")).unwrap(), [0, 255]);
    assert_eq!(
        send(
            &app,
            "readTerminals",
            serde_json::json!({"workspaceId": workspace})
        )["payload"]["terminals"],
        serde_json::json!([])
    );
}

#[cfg(unix)]
#[test]
fn routes_oneshots_away_from_live_daemons_and_reports_port_conflicts() {
    let (temp, app, id) = fixture(
        "# Local\n\n```sh mode=daemon label=API port=5222\nprintf 'daemon ready\\n'\nwhile :; do sleep 1; done\n```\n\n```sh mode=oneshot\nprintf 'ERROR: address already in use\\n'\n```\n",
    );
    trust(&temp, &app);
    let workspace = bind(&temp, &app, "work");
    let daemon = send(
        &app,
        "runCommandBlock",
        serde_json::json!({"workspaceId":workspace,"commandId":id}),
    );
    let service = daemon["payload"]["terminal"]["id"].as_u64().unwrap();
    assert_eq!(daemon["payload"]["terminal"]["port"], 5222);
    wait_terminal(&app, workspace, service, |_, output| {
        output.contains("daemon ready")
    });
    let one_id = id.replace(":cmd-1", ":cmd-2");
    let run = || {
        send(
            &app,
            "runCommandBlock",
            serde_json::json!({"workspaceId":workspace,"commandId":one_id}),
        )
    };
    let one = run()["payload"]["terminal"]["id"].as_u64().unwrap();
    assert_ne!(one, service);
    let (status, _) = wait_terminal(&app, workspace, one, |updates, output| {
        output.contains("address already in use") && updates["terminals"][1]["running"] == false
    });
    assert_eq!(status["terminals"][0]["running"], true);
    assert_eq!(status["terminals"][1]["portConflict"], true);
    assert_eq!(status["recentErrors"][0], "ERROR: address already in use");
    assert_eq!(run()["payload"]["terminal"]["id"], one);
    send(
        &app,
        "stopTerminal",
        serde_json::json!({"workspaceId":workspace,"terminalId":service}),
    );
    let restarted = send(
        &app,
        "runCommandBlock",
        serde_json::json!({"workspaceId":workspace,"commandId":id,"restartTerminalId":service}),
    );
    assert_eq!(restarted["payload"]["terminal"]["id"], service);
    assert_ne!(
        restarted["payload"]["terminal"]["startedAt"],
        daemon["payload"]["terminal"]["startedAt"]
    );
}

#[test]
fn detects_the_documented_risky_shell_patterns_in_any_statement() {
    for command in [
        "printf safe; /bin/rm -f -r ./fixture",
        "if true; then command sudo -n true; fi",
        "checkpoint=$( /sbin/mkfs.ext4 /tmp/fixture )",
        "env dd if='/tmp/fixture' of=/dev/null",
        "chmod --recursive 0777 /",
        "printf safe | env bash",
    ] {
        let (temp, app, id) = fixture(&format!("# Local\n\n```bash\n{command}\n```\n"));
        trust(&temp, &app);
        let workspace = bind(&temp, &app, "work");
        let pending = send(
            &app,
            "runCommandBlock",
            serde_json::json!({"workspaceId":workspace,"commandId":id}),
        );
        assert_eq!(pending["type"], "commandConfirmationRequired", "{command}");
    }
}

#[test]
fn cwd_override_survives_reload_and_reopen_without_moving_to_a_different_block() {
    let command = "```sh cwd=fenced\nprintf '%s\\n' \"$PWD\"\n```\n";
    let (temp, app, id) = fixture(&format!("# Local\n\n{command}"));
    trust(&temp, &app);
    let work = temp.path().join("work");
    fs::create_dir_all(work.join("fenced")).unwrap();
    fs::create_dir(work.join("chosen")).unwrap();
    let workspace = bind(&temp, &app, "work");
    let listed = |workspace| {
        send(
            &app,
            "getCommandCwdOverrides",
            serde_json::json!({"workspaceId":workspace}),
        )
    };
    let invalid = app
        .execute(wire(
            serde_json::json!({"type":"setCommandCwdOverride","payload":{
                "workspaceId":workspace,"commandId":id,"cwd":"../"
            }}),
        ))
        .unwrap_err();
    assert_eq!(invalid.code, ErrorCode::CommandCwdInvalid);

    send(
        &app,
        "setCommandCwdOverride",
        serde_json::json!({"workspaceId":workspace,"commandId":id,"cwd":"chosen"}),
    );
    fs::write(
        temp.path().join("manual.md"),
        format!("# Local\n\n```sh\ntrue\n```\n\n{command}"),
    )
    .unwrap();
    app.execute(AppCommand::ReloadActiveCourse).unwrap();
    let updated_id = id.replace(":cmd-1", ":cmd-2");
    assert_eq!(
        listed(workspace)["payload"],
        serde_json::json!({updated_id.clone():"chosen"})
    );
    let workspace = bind(&temp, &app, "work");
    assert_eq!(
        listed(workspace)["payload"],
        serde_json::json!({updated_id.clone():"chosen"})
    );

    #[cfg(unix)]
    {
        let run = || {
            send(
                &app,
                "runCommandBlock",
                serde_json::json!({"workspaceId":workspace,"commandId":updated_id}),
            )
        };
        let terminal = run()["payload"]["terminal"].clone();
        assert_eq!(
            terminal["cwd"],
            path(&work.join("chosen").canonicalize().unwrap())
        );
        wait_terminal(
            &app,
            workspace,
            terminal["id"].as_u64().unwrap(),
            |_, output| output.contains("/chosen"),
        );
        send(
            &app,
            "setCommandCwdOverride",
            serde_json::json!({"workspaceId":workspace,"commandId":updated_id,"cwd":null}),
        );
        assert_eq!(
            run()["payload"]["terminal"]["cwd"],
            path(&work.join("fenced").canonicalize().unwrap())
        );
    }
}

#[test]
fn terminal_service_fixture() {
    let Ok(port) = std::env::var("CODELAB_TEST_LISTEN_PORT") else {
        return;
    };
    if let Ok(expected) = std::env::var("CODELAB_TEST_WORKSPACE_CWD") {
        assert_eq!(fs::read_to_string("terminal-cwd.txt").unwrap(), expected);
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("CODELAB_TEST_NEW_SESSION").is_some() {
        assert!(unsafe { libc::setsid() } > 0);
    }
    let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();
    println!(
        "Listening on http://127.0.0.1:{}",
        listener.local_addr().unwrap().port()
    );
    loop {
        std::thread::park();
    }
}

#[cfg(target_os = "linux")]
#[test]
fn shutdown_reclaims_a_detached_child_with_an_identifiable_parent() {
    let executable = std::env::current_exe().unwrap();
    let manual = format!(
        "# Local\n\n```sh mode=daemon\nCODELAB_TEST_NEW_SESSION=1 CODELAB_TEST_LISTEN_PORT=0 '{}' --exact terminal_service_fixture --nocapture &\nwait\n```\n",
        path(&executable).replace('\'', "'\\''"),
    );
    let (temp, app, command) = fixture(&manual);
    trust(&temp, &app);
    let workspace = bind(&temp, &app, "work");
    let started = send(
        &app,
        "runCommandBlock",
        serde_json::json!({"workspaceId": workspace, "commandId": command}),
    );
    let id = started["payload"]["terminal"]["id"].as_u64().unwrap();
    let (updates, _) = wait_terminal(&app, workspace, id, |updates, _| {
        updates["terminals"][0]["port"].is_number()
    });
    let port = updates["terminals"][0]["port"].as_u64().unwrap() as u16;
    app.shutdown().unwrap();
    assert_port_released(port);
}

fn assert_port_released(port: u16) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
        assert!(
            std::time::Instant::now() < deadline,
            "PTY descendant retained port {port}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn workspace_switch_and_shutdown_reclaim_manual_terminal_descendants() {
    let (temp, app, _) = fixture("# Local\n\n```sh\ntrue\n```\n");
    trust(&temp, &app);
    let first = bind(&temp, &app, "first");
    let executable = std::env::current_exe().unwrap();
    for name in ["first", "second"] {
        let directory = temp.path().join(name);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("terminal-cwd.txt"), name).unwrap();
    }

    let launch = |workspace, name| {
        let terminal = send(
            &app,
            "openTerminal",
            serde_json::json!({"workspaceId":workspace}),
        );
        let terminal_id = terminal["payload"]["id"].as_u64().unwrap();
        #[cfg(unix)]
        let text = format!(
            "/usr/bin/env CODELAB_TEST_WORKSPACE_CWD={name} CODELAB_TEST_LISTEN_PORT=0 '{}' --exact terminal_service_fixture --nocapture &\r",
            path(&executable).replace('\'', "'\\''")
        );
        #[cfg(windows)]
        let text = format!(
            "set CODELAB_TEST_WORKSPACE_CWD={name}&&set CODELAB_TEST_LISTEN_PORT=0&&\"{}\" --exact terminal_service_fixture --nocapture\r",
            dunce::simplified(&executable).display()
        );
        send(
            &app,
            "writeTerminal",
            serde_json::json!({"workspaceId":workspace,"terminalId":terminal_id,"text":text}),
        );
        let (updates, output) = wait_terminal(&app, workspace, terminal_id, |updates, output| {
            output.contains("Listening on http://127.0.0.1:")
                && updates["terminals"][0]["port"].is_number()
        });
        let port = updates["terminals"][0]["port"].as_u64().unwrap() as u16;
        assert!(output.contains(&format!("Listening on http://127.0.0.1:{port}")));
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_err());
        port
    };

    let port = launch(first, "first");
    let second = bind(&temp, &app, "second");
    assert_port_released(port);
    let port = launch(second, "second");
    app.execute(AppCommand::Shutdown).unwrap();
    assert_port_released(port);
}

#[cfg(unix)]
#[test]
fn retains_five_thousand_output_lines_and_two_hundred_recent_errors() {
    let (temp, app, id) = fixture(
        "# Local\n\n```sh\ni=1; while [ \"$i\" -le 5300 ]; do printf 'ERROR line %04d\\n' \"$i\"; i=$((i+1)); done\n```\n",
    );
    trust(&temp, &app);
    let workspace = bind(&temp, &app, "work");
    let terminal = send(
        &app,
        "runCommandBlock",
        serde_json::json!({"workspaceId":workspace,"commandId":id}),
    );
    let id = terminal["payload"]["terminal"]["id"].as_u64().unwrap();
    wait_terminal(&app, workspace, id, |updates, output| {
        output.contains("ERROR line 5300") && updates["terminals"][0]["running"] == false
    });
    let (updates, output) = wait_terminal(&app, workspace, id, |_, output| {
        output.contains("ERROR line 5300")
    });
    assert_eq!(output.lines().count(), 5_000);
    assert!(output.starts_with("ERROR line 0301"));
    let errors = updates["recentErrors"].as_array().unwrap();
    assert_eq!(errors.len(), 200);
    assert_eq!(errors[0], "ERROR line 5101");
    assert_eq!(errors[199], "ERROR line 5300");
}

#[cfg(unix)]
fn app_with_system_go(temp: &TempDir, executable: std::path::PathBuf) -> App {
    use codelab_core::{HostProbeResult, HostTool, HostToolchainProbe, RuntimeAdapters};

    struct SelectedGo(std::path::PathBuf);
    impl HostToolchainProbe for SelectedGo {
        fn probe(&self, tool: HostTool, _: std::time::Duration) -> HostProbeResult {
            if tool == HostTool::Go {
                return HostProbeResult::Found {
                    output: "go version go1.23.5".into(),
                    executable: Some(self.0.clone()),
                };
            }
            HostProbeResult::Missing
        }
    }

    let app = App::open_with_runtime_adapters(
        AppConfig {
            data_dir: temp.path().join("data"),
        },
        RuntimeAdapters {
            host: std::sync::Arc::new(SelectedGo(executable)),
            ..RuntimeAdapters::default()
        },
    )
    .unwrap();
    let events = app.subscribe();
    app.execute(AppCommand::RefreshPreflight).unwrap();
    while !matches!(events.recv_timeout(std::time::Duration::from_secs(5)).unwrap(),
        codelab_core::AppEvent::PreflightUpdated(report) if report.checked && !report.probing
    ) {}
    app
}

#[cfg(unix)]
#[test]
fn derives_selected_system_toolchain_environment_in_the_child_only() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().unwrap();
    let executable = temp.path().join("host tools/go");
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::write(&executable, "#!/bin/sh\nprintf 'selected Go fixture\\n'\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let original_path = std::env::var_os("PATH");
    let app = app_with_system_go(&temp, executable);
    let manual = temp.path().join("manual.md");
    fs::write(&manual, "# Local\n\n```sh\ngo\n```\n").unwrap();
    let AppResponse::LoadedCourse(course) = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: path(&manual),
        }))
        .unwrap()
    else {
        panic!("expected Course")
    };
    trust(&temp, &app);
    let workspace = bind(&temp, &app, "work");
    let command = &course.chapters[0].commands[0];
    let started = send(
        &app,
        "runCommandBlock",
        serde_json::json!({"workspaceId":workspace,"commandId":command.id}),
    );
    let id = started["payload"]["terminal"]["id"].as_u64().unwrap();
    wait_terminal(&app, workspace, id, |_, output| {
        output.contains("selected Go fixture")
    });
    assert_eq!(std::env::var_os("PATH"), original_path);
}

#[cfg(unix)]
#[test]
fn returns_saved_revisions_when_terminal_start_fails() {
    let temp = TempDir::new().unwrap();
    // A colon is valid in the tool directory but cannot be encoded as a PATH entry.
    let app = app_with_system_go(&temp, temp.path().join("host:tools/go"));
    let manual = temp.path().join("manual.md");
    fs::write(&manual, "# Local\n\n```sh\nprintf OK\n```\n").unwrap();
    let AppResponse::LoadedCourse(course) = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: path(&manual),
        }))
        .unwrap()
    else {
        panic!("expected Course")
    };
    trust(&temp, &app);
    let workspace = bind(&temp, &app, "work");
    fs::write(temp.path().join("work/main.txt"), "original").unwrap();
    let read = || {
        send(
            &app,
            "readWorkspaceFile",
            serde_json::json!({"workspaceId": workspace, "path": "main.txt"}),
        )
    };
    let response = send(
        &app,
        "runCommandBlock",
        serde_json::json!({
            "workspaceId": workspace, "commandId": course.chapters[0].commands[0].id,
            "drafts": [{"path": "main.txt", "text": "updated", "expectedRevision": read()["payload"]["revision"]}]
        }),
    );
    assert_eq!(response["type"], "commandFailed");
    assert_eq!(response["payload"]["error"]["code"], "EXECUTION_FAILED");
    assert_eq!(
        response["payload"]["saved"],
        serde_json::json!([read()["payload"]])
    );
    assert_eq!(response["payload"]["saved"][0]["text"], "updated");
    assert_eq!(
        send(
            &app,
            "readTerminals",
            serde_json::json!({"workspaceId": workspace})
        )["payload"]["terminals"],
        serde_json::json!([])
    );
}

#[cfg(unix)]
#[test]
fn opening_a_draft_and_leaving_a_trial_reclaims_live_managed_daemons() {
    let executable = std::env::current_exe().unwrap();
    let manual = format!(
        "# Local\n\n## Exercise 1 {{#ex-1}}\n\n```sh mode=daemon label=API\nCODELAB_TEST_LISTEN_PORT=0 '{}' --exact terminal_service_fixture --nocapture\n```\n",
        path(&executable).replace('\'', "'\\''"),
    );
    let (temp, app, regular_command) = fixture(&manual);
    trust(&temp, &app);
    let regular = bind(&temp, &app, "work");
    let launch = |workspace_id: u64, command_id: &str| {
        let started = send(
            &app,
            "runCommandBlock",
            serde_json::json!({
                "workspaceId":workspace_id, "commandId":command_id,
            }),
        );
        let id = started["payload"]["terminal"]["id"].as_u64().unwrap();
        let (state, _) = wait_terminal(&app, workspace_id, id, |state, output| {
            state["terminals"][0]["port"].is_number() && output.contains("Listening on ")
        });
        let terminal = &state["terminals"][0];
        assert_eq!(terminal["mode"], "daemon");
        assert_eq!(terminal["running"], true);
        terminal["port"].as_u64().unwrap() as u16
    };

    let port = launch(regular, &regular_command);
    let draft = send(
        &app,
        "createCourseDraft",
        serde_json::json!({
            "path":path(&temp.path().join("draft")), "title":"Trial service",
            "language":"Go", "referenceSolution":false,
        }),
    );
    assert_port_released(port);

    let draft_id = draft["payload"]["id"].as_u64().unwrap();
    let target = serde_json::json!({"draftId":draft_id,"path":"README.md"});
    let before = send(&app, "readCourseDraftFile", target.clone());
    let mut write = target;
    write["text"] = serde_json::json!(manual);
    write["expectedRevision"] = before["payload"]["revision"].clone();
    send(&app, "writeCourseDraftFile", write);
    let start_trial = || {
        send(
            &app,
            "startTrialWorkspace",
            serde_json::json!({"draftId":draft_id}),
        )
    };
    let trial = start_trial();
    let workspace = &trial["payload"]["workspace"];
    let trial_id = workspace["id"].as_u64().unwrap();
    let root = Path::new(workspace["root"].as_str().unwrap());
    let block = &trial["payload"]["course"]["chapters"][0]["commands"][0]["id"];
    let port = launch(trial_id, block.as_str().unwrap());

    let replaced = start_trial();
    assert!(!root.exists());
    assert_port_released(port);
    let workspace = &replaced["payload"]["workspace"];
    let trial_id = workspace["id"].as_u64().unwrap();
    let root = Path::new(workspace["root"].as_str().unwrap());
    let port = launch(trial_id, block.as_str().unwrap());
    app.execute(AppCommand::OpenCourse(OpenCourseRequest::Builtin {
        course_id: "go-goframe-practice".into(),
    }))
    .unwrap();
    assert!(!root.exists());
    assert_port_released(port);
}

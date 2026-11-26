use codelab_core::{App, AppCommand, AppConfig, ErrorCode};
use serde_json::{Value, json};
use std::{
    fs,
    net::TcpListener,
    sync::Mutex,
    time::{Duration, Instant},
};
use tempfile::TempDir;

static GO_TEST_LOCK: Mutex<()> = Mutex::new(());

fn send(app: &App, kind: &str, payload: Value) -> Value {
    serde_json::to_value(
        app.execute(
            serde_json::from_value::<AppCommand>(json!({"type":kind,"payload":payload})).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}
fn fixture(checks: Value, manual: &str) -> (TempDir, App, u64) {
    fixture_with_language(checks, manual, None)
}

fn fixture_with_language(
    checks: Value,
    manual: &str,
    language: Option<&str>,
) -> (TempDir, App, u64) {
    let temp = TempDir::new().unwrap();
    let pack = temp.path().join("pack");
    fs::create_dir(&pack).unwrap();
    fs::write(pack.join("README.md"), manual).unwrap();
    let mut manifest = json!({
        "schemaVersion": "1.0", "courseId": "verify", "title": "Verification",
        "entryFile": "README.md"
    });
    if let Some(language) = language {
        manifest["language"] = json!(language);
    }
    fs::write(pack.join("codelab.json"), manifest.to_string()).unwrap();
    fs::write(
        pack.join(".codelab-checks.json"),
        json!({"schemaVersion":"1.0","chapters":checks}).to_string(),
    )
    .unwrap();
    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    send(&app, "openCourse", json!({"kind":"localPack","path":pack}));
    send(
        &app,
        "grantTrust",
        json!({"sourceKind":"localPath","source":pack}),
    );
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    let id = send(&app, "bindWorkspace", json!({"path":root}))["payload"]["id"]
        .as_u64()
        .unwrap();
    (temp, app, id)
}
fn wait(app: &App, id: u64) -> Value {
    wait_for_checks(app, id, Duration::from_secs(20))
}

fn wait_for_go_checks(app: &App, id: u64) -> Value {
    // Allow the Go Check's full timeout plus process cleanup on a cold runner.
    wait_for_checks(app, id, Duration::from_secs(125))
}

fn run_go_checks(app: &App, id: u64, chapter_id: &str) -> Value {
    // Serialize real `go test -race` checks so parallel Rust tests do not
    // compile the Go standard library simultaneously into a cold GOCACHE.
    let _guard = GO_TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    send(
        app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":chapter_id}),
    );
    wait_for_go_checks(app, id)
}

fn wait_port_released(port: u16) -> TcpListener {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => return listener,
            Err(error) => {
                assert!(
                    Instant::now() < until,
                    "descendant retained port {port}: {error}"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

fn wait_for_checks(app: &App, id: u64, timeout: Duration) -> Value {
    let until = Instant::now() + timeout;
    loop {
        let status = send(app, "getProgress", json!({"workspaceId":id}))["payload"].clone();
        if status["runningChapterId"].is_null() {
            return status;
        }
        assert!(
            Instant::now() < until,
            "verification did not finish: {status}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn headless_commands_produce_results_and_sticky_completion_without_pty_output() {
    let (temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"command","title":"reads saved code","type":"command_test","command":"cat answer.txt","expectExitCode":0,"forbidOutput":["wrong"]},
            {"id":"reflection","title":"reflect","type":"self_check"}
        ]}]),
        "# Course\n\n## Exercise 1\n\n### ③ Check\n",
    );
    fs::write(temp.path().join("work/answer.txt"), "right").unwrap();
    assert_eq!(
        send(
            &app,
            "runChecks",
            json!({"workspaceId":id,"chapterId":"ex-1"})
        )["type"],
        "verificationStarted"
    );
    let progress = wait(&app, id);
    assert_eq!(progress["chapters"]["ex-1"]["completionState"], "completed");
    assert_eq!(
        progress["chapters"]["ex-1"]["lastCheckResults"][0]["status"],
        "passed"
    );
    assert!(
        progress["chapters"]["ex-1"]["lastCheckResults"][0]["rawOutput"]
            .as_str()
            .unwrap()
            .contains("right")
    );
    assert_eq!(
        send(&app, "readTerminals", json!({"workspaceId":id}))["payload"]["terminals"],
        json!([])
    );
    fs::write(temp.path().join("work/answer.txt"), "wrong").unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let failed = wait(&app, id);
    assert_eq!(
        failed["chapters"]["ex-1"]["lastCheckResults"][0]["status"],
        "failed"
    );
    assert_eq!(failed["completedExercises"], 1);
    assert_eq!(
        send(&app, "listRecents", Value::Null)["payload"][0]["completedExercises"],
        1
    );
}

#[test]
fn running_checks_blocks_command_blocks_until_verification_finishes() {
    let (temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"slow","title":"wait for the runner","type":"command_test","command":"while [ ! -f release ]; do sleep 0.01; done"}
        ]}]),
        "# Course\n\n## Exercise 1\n\n```sh\nprintf command\n```\n",
    );
    assert_eq!(
        send(
            &app,
            "runChecks",
            json!({"workspaceId":id,"chapterId":"ex-1"}),
        )["type"],
        "verificationStarted"
    );

    let blocked = app
        .execute(
            serde_json::from_value::<AppCommand>(json!({
                "type":"runCommandBlock",
                "payload":{"workspaceId":id,"commandId":"ex-1:cmd-1"}
            }))
            .unwrap(),
        )
        .unwrap_err();
    assert_eq!(blocked.code, ErrorCode::VerificationBusy);

    fs::write(temp.path().join("work/release"), "done").unwrap();
    wait(&app, id);
    assert_eq!(
        send(
            &app,
            "runCommandBlock",
            json!({"workspaceId":id,"commandId":"ex-1:cmd-1"}),
        )["type"],
        "commandStarted"
    );
}

#[test]
fn http_contract_checks_request_and_response_then_reports_expected_actual() {
    use std::io::{Read, Write};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = vec![0; 8192];
            let count = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..count]);
            assert!(request.starts_with("POST /tasks"));
            assert!(
                request
                    .to_lowercase()
                    .contains("content-type: application/json")
            );
            let body = "{\"status\":\"pending\"}";
            write!(stream,"HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        }
    });
    let (_temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"http","title":"create","type":"http_assert","request":{"method":"POST","url":format!("http://{address}/tasks"),"headers":{"Content-Type":"application/json"},"body":"{\"title\":\"learn\"}"},"expect":{"status":201,"headers":{"content-type":"application/json"},"bodyContains":"pending"}},
            {"id":"wrong","title":"mismatch","type":"http_assert","request":{"method":"POST","url":format!("http://{address}/tasks"),"headers":{"Content-Type":"application/json"}},"expect":{"status":200,"bodyContains":"missing"}}
        ]}]),
        "# Course\n\n## Exercise 1\n\n### Check\n",
    );
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait(&app, id);
    let results = &progress["chapters"]["ex-1"]["lastCheckResults"];
    assert_eq!(results[0]["status"], "passed");
    assert_eq!(results[1]["status"], "failed");
    assert_eq!(
        results[1]["diffs"][0],
        json!({"label":"status","expected":"200","actual":"201"})
    );
    assert_eq!(results[1]["diffs"][1]["expected"], "missing");
    server.join().unwrap();
}

#[test]
fn go_test_adapter_exposes_real_test_cases_and_failure_output() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[{"id":"go","title":"Go tests","type":"test_suite"}]}]),
        "# Course\n\n## Exercise 1\n\n### Check\n",
        Some("go"),
    );
    let root = temp.path().join("work");
    fs::write(root.join("go.mod"), "module seam.test/check\n\ngo 1.22\n").unwrap();
    fs::write(root.join("main_test.go"),"package check\nimport \"testing\"\nfunc TestContract(t *testing.T) { t.Error(\"expected: 201 actual: 200\") }\n").unwrap();
    let progress = run_go_checks(&app, id, "ex-1");
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "failed", "{result}");
    assert_eq!(
        result["testCases"][0]["name"], "seam.test/check/TestContract",
        "{result}"
    );
    assert_eq!(result["testCases"][0]["status"], "failed");
    assert!(
        result["testCases"][0]["output"]
            .as_str()
            .unwrap()
            .contains("expected: 201 actual: 200")
    );
}

#[test]
fn requires_daemon_starts_then_restarts_after_app_saves_without_killing_external_listener() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let checks = json!([{"chapterId":"ex-1","requiresDaemon":{"label":"API","port":port,"serviceKey":"api"},"checks":[{"id":"http","title":"fresh server","type":"http_assert","request":{"url":format!("http://127.0.0.1:{port}/")},"expect":{"bodyContains":"fresh"}}]}]);
    let command = service_command(port, "from_source");
    let (temp, app, id) = fixture(
        checks,
        &format!(
            "# Course\n\n## Exercise 1\n\n```sh mode=daemon label=API port={port}\n{command}\n```\n"
        ),
    );
    let code = "fresh";
    fs::write(temp.path().join("work/server.py"), code).unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    assert_eq!(
        wait(&app, id)["chapters"]["ex-1"]["lastCheckResults"][0]["status"],
        "passed"
    );
    let first =
        send(&app, "readTerminals", json!({"workspaceId":id}))["payload"]["terminals"][0].clone();
    let file = send(
        &app,
        "readWorkspaceFile",
        json!({"workspaceId":id,"path":"server.py"}),
    )["payload"]
        .clone();
    send(
        &app,
        "writeWorkspaceFile",
        json!({"workspaceId":id,"path":"server.py","text":code.replace("fresh","stale"),"expectedRevision":file["revision"]}),
    );
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    assert_eq!(
        wait(&app, id)["chapters"]["ex-1"]["lastCheckResults"][0]["status"],
        "failed"
    );
    let second =
        send(&app, "readTerminals", json!({"workspaceId":id}))["payload"]["terminals"][0].clone();
    assert_ne!(first["startedAt"], second["startedAt"]);
    send(
        &app,
        "stopTerminal",
        json!({"workspaceId":id,"terminalId":second["id"]}),
    );
    let external = wait_port_released(port);
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let result = wait(&app, id)["chapters"]["ex-1"]["lastCheckResults"][0].clone();
    assert_eq!(result["error"]["code"], "DAEMON_PORT_CONFLICT");
    assert!(external.local_addr().is_ok());
}

#[cfg(unix)]
#[test]
fn timeout_interrupts_and_kills_the_entire_headless_process_group() {
    use std::net::TcpStream;
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let command = format!(
        "trap '' INT; python3 -c \"import socket,time,signal; signal.signal(signal.SIGINT,signal.SIG_IGN); s=socket.socket(); s.bind(('127.0.0.1',{port})); s.listen(); time.sleep(60)\" & wait"
    );
    let (_temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[{"id":"timeout","title":"deadline","type":"command_test","command":command,"timeoutSec":1}]}]),
        "# Course\n\n## Exercise 1\n",
    );
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let until = Instant::now() + Duration::from_secs(1);
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    let result = wait(&app, id);
    assert_eq!(
        result["chapters"]["ex-1"]["lastCheckResults"][0]["status"],
        "timedOut"
    );
    wait_port_released(port);
}

#[cfg(windows)]
mod windows_timeout {
    use super::*;
    use std::{
        os::windows::process::CommandExt,
        process::Command,
        sync::atomic::{AtomicBool, Ordering},
        thread,
    };
    use windows_sys::Win32::System::{
        Console::{CTRL_BREAK_EVENT, GetConsoleCP, SetConsoleCtrlHandler},
        Threading::DETACHED_PROCESS,
    };

    const DRIVER: &str = "CODELAB_TEST_DETACHED_DRIVER";
    const LEAF: &str = "CODELAB_TEST_TIMEOUT_LEAF";
    const CHILD_TEST: &str = "windows_timeout::timeout_child";
    static INTERRUPTED: AtomicBool = AtomicBool::new(false);

    #[test]
    fn timeout_delivers_interrupt_from_an_app_without_a_console() {
        if run_in_detached_driver(
            "windows_timeout::timeout_delivers_interrupt_from_an_app_without_a_console",
        ) {
            return;
        }
        let (_temp, result) = time_out("graceful");
        assert!(
            result["rawOutput"]
                .as_str()
                .unwrap()
                .contains("console interrupt handled"),
            "{result}"
        );
    }

    #[test]
    fn timeout_reclaims_descendants_that_ignore_the_interrupt() {
        if run_in_detached_driver(
            "windows_timeout::timeout_reclaims_descendants_that_ignore_the_interrupt",
        ) {
            return;
        }
        let (temp, result) = time_out("stubborn");
        let port: u16 = fs::read_to_string(temp.path().join("work/listener-port.txt"))
            .unwrap_or_else(|error| panic!("descendant did not start: {error}; {result}"))
            .parse()
            .unwrap();
        wait_port_released(port);
    }

    fn run_in_detached_driver(test: &str) -> bool {
        if std::env::var_os(DRIVER).is_some() {
            assert_eq!(unsafe { GetConsoleCP() }, 0, "driver has a console");
            return false;
        }
        // Match the no-console startup of the release desktop app in a new process.
        let result = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env(DRIVER, "1")
            .creation_flags(DETACHED_PROCESS)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}\n{}",
            result.status,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        true
    }

    fn time_out(mode: &str) -> (TempDir, Value) {
        let executable = std::env::current_exe().unwrap();
        let executable = dunce::simplified(&executable)
            .to_string_lossy()
            .replace('\\', "/")
            .replace('\'', "'\\''");
        let command = format!("exec '{executable}' --ignored --exact {CHILD_TEST} --nocapture");
        let (temp, app, id) = fixture(
            json!([{"chapterId":"ex-1","checks":[
                {"id":"timeout","title":"deadline","type":"command_test","command":command,"timeoutSec":3}
            ]}]),
            "# Course\n\n## Exercise 1\n",
        );
        fs::write(temp.path().join("work/timeout-mode.txt"), mode).unwrap();
        send(
            &app,
            "runChecks",
            json!({"workspaceId":id,"chapterId":"ex-1"}),
        );
        let result = wait(&app, id)["chapters"]["ex-1"]["lastCheckResults"][0].clone();
        assert_eq!(result["status"], "timedOut", "{result}");
        (temp, result)
    }

    #[test]
    #[ignore = "spawned by the Windows timeout tests"]
    fn timeout_child() {
        let mode = fs::read_to_string("timeout-mode.txt").unwrap();
        assert_ne!(unsafe { SetConsoleCtrlHandler(Some(handle_break), 1) }, 0);
        if mode == "graceful" {
            let until = Instant::now() + Duration::from_secs(15);
            while !INTERRUPTED.load(Ordering::Relaxed) && Instant::now() < until {
                thread::sleep(Duration::from_millis(5));
            }
            assert!(INTERRUPTED.load(Ordering::Relaxed));
            println!("console interrupt handled");
        } else if std::env::var_os(LEAF).is_none() {
            Command::new(std::env::current_exe().unwrap())
                .args(["--ignored", "--exact", CHILD_TEST, "--nocapture"])
                .env(LEAF, "1")
                .status()
                .unwrap();
        } else {
            let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
            fs::write(
                "listener-port.txt",
                listener.local_addr().unwrap().port().to_string(),
            )
            .unwrap();
            thread::sleep(Duration::from_secs(15));
        }
    }

    unsafe extern "system" fn handle_break(event: u32) -> i32 {
        if event == CTRL_BREAK_EVENT {
            INTERRUPTED.store(true, Ordering::Relaxed);
            1
        } else {
            0
        }
    }
}

#[test]
fn verification_trust_confirmation_and_conflicts_guard_execution_before_saving() {
    let (temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[{"id":"risk","title":"confirm","type":"command_test","command":"printf safe | sh"}]}]),
        "# Course\n\n## Exercise 1\n",
    );
    fs::write(temp.path().join("work/answer.txt"), "original").unwrap();
    let file = send(
        &app,
        "readWorkspaceFile",
        json!({"workspaceId":id,"path":"answer.txt"}),
    )["payload"]
        .clone();
    let mut request = json!({"workspaceId":id,"chapterId":"ex-1","drafts":[{"path":"answer.txt","text":"draft","expectedRevision":file["revision"]}]});
    let confirmation = send(&app, "runChecks", request.clone());
    assert_eq!(confirmation["type"], "commandConfirmationRequired");
    assert_eq!(
        send(
            &app,
            "readWorkspaceFile",
            json!({"workspaceId":id,"path":"answer.txt"})
        )["payload"]["text"],
        "original"
    );
    fs::write(temp.path().join("work/answer.txt"), "external").unwrap();
    request["confirmation"] = confirmation["payload"]["token"].clone();
    assert_eq!(
        send(&app, "runChecks", request.clone())["type"],
        "commandSaveConflict"
    );
    assert_eq!(
        send(&app, "runChecks", request)["type"],
        "commandConfirmationRequired"
    );
    send(
        &app,
        "revokeTrust",
        json!({"sourceKind":"localPath","source":temp.path().join("pack")}),
    );
    assert_eq!(
        app.execute(
            serde_json::from_value(
                json!({"type":"runChecks","payload":{"workspaceId":id,"chapterId":"ex-1"}})
            )
            .unwrap()
        )
        .unwrap_err()
        .code,
        codelab_core::ErrorCode::CourseUntrusted
    );
}

#[test]
fn required_self_checks_manual_completion_export_reset_and_reopening_are_independent() {
    let (temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[{"id":"auto","title":"pass","type":"command_test","command":"printf ok"},{"id":"self","title":"required","type":"self_check","required":true}]},{"chapterId":"ex-2","checks":[]}]),
        "# Course\n\n## Exercise 1\n\n## Exercise 2\n",
    );
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    assert_eq!(wait(&app, id)["completedExercises"], 0);
    assert_eq!(
        send(
            &app,
            "setSelfCheck",
            json!({"workspaceId":id,"chapterId":"ex-1","checkId":"self","checked":true})
        )["payload"]["completedExercises"],
        1
    );
    assert_eq!(
        send(
            &app,
            "completeExercise",
            json!({"workspaceId":id,"chapterId":"ex-2"})
        )["payload"]["completedExercises"],
        2
    );
    let record = send(&app, "exportLearningRecord", json!({"workspaceId":id}));
    assert_eq!(record["payload"]["progress"]["completedExercises"], 2);
    send(&app, "closeActiveCourse", Value::Null);
    let opened = send(
        &app,
        "openRecentWorkspace",
        json!({"path":temp.path().join("work")}),
    );
    let id = opened["payload"]["workspace"]["id"].as_u64().unwrap();
    assert_eq!(
        send(&app, "getProgress", json!({"workspaceId":id}))["payload"]["completedExercises"],
        2
    );
    assert_eq!(
        send(&app, "resetProgress", json!({"workspaceId":id}))["payload"]["completedExercises"],
        0
    );
    assert_eq!(
        send(&app, "listRecents", Value::Null)["payload"][0]["completedExercises"],
        0
    );
}

#[test]
fn builtin_exercises_zero_through_nine_expose_executable_complete_check_definitions() {
    let temp = TempDir::new().unwrap();
    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let course = send(
        &app,
        "openCourse",
        json!({"kind":"builtin","courseId":"go-goframe-practice"}),
    )["payload"]
        .clone();
    let exercises: Vec<_> = course["chapters"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["kind"] == "exercise")
        .collect();
    assert_eq!(exercises.len(), 10);
    for (index, chapter) in exercises.iter().enumerate() {
        assert_eq!(chapter["chapterId"], format!("ex-{index}"));
        let checks = chapter["threeStep"]["step3Checks"].as_array().unwrap();
        assert!(!checks.is_empty());
        if index > 0 {
            assert!(checks.iter().any(|c| c["kind"] != "selfCheck"));
        }
        for check in checks {
            if check["kind"] == "httpAssert" {
                assert!(check["expect"]["status"].as_u64().is_some());
            }
            if check["kind"] == "testSuite" {
                assert_eq!(check["command"], "go test -json -race ./...");
                assert_eq!(check["forbidOutput"][0], "WARNING: DATA RACE");
            }
        }
    }
}

#[test]
fn shell_risk_checks_preserve_each_check_command_parse_boundary() {
    let (_temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"open","title":"open quote","type":"command_test","command":"printf '"},
            {"id":"risk","title":"risk","type":"command_test","command":"printf safe | sh"},
            {"id":"close","title":"close quote","type":"command_test","command":"'"}
        ]}]),
        "# Course\n\n## Exercise 1\n",
    );
    assert_eq!(
        send(
            &app,
            "runChecks",
            json!({"workspaceId":id,"chapterId":"ex-1"})
        )["type"],
        "commandConfirmationRequired"
    );
}

#[test]
fn required_go_cases_prevent_an_untouched_starter_from_completing_later_exercises() {
    let temp = TempDir::new().unwrap();
    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    send(
        &app,
        "openCourse",
        json!({"kind":"builtin","courseId":"go-goframe-practice"}),
    );
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    let id = send(&app, "bindWorkspace", json!({"path":root}))["payload"]["id"]
        .as_u64()
        .unwrap();
    let progress = run_go_checks(&app, id, "ex-3");
    assert_eq!(progress["completedExercises"], 0);
    let result = &progress["chapters"]["ex-3"]["lastCheckResults"][0];
    assert_eq!(result["status"], "failed", "{result}");
    assert!(
        result["diffs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["label"] == "TestTaskHTTPContract")
    );
}

fn service_command(port: u16, body: &str) -> String {
    let executable = std::env::current_exe().unwrap();
    #[cfg(windows)]
    let executable = dunce::simplified(&executable);
    let executable = executable.to_string_lossy().replace('\\', "/");
    format!(
        "CODELAB_SEAM_PORT={port} CODELAB_SEAM_BODY={body} '{executable}' --exact verification_service_fixture --nocapture"
    )
}
#[test]
fn verification_service_fixture() {
    use std::io::{Read, Write};
    let Ok(port) = std::env::var("CODELAB_SEAM_PORT") else {
        return;
    };
    let listener = TcpListener::bind(("127.0.0.1", port.parse::<u16>().unwrap())).unwrap();
    let body = std::env::var("CODELAB_SEAM_BODY").unwrap();
    let body = match body.as_str() {
        "from_source" => fs::read_to_string("server.py").unwrap(),
        "from_text_source" => fs::read_to_string("server.txt").unwrap(),
        _ => body,
    };
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut bytes = [0; 4096];
        if stream.read(&mut bytes).unwrap_or(0) == 0 {
            continue;
        }
        if bytes.starts_with(b"GET /__stop-listener") {
            break;
        }
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
    }
    drop(listener);
    std::thread::park();
}
#[test]
fn different_managed_service_on_the_same_port_is_restarted_with_the_required_launch() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let first = service_command(port, "first");
    let second = service_command(port, "second");
    let checks = json!([{"chapterId":"ex-1","requiresDaemon":{"label":"first","port":port},"checks":[{"id":"http","title":"first service","type":"http_assert","request":{"url":format!("http://127.0.0.1:{port}")},"expect":{"bodyEquals":"first"}}]}, {"chapterId":"ex-2","requiresDaemon":{"label":"second","port":port},"checks":[{"id":"http","title":"second service","type":"http_assert","request":{"url":format!("http://127.0.0.1:{port}")},"expect":{"bodyEquals":"second"}}]}]);
    let (_temp, app, id) = fixture(
        checks,
        &format!(
            "# Course\n\n## Exercise 1\n\n```sh mode=daemon label=first port={port}\n{first}\n```\n\n## Exercise 2\n\n```sh mode=daemon label=second port={port}\n{second}\n```\n"
        ),
    );
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    assert_eq!(
        wait(&app, id)["chapters"]["ex-1"]["lastCheckResults"][0]["status"],
        "passed"
    );
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-2"}),
    );
    assert_eq!(
        wait(&app, id)["chapters"]["ex-2"]["lastCheckResults"][0]["status"],
        "passed"
    );
}
#[test]
fn shutdown_reclaims_an_active_headless_service_and_reopening_recovers_interrupted_results() {
    use std::net::TcpStream;
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let (temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[{"id":"long","title":"owned process","type":"command_test","command":service_command(port,"active"),"timeoutSec":60}]}]),
        "# Course\n\n## Exercise 1\n",
    );
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let until = Instant::now() + Duration::from_secs(10);
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    app.shutdown().unwrap();
    wait_port_released(port);
    let reopened = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let response = send(
        &reopened,
        "openRecentWorkspace",
        json!({"path":temp.path().join("work")}),
    );
    let id = response["payload"]["workspace"]["id"].as_u64().unwrap();
    assert_eq!(
        send(&reopened, "getProgress", json!({"workspaceId":id}))["payload"]["chapters"]["ex-1"]["lastCheckResults"]
            [0]["status"],
        "failed"
    );
}

#[test]
fn reloading_the_same_course_recovers_interrupted_checks_in_the_retained_workspace() {
    use std::net::TcpStream;

    let port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let (temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[{"id":"long","title":"owned process","type":"command_test","command":service_command(port,"active"),"timeoutSec":60}]}]),
        "# Course\n\n## Exercise 1\n",
    );
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let until = Instant::now() + Duration::from_secs(10);
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(Instant::now() < until, "verification process did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
    let events = app.subscribe();
    let response = send(&app, "reloadActiveCourse", Value::Null);
    assert_eq!(response["type"], "loadedCourse", "{response}");
    let event = events.recv_timeout(Duration::from_secs(1)).unwrap();
    let codelab_core::AppEvent::ProgressUpdated(progress) = event else {
        panic!("expected recovered progress event, got {event:?}");
    };
    assert_eq!(
        progress.chapters["ex-1"].last_check_results[0].status,
        codelab_core::CheckStatus::Failed
    );

    let progress = send(&app, "getProgress", json!({"workspaceId":id}));
    let result = &progress["payload"]["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "failed");
    assert_eq!(
        result["error"]["details"]["reasonCode"],
        "VERIFICATION_INTERRUPTED"
    );
    assert_eq!(progress["payload"]["runningChapterId"], Value::Null);

    drop(app);
    let reopened = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    send(
        &reopened,
        "openRecentWorkspace",
        json!({"path":temp.path().join("work")}),
    );
    let persisted = send(&reopened, "getProgress", json!({"workspaceId":id}));
    assert_eq!(
        persisted["payload"]["chapters"]["ex-1"]["lastCheckResults"][0]["status"],
        "failed"
    );
}

#[test]
fn contradictory_duplicate_results_are_quarantined_on_reopen() {
    let (temp, app, _id) = fixture(
        json!([{"chapterId":"ex-1","checks":[]}]),
        "# Course\n\n## Exercise 1\n",
    );
    send(&app, "closeActiveCourse", Value::Null);
    let result = json!({"checkId":"same","status":"passed","durationMs":0,"diffs":[],"rawOutput":"","testCases":[],"error":null});
    let mut contradictory = result.clone();
    contradictory["status"] = json!("failed");
    fs::write(temp.path().join("work/.codelab/progress.json"),json!({"schemaVersion":1,"data":{"chapters":{"ex-1":{"completionState":"in_progress","lastCheckResults":[result,contradictory]}}}}).to_string()).unwrap();
    let opened = send(
        &app,
        "openRecentWorkspace",
        json!({"path":temp.path().join("work")}),
    );
    assert!(
        opened["payload"]["workspace"]["recoveryNotices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["store"] == "progress")
    );
    let id = opened["payload"]["workspace"]["id"].as_u64().unwrap();
    assert_eq!(
        send(&app, "getProgress", json!({"workspaceId":id}))["payload"]["chapters"],
        json!({})
    );
}

#[test]
fn skipped_required_go_case_cannot_complete_an_exercise() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[{"id":"go","title":"required","type":"test_suite","command":"go test -json -race ./...","requireTests":["TestRequired"]}]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    fs::write(
        temp.path().join("work/go.mod"),
        "module seam.test/skip\n\ngo 1.22\n",
    )
    .unwrap();
    fs::write(temp.path().join("work/main_test.go"),"package skip\nimport \"testing\"\nfunc TestRequired(t *testing.T) { t.Skip(\"not implemented\") }\n").unwrap();
    let progress = run_go_checks(&app, id, "ex-1");
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "failed", "{result}");
    assert_eq!(result["testCases"][0]["status"], "skipped", "{result}");
    assert_eq!(progress["completedExercises"], 0);
}

#[test]
fn daemon_label_takes_precedence_over_an_earlier_block_with_the_same_port() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let first = service_command(port, "first");
    let second = service_command(port, "second");
    let (_temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","requiresDaemon":{"label":"second","port":port},"checks":[{"id":"http","title":"selected service","type":"http_assert","request":{"url":format!("http://127.0.0.1:{port}")},"expect":{"bodyEquals":"second"}}]}]),
        &format!(
            "# Course\n\n## Exercise 1\n\n```sh mode=daemon label=first port={port}\n{first}\n```\n\n```sh mode=daemon label=second port={port}\n{second}\n```\n"
        ),
    );
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    assert_eq!(
        wait(&app, id)["chapters"]["ex-1"]["lastCheckResults"][0]["status"],
        "passed"
    );
}

#[test]
fn custom_go_suite_uses_the_course_adapter_and_disambiguates_required_cases() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"ambiguous","title":"Short name","type":"test_suite","command":"sh -c 'sh verify.sh'","cwd":"root-suite","requireTests":["TestSave"]},
            {"id":"qualified","title":"Full name","type":"test_suite","command":"sh verify-one.sh","cwd":"single-suite/a","requireTests":["seam.test/single/a/TestSave"]}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    let root = temp.path().join("work");
    for (dir, module) in [("root-suite", "suites"), ("single-suite", "single")] {
        let suite = root.join(dir);
        fs::create_dir_all(&suite).unwrap();
        fs::write(
            suite.join("go.mod"),
            format!("module seam.test/{module}\n\ngo 1.22\n"),
        )
        .unwrap();
        for package in ["a", "b"] {
            fs::create_dir(suite.join(package)).unwrap();
            fs::write(
                suite.join(package).join("save_test.go"),
                format!(
                    "package {package}\nimport \"testing\"\nfunc TestSave(t *testing.T) {{}}\n"
                ),
            )
            .unwrap();
        }
    }
    fs::write(
        root.join("root-suite/verify.sh"),
        "printf 'both packages\\n'\ngo test -json ./...\n",
    )
    .unwrap();
    fs::write(
        root.join("single-suite/a/verify-one.sh"),
        "printf 'one package\\n'\ngo test -json .\n",
    )
    .unwrap();
    let confirmation = send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    assert_eq!(confirmation["type"], "commandConfirmationRequired");
    assert_eq!(
        confirmation["payload"]["command"],
        "sh -c 'sh verify.sh'\nsh verify-one.sh"
    );
    let started = send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1","confirmation":confirmation["payload"]["token"]}),
    );
    assert_eq!(started["type"], "verificationStarted", "{started}");
    let progress = wait_for_go_checks(&app, id);
    let results = &progress["chapters"]["ex-1"]["lastCheckResults"];
    assert_eq!(results[0]["status"], "failed", "{results}");
    assert!(
        results[0]["diffs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diff| diff["label"] == "TestSave" && diff["actual"] == "ambiguousTestName")
    );
    assert_eq!(results[1]["status"], "passed", "{results}");
    assert!(
        results[0]["rawOutput"]
            .as_str()
            .unwrap()
            .starts_with("both packages\n")
    );
    assert!(
        results[1]["rawOutput"]
            .as_str()
            .unwrap()
            .starts_with("one package\n")
    );
    assert_eq!(results[1]["testCases"].as_array().unwrap().len(), 1);
    assert_eq!(
        results[1]["testCases"][0]["name"],
        "seam.test/single/a/TestSave"
    );
    assert_eq!(progress["completedExercises"], 0);
}

#[test]
fn suite_report_requirements_do_not_apply_to_command_tests_and_timeout_is_preserved() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"plain","title":"Plain command","type":"command_test","command":"printf 'go test -json'"},
            {"id":"missing","title":"Missing report","type":"test_suite","command":"printf 'no report'"},
            {"id":"empty","title":"No tests passed","type":"test_suite","command":"cat skipped.jsonl"},
            {"id":"partial","title":"Unfinished package","type":"test_suite","command":"cat partial.jsonl"},
            {"id":"timeout","title":"Timed out","type":"test_suite","command":"sleep 10","timeoutSec":1,"requireTests":["TestSave"]}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    let root = temp.path().join("work");
    fs::write(
        root.join("skipped.jsonl"),
        concat!(
            "{\"Action\":\"start\",\"Package\":\"seam.test/skip\"}\n",
            "{\"Action\":\"run\",\"Package\":\"seam.test/skip\",\"Test\":\"TestSkipped\"}\n",
            "{\"Action\":\"skip\",\"Package\":\"seam.test/skip\",\"Test\":\"TestSkipped\"}\n",
            "{\"Action\":\"pass\",\"Package\":\"seam.test/skip\"}\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("partial.jsonl"),
        concat!(
            "{\"Action\":\"start\",\"Package\":\"seam.test/a\"}\n",
            "{\"Action\":\"start\",\"Package\":\"seam.test/b\"}\n",
            "{\"Action\":\"run\",\"Package\":\"seam.test/a\",\"Test\":\"TestSave\"}\n",
            "{\"Action\":\"pass\",\"Package\":\"seam.test/a\",\"Test\":\"TestSave\"}\n",
            "{\"Action\":\"pass\",\"Package\":\"seam.test/a\"}\n",
        ),
    )
    .unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait(&app, id);
    let results = &progress["chapters"]["ex-1"]["lastCheckResults"];
    assert_eq!(results[0]["status"], "passed", "{results}");
    assert_eq!(results[0]["testCases"], json!([]));
    for index in [1, 2, 3] {
        assert_eq!(results[index]["status"], "failed", "{results}");
        assert_eq!(results[index]["diffs"][0]["label"], "goTestReport");
    }
    assert_eq!(results[2]["testCases"][0]["status"], "skipped");
    assert_eq!(results[4]["status"], "timedOut", "{results}");
    assert_eq!(progress["completedExercises"], 0);
}

#[test]
fn unsupported_suite_reports_a_check_error_without_requesting_consent_or_running_its_command() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"suite","title":"Unsupported","type":"test_suite","command":"printf 'printf ran > unwanted.txt' | sh"},
            {"id":"command","title":"Other Check","type":"command_test","command":"printf checked"}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        None,
    );
    let response = send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    assert_eq!(response["type"], "verificationStarted", "{response}");
    let progress = wait(&app, id);
    let results = &progress["chapters"]["ex-1"]["lastCheckResults"];
    assert_eq!(results[0]["status"], "failed", "{results}");
    assert_eq!(
        results[0]["error"],
        json!({"code":"TEST_SUITE_ADAPTER_UNAVAILABLE","details":{"checkId":"suite"}})
    );
    assert_eq!(results[1]["status"], "passed", "{results}");
    assert!(!temp.path().join("work/unwanted.txt").exists());
}

#[cfg(unix)]
#[test]
fn default_go_suite_uses_the_selected_managed_runtime() {
    use std::os::unix::fs::PermissionsExt;

    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"go","title":"Default Go Test Suite","type":"test_suite","requireTests":["TestManaged"]}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    let catalog: Value =
        serde_json::from_str(include_str!("../src/runtime/artifacts.json")).unwrap();
    let version = catalog["goVersion"].as_str().unwrap();
    let go_root = temp
        .path()
        .join(format!("data/runtimes/go-{version}/toolchain"));
    let executable = go_root.join("bin/go");
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::write(
        &executable,
        concat!(
            "#!/bin/sh\n",
            "case \"$*\" in\n",
            "  'env GOPROXY') printf 'off\\n' ;;\n",
            "  'test -json -race ./...')\n",
            "    test \"$GOROOT\" = \"${0%/bin/go}\" || exit 3\n",
            "    printf '%s\\n' ",
            "'{\"Action\":\"start\",\"Package\":\"seam.test/managed\"}' ",
            "'{\"Action\":\"run\",\"Package\":\"seam.test/managed\",\"Test\":\"TestManaged\"}' ",
            "'{\"Action\":\"pass\",\"Package\":\"seam.test/managed\",\"Test\":\"TestManaged\"}' ",
            "'{\"Action\":\"pass\",\"Package\":\"seam.test/managed\"}' ;;\n",
            "  *) exit 2 ;;\n",
            "esac\n",
        ),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

    let events = app.subscribe();
    send(
        &app,
        "setRuntimePreference",
        json!({"language":"Go","preferManaged":true}),
    );
    send(&app, "refreshPreflight", Value::Null);
    loop {
        if let codelab_core::AppEvent::PreflightUpdated(report) =
            events.recv_timeout(Duration::from_secs(10)).unwrap()
            && report.checked
            && !report.probing
        {
            let go = report
                .languages
                .iter()
                .find(|row| row.language == codelab_core::CourseLanguage::Go)
                .unwrap();
            assert_eq!(go.selected, Some(codelab_core::RuntimeSource::Managed));
            break;
        }
    }

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait_for_go_checks(&app, id);
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "passed", "{result}");
    assert_eq!(
        result["testCases"][0]["name"],
        "seam.test/managed/TestManaged"
    );
    assert_eq!(progress["completedExercises"], 1);
}

#[test]
fn incomplete_go_reports_cannot_complete_exercises() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"terminals","title":"Only terminal events","type":"test_suite","command":"cat terminals.jsonl"},
            {"id":"unstarted","title":"Test never started","type":"test_suite","command":"cat unstarted.jsonl"},
            {"id":"truncated","title":"Truncated next event","type":"test_suite","command":"cat truncated.jsonl"},
            {"id":"spanning","title":"Case spans package invocations","type":"test_suite","command":"cat spanning.jsonl"},
            {"id":"reordered","title":"Truncated reordered event","type":"test_suite","command":"cat reordered.jsonl"},
            {"id":"structuredLog","title":"Complete report with structured wrapper logging","type":"test_suite","command":"cat structured-log.jsonl"},
            {"id":"complete","title":"Complete report with wrapper logging","type":"test_suite","command":"cat complete.jsonl"}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    let root = temp.path().join("work");
    let start = "{\"Action\":\"start\",\"Package\":\"seam.test/report\"}\n";
    let run = "{\"Action\":\"run\",\"Package\":\"seam.test/report\",\"Test\":\"TestSave\"}\n";
    let pass = concat!(
        "{\"Action\":\"pass\",\"Package\":\"seam.test/report\",\"Test\":\"TestSave\"}\n",
        "{\"Action\":\"pass\",\"Package\":\"seam.test/report\"}\n",
    );
    fs::write(root.join("terminals.jsonl"), pass).unwrap();
    fs::write(root.join("unstarted.jsonl"), format!("{start}{pass}")).unwrap();
    fs::write(
        root.join("truncated.jsonl"),
        format!("{start}{run}{pass}{{\"Action\":\"start\",\"Package\":\"seam.test/next"),
    )
    .unwrap();
    let package_pass = "{\"Action\":\"pass\",\"Package\":\"seam.test/report\"}\n";
    fs::write(
        root.join("spanning.jsonl"),
        format!("{start}{run}{package_pass}{start}{pass}"),
    )
    .unwrap();
    fs::write(
        root.join("reordered.jsonl"),
        format!("{start}{run}{pass}{{\"Test\":\"TestOther\",\"Action\":\"run\",\"Package\":"),
    )
    .unwrap();
    fs::write(
        root.join("structured-log.jsonl"),
        format!(
            "{start}{run}{pass}{{\"Time\":\"wrapper\",\"Action\":\"cleanup\",\"status\":\"ok\"}}\n"
        ),
    )
    .unwrap();
    fs::write(
        root.join("complete.jsonl"),
        format!(
            "wrapper log\n{{\"status\":\"starting tests\"}}\n{start}{run}{}{}{}",
            "{\"Action\":\"attr\",\"Package\":\"seam.test/report\",\"Test\":\"TestSave\",\"Key\":\"kind\",\"Value\":\"example\"}\n",
            "{\"Action\":\"artifacts\",\"Package\":\"seam.test/report\",\"Test\":\"TestSave\",\"Path\":\"test-artifacts\"}\n",
            pass,
        ),
    )
    .unwrap();

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait(&app, id);
    let results = progress["chapters"]["ex-1"]["lastCheckResults"]
        .as_array()
        .unwrap();
    let statuses: Vec<_> = results
        .iter()
        .map(|result| result["status"].as_str().unwrap())
        .collect();
    assert_eq!(
        statuses,
        [
            "failed", "failed", "failed", "failed", "failed", "passed", "passed"
        ]
    );
    assert_eq!(progress["completedExercises"], 0);
}

#[test]
fn go_suite_keeps_package_and_subtest_identities_distinct() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"absent","title":"Missing qualified case","type":"test_suite","command":"cat names.jsonl","requireTests":["seam.test/app/TestSave"]},
            {"id":"joined","title":"Ambiguous joined name","type":"test_suite","command":"cat names.jsonl","requireTests":["outer.test/root/seam.test/app/TestSuite/TestSave"]},
            {"id":"native","title":"Native subtest name","type":"test_suite","command":"cat names.jsonl","requireTests":["TestSuite/TestSave"]},
            {"id":"explicit","title":"Explicit package and test","type":"test_suite","command":"cat names.jsonl","requireTests":["outer.test/root/seam.test/app/TestSuite::TestSave"]}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    let report = [
        json!({"Action":"start","Package":"outer.test/root/seam.test/app"}),
        json!({"Action":"start","Package":"outer.test/root/seam.test/app/TestSuite"}),
        json!({"Action":"run","Package":"outer.test/root/seam.test/app","Test":"TestSave"}),
        json!({"Action":"run","Package":"outer.test/root/seam.test/app","Test":"TestSuite"}),
        json!({"Action":"run","Package":"outer.test/root/seam.test/app","Test":"TestSuite/TestSave"}),
        json!({"Action":"run","Package":"outer.test/root/seam.test/app","Test":"TestSuite::TestSave"}),
        json!({"Action":"run","Package":"outer.test/root/seam.test/app/TestSuite","Test":"TestSave"}),
        json!({"Action":"pass","Package":"outer.test/root/seam.test/app","Test":"TestSave"}),
        json!({"Action":"pass","Package":"outer.test/root/seam.test/app","Test":"TestSuite/TestSave"}),
        json!({"Action":"pass","Package":"outer.test/root/seam.test/app","Test":"TestSuite::TestSave"}),
        json!({"Action":"pass","Package":"outer.test/root/seam.test/app","Test":"TestSuite"}),
        json!({"Action":"pass","Package":"outer.test/root/seam.test/app/TestSuite","Test":"TestSave"}),
        json!({"Action":"pass","Package":"outer.test/root/seam.test/app"}),
        json!({"Action":"pass","Package":"outer.test/root/seam.test/app/TestSuite"}),
    ].map(|event| event.to_string()).join("\n");
    fs::write(temp.path().join("work/names.jsonl"), report).unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let result = wait(&app, id);
    let results = &result["chapters"]["ex-1"]["lastCheckResults"];
    assert_eq!(
        results[0]["diffs"][0]["actual"], "missingOrFailed",
        "{results}"
    );
    assert_eq!(
        results[1]["diffs"][0]["actual"], "ambiguousTestName",
        "{results}"
    );
    assert_eq!(results[2]["status"], "passed", "{results}");
    assert_eq!(results[3]["status"], "passed", "{results}");
    let names: Vec<_> = results[3]["testCases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|test| test["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 5, "{names:?}");
    assert!(names.contains(&"outer.test/root/seam.test/app::TestSuite/TestSave"));
    assert!(names.contains(&"outer.test/root/seam.test/app/TestSuite::TestSave"));
}

#[test]
fn repeated_go_case_keeps_a_pass_when_a_later_run_skips() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"repeat","title":"Repeated test","type":"test_suite","command":"go test -json -count=2 ./...","requireTests":["TestSave"]}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    let root = temp.path().join("work");
    fs::write(root.join("go.mod"), "module seam.test/repeat\n\ngo 1.22\n").unwrap();
    fs::write(
        root.join("save_test.go"),
        "package repeat\nimport \"testing\"\nvar calls int\nfunc TestSave(t *testing.T) {\n calls++\n if calls > 1 { t.Skip(\"second run\") }\n}\n",
    )
    .unwrap();

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait_for_go_checks(&app, id);
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "passed", "{result}");
    assert_eq!(result["testCases"][0]["status"], "passed");
    assert_eq!(progress["completedExercises"], 1);
}

#[test]
fn required_go_subtest_names_can_contain_double_colons() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"named","title":"Native and qualified names","type":"test_suite","command":"go test -json ./...","requireTests":[
                "TestSave/key::value", "seam.test/literal/TestSave/key::value", "seam.test/literal::TestSave/key::value"
            ]}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    let root = temp.path().join("work");
    fs::write(root.join("go.mod"), "module seam.test/literal\n\ngo 1.22\n").unwrap();
    fs::write(
        root.join("save_test.go"),
        "package literal\nimport \"testing\"\nfunc TestSave(t *testing.T) { t.Run(\"key::value\", func(t *testing.T) {}) }\n",
    )
    .unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait_for_go_checks(&app, id);
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "passed", "{}", result["diffs"]);
}

#[test]
fn passing_go_tests_can_share_a_report_with_benchmarks() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"benchmarks","title":"Tests and benchmarks","type":"test_suite","command":"go test -json -bench=. -benchtime=1x ./...","requireTests":["TestSave"]}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    let root = temp.path().join("work");
    fs::write(
        root.join("go.mod"),
        "module seam.test/benchmark\n\ngo 1.22\n",
    )
    .unwrap();
    fs::write(
        root.join("save_test.go"),
        "package benchmark\nimport \"testing\"\nfunc TestSave(t *testing.T) {}\nfunc BenchmarkSave(b *testing.B) { b.Run(\"child\", func(b *testing.B) {}) }\nfunc BenchmarkLogged(b *testing.B) { b.Log(\"one run\") }\n",
    )
    .unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait_for_go_checks(&app, id);
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "passed", "{}", result["diffs"]);
    assert_eq!(progress["completedExercises"], 1);
}

#[cfg(unix)]
#[test]
fn go_suite_requires_new_confirmation_when_resolved_working_directory_changes() {
    use std::os::unix::fs::symlink;

    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"go","title":"Check a selected directory","type":"test_suite","cwd":"selected","command":"sh -c 'cat report.jsonl'"}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    let root = temp.path().join("work");
    for (directory, test) in [("first", "TestFirst"), ("second", "TestSecond")] {
        let package = format!("seam.test/{directory}");
        let report = [
            json!({"Action":"start","Package":package}),
            json!({"Action":"run","Package":package,"Test":test}),
            json!({"Action":"pass","Package":package,"Test":test}),
            json!({"Action":"pass","Package":package}),
        ]
        .map(|event| event.to_string())
        .join("\n");
        fs::create_dir(root.join(directory)).unwrap();
        fs::write(root.join(directory).join("report.jsonl"), report).unwrap();
    }
    let selected = root.join("selected");
    symlink("first", &selected).unwrap();

    let mut request = json!({"workspaceId":id,"chapterId":"ex-1"});
    let first = send(&app, "runChecks", request.clone());
    assert_eq!(first["type"], "commandConfirmationRequired");

    fs::remove_file(&selected).unwrap();
    symlink("second", &selected).unwrap();
    request["confirmation"] = first["payload"]["token"].clone();
    let second = send(&app, "runChecks", request.clone());
    assert_eq!(second["type"], "commandConfirmationRequired");
    assert_ne!(first["payload"]["token"], second["payload"]["token"]);

    request["confirmation"] = second["payload"]["token"].clone();
    assert_eq!(
        send(&app, "runChecks", request)["type"],
        "verificationStarted"
    );
    let progress = wait(&app, id);
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "passed", "{result}");
    assert_eq!(
        result["testCases"][0]["name"],
        "seam.test/second/TestSecond"
    );
}

#[test]
fn go_suite_reassembles_benchmark_timing_split_across_output_events() {
    let (temp, app, id) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[
            {"id":"benchmarks","title":"Split benchmark timing","type":"test_suite","command":"cat benchmark.jsonl","requireTests":["TestSave"]}
        ]}]),
        "# Course\n\n## Exercise 1\n",
        Some("go"),
    );
    // This stream includes the timing line split by the Ubuntu Go runner.
    let report = r#"
{"Action":"start","Package":"seam.test/benchmark"}
{"Action":"run","Package":"seam.test/benchmark","Test":"TestSave"}
{"Action":"pass","Package":"seam.test/benchmark","Test":"TestSave","Elapsed":0}
{"Action":"run","Package":"seam.test/benchmark","Test":"BenchmarkSave"}
{"Action":"output","Package":"seam.test/benchmark","Test":"BenchmarkSave","Output":"BenchmarkSave\n"}
{"Action":"run","Package":"seam.test/benchmark","Test":"BenchmarkSave/child"}
{"Action":"output","Package":"seam.test/benchmark","Test":"BenchmarkSave/child","Output":"BenchmarkSave/child\n"}
{"Action":"output","Package":"seam.test/benchmark","Test":"BenchmarkSave/child","Output":"BenchmarkSave/child-2         \t       1\t       500.0 ns/op\n"}
{"Action":"run","Package":"seam.test/benchmark","Test":"BenchmarkLogged"}
{"Action":"output","Package":"seam.test/benchmark","Test":"BenchmarkLogged","Output":"BenchmarkLogged\n"}
{"Action":"output","Package":"seam.test/benchmark","Test":"BenchmarkLogged","Output":"BenchmarkLogged-2             \t"}
{"Action":"output","Package":"seam.test/benchmark","Test":"BenchmarkLogged","Output":"       1\t     90023 ns/op\n"}
{"Action":"pass","Package":"seam.test/benchmark","Elapsed":0.023}
"#;
    fs::write(temp.path().join("work/benchmark.jsonl"), report).unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait(&app, id);
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "passed", "{}", result["diffs"]);
    assert_eq!(progress["completedExercises"], 1);
}

#[test]
fn verification_reuses_live_managed_daemon_after_an_older_terminal_stops() {
    use std::net::TcpStream;

    let port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let command = service_command(port, "fresh");
    let checks = json!([{"chapterId":"ex-1","requiresDaemon":{"label":"API","port":port},"checks":[
        {"id":"http","title":"Uses running Managed Daemon","type":"http_assert","request":{"url":format!("http://127.0.0.1:{port}/")},"expect":{"bodyEquals":"fresh"}}
    ]}]);
    let (_temp, app, id) = fixture(
        checks,
        &format!(
            "# Course\n\n## Exercise 1\n\n{fence}sh mode=daemon label=API port={port}\n{command}\n{fence}\n",
            fence = "\x60\x60\x60"
        ),
    );
    let start = || {
        let response = send(
            &app,
            "runCommandBlock",
            json!({"workspaceId":id,"commandId":"ex-1:cmd-1"}),
        );
        assert_eq!(response["type"], "commandStarted", "{response}");
        response["payload"]["terminal"].clone()
    };
    let wait_for_port = || {
        let until = Instant::now() + Duration::from_secs(5);
        while TcpStream::connect(("127.0.0.1", port)).is_err() {
            assert!(Instant::now() < until, "daemon did not become ready");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    let first = start();
    wait_for_port();
    send(
        &app,
        "stopTerminal",
        json!({"workspaceId":id,"terminalId":first["id"]}),
    );
    drop(wait_port_released(port));
    let second = start();
    assert_ne!(first["id"], second["id"]);
    wait_for_port();

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait(&app, id);
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "passed", "{result}");
    let terminals = send(&app, "readTerminals", json!({"workspaceId":id}));
    let running = terminals["payload"]["terminals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|terminal| terminal["running"] == true)
        .unwrap();
    assert_eq!(running["id"], second["id"]);
    assert_eq!(running["startedAt"], second["startedAt"]);
}

#[test]
fn verification_restarts_a_live_managed_daemon_when_its_port_is_closed() {
    use std::io::Write;
    use std::net::TcpStream;

    let port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let command = service_command(port, "fresh");
    let checks = json!([{"chapterId":"ex-1","requiresDaemon":{"label":"API","port":port},"checks":[
        {"id":"http","title":"Restarts stopped listener","type":"http_assert","request":{"url":format!("http://127.0.0.1:{port}/")},"expect":{"bodyEquals":"fresh"}}
    ]}]);
    let (_temp, app, id) = fixture(
        checks,
        &format!(
            "# Course\n\n## Exercise 1\n\n{fence}sh mode=daemon label=API port={port}\n{command}\n{fence}\n",
            fence = "\x60\x60\x60"
        ),
    );
    let first = send(
        &app,
        "runCommandBlock",
        json!({"workspaceId":id,"commandId":"ex-1:cmd-1"}),
    )["payload"]["terminal"]
        .clone();
    let ready_until = Instant::now() + Duration::from_secs(5);
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(Instant::now() < ready_until, "daemon did not become ready");
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .write_all(b"GET /__stop-listener HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    drop(stream);
    drop(wait_port_released(port));

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait(&app, id);
    assert_eq!(
        progress["chapters"]["ex-1"]["lastCheckResults"][0]["status"],
        "passed"
    );
    let terminals = send(&app, "readTerminals", json!({"workspaceId":id}));
    let running = terminals["payload"]["terminals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|terminal| terminal["running"] == true)
        .unwrap();
    assert_eq!(running["id"], first["id"]);
    assert_ne!(running["startedAt"], first["startedAt"]);
}

#[test]
fn partial_draft_writes_make_the_managed_daemon_stale() {
    for entry in ["runCommandBlock", "runChecks"] {
        let port = TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let command = service_command(port, "from_text_source");
        let checks = json!([{"chapterId":"ex-1","requiresDaemon":{"label":"API","port":port},"checks":[
            {"id":"http","title":"Reads fresh source","type":"http_assert","request":{"url":format!("http://127.0.0.1:{port}/")},"expect":{"bodyEquals":"updated"}}
        ]}]);
        let (temp, app, id) = fixture(
            checks,
            &format!(
                "# Course\n\n## Exercise 1\n\n{fence}sh mode=daemon label=API port={port}\n{command}\n{fence}\n",
                fence = "\x60\x60\x60"
            ),
        );
        let root = temp.path().join("work");
        // The watcher does not use .txt changes for freshness, so this tests
        // synchronous tracking by the Workspace write path.
        fs::write(root.join("server.txt"), "original").unwrap();
        fs::write(root.join("binary.txt"), [0, 255]).unwrap();
        let target = json!({"workspaceId":id,"chapterId":"ex-1"});
        send(&app, "runChecks", target.clone());
        let initial = wait(&app, id);
        let check = |progress: &Value| progress["chapters"]["ex-1"]["lastCheckResults"][0].clone();
        assert_eq!(check(&initial)["diffs"][0]["actual"], "original");

        let revision = |path| {
            send(
                &app,
                "readWorkspaceFile",
                json!({"workspaceId":id,"path":path}),
            )["payload"]["revision"]
                .clone()
        };
        let response = send(
            &app,
            entry,
            json!({
                "workspaceId":id,"chapterId":"ex-1","commandId":"ex-1:cmd-1","drafts":[
                    {"path":"server.txt","text":"updated","expectedRevision":revision("server.txt")},
                    {"path":"binary.txt","text":"cannot write","expectedRevision":revision("binary.txt")}
                ]
            }),
        );
        assert_eq!(response["type"], "commandFailed", "{entry} {response}");
        assert_eq!(
            response["payload"]["saved"][0]["revision"],
            revision("server.txt")
        );
        assert_eq!(response["payload"]["saved"][0]["path"], "server.txt");
        assert_eq!(fs::read(root.join("binary.txt")).unwrap(), [0, 255]);

        send(&app, "runChecks", target);
        let latest = check(&wait(&app, id));
        assert_eq!(latest["status"], "passed", "{entry} {latest}");
    }
}

#[test]
fn js_ts_adapter_runs_real_tests_extracts_expected_actual_diffs_and_syntax_errors() {
    let (temp, app, id) = fixture_with_language(
        json!([
            {"chapterId":"ex-1","checks":[
                {"id":"ts-pass","title":"JS/TS pass","type":"test_suite","command":"bun test tests/pass.test.ts","requireTests":["adds two numbers"]}
            ]},
            {"chapterId":"ex-2","checks":[
                {"id":"ts-fail","title":"JS/TS diff","type":"test_suite","command":"bun test tests/fail.test.ts"}
            ]},
            {"chapterId":"ex-3","checks":[
                {"id":"ts-syntax","title":"JS/TS syntax","type":"test_suite","command":"bun test tests/syntax.test.ts"}
            ]},
            {"chapterId":"ex-4","checks":[
                {"id":"node-fail","title":"Node TS diff","type":"test_suite","command":"node --test tests/fail.test.ts"}
            ]}
        ]),
        "# JS/TS Seam\n\n## Exercise 1\n\n## Exercise 2\n\n## Exercise 3\n\n## Exercise 4\n",
        Some("ts"),
    );
    let root = temp.path().join("work");
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("package.json"),
        r#"{"name":"seam","type":"module"}"#,
    )
    .unwrap();
    fs::write(
        root.join("tests/pass.test.ts"),
        concat!(
            "import { describe, test } from \"node:test\";\n",
            "import assert from \"node:assert/strict\";\n",
            "describe(\"math\", () => {\n",
            "  test(\"adds two numbers\", () => {\n",
            "    const sum: number = 2 + 3;\n",
            "    assert.strictEqual(sum, 5);\n",
            "  });\n",
            "});\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("tests/fail.test.ts"),
        concat!(
            "import { describe, test } from \"node:test\";\n",
            "import assert from \"node:assert/strict\";\n",
            "describe(\"diff\", () => {\n",
            "  test(\"reports deep diff\", () => {\n",
            "    assert.deepStrictEqual({ count: 1 }, { count: 2 });\n",
            "  });\n",
            "});\n",
        ),
    )
    .unwrap();
    fs::write(
        root.join("tests/syntax.test.ts"),
        "export const broken = (\n",
    )
    .unwrap();

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let pass = wait(&app, id);
    let r1 = &pass["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(r1["status"], "passed", "{r1}");
    assert_eq!(r1["testCases"][0]["status"], "passed", "{r1}");
    assert_eq!(pass["completedExercises"], 1);

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-2"}),
    );
    let fail = wait(&app, id);
    let r2 = &fail["chapters"]["ex-2"]["lastCheckResults"][0];
    assert_eq!(r2["status"], "failed", "{r2}");
    assert_eq!(r2["testCases"][0]["status"], "failed", "{r2}");
    let diff = r2["diffs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["label"].as_str().unwrap().contains("reports deep diff"))
        .unwrap_or_else(|| panic!("expected test diff in {r2}"));
    assert!(
        diff["expected"].as_str().unwrap().contains("count: 2")
            || diff["expected"].as_str().unwrap().contains("\"count\": 2"),
        "unexpected expected diff: {diff}"
    );
    assert!(
        diff["actual"].as_str().unwrap().contains("count: 1")
            || diff["actual"].as_str().unwrap().contains("\"count\": 1"),
        "unexpected actual diff: {diff}"
    );

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-3"}),
    );
    let syntax = wait(&app, id);
    let r3 = &syntax["chapters"]["ex-3"]["lastCheckResults"][0];
    assert_eq!(r3["status"], "failed", "{r3}");
    assert!(
        r3["diffs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["label"] == "jsTsTestReport"),
        "expected syntax error report diff in {r3}"
    );

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-4"}),
    );
    let node_fail = wait(&app, id);
    let r4 = &node_fail["chapters"]["ex-4"]["lastCheckResults"][0];
    assert_eq!(r4["status"], "failed", "{r4}");
    let node_diff = r4["diffs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["label"].as_str().unwrap().contains("reports deep diff"))
        .unwrap_or_else(|| panic!("expected node test diff in {r4}"));
    assert!(
        node_diff["expected"].as_str().unwrap().contains("count: 2"),
        "unexpected node expected diff: {node_diff}"
    );
    assert!(
        node_diff["actual"].as_str().unwrap().contains("count: 1"),
        "unexpected node actual diff: {node_diff}"
    );
    // Report temp files must never pollute the learner Workspace.
    assert!(!root.join("junit.xml").exists());
}

#[test]
fn js_ts_adapter_parses_bun_diffs_rejects_malformed_xml_and_enforces_5s_default_timeout() {
    let (temp, app, id) = fixture_with_language(
        json!([
            {"chapterId":"ex-1","checks":[
                {"id":"bun-diff","title":"Bun scalar and object diff","type":"test_suite","command":"cat bun-junit.xml"},
                {"id":"node-diff","title":"Node 22 deepStrictEqual diff","type":"test_suite","command":"cat node-junit.xml"},
                {"id":"bad-xml","title":"Truncated XML","type":"test_suite","command":"cat truncated.xml"},
                {"id":"skip-only","title":"Skipped only","type":"test_suite","command":"cat skipped.xml"}
            ]},
            {"chapterId":"ex-2","checks":[
                {"id":"default-timeout","title":"Default 5s timeout","type":"test_suite","command":"sleep 20"}
            ]}
        ]),
        "# JS/TS Parser\n\n## Exercise 1\n\n## Exercise 2\n",
        Some("ts"),
    );
    let root = temp.path().join("work");
    fs::write(
        root.join("bun-junit.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="bun test" tests="3" failures="2" skipped="0" time="0.02">
  <testsuite name="pipeline" tests="3" failures="2" skipped="0" time="0.02">
    <testcase name="passes" classname="pipeline" time="0.001" />
    <testcase name="fails toBe" classname="pipeline" time="0.002">
      <failure type="AssertionError" message="expect(received).toBe(expected)&#10;&#10;Expected: &quot;world&quot;&#10;Received: &quot;hello&quot;">AssertionError: expect(received).toBe(expected)

Expected: "world"
Received: "hello"
</failure>
    </testcase>
    <testcase name="fails toEqual" classname="pipeline" time="0.004">
      <failure type="AssertionError">expect(received).toEqual(expected)

  {
-   "count": 2,
+   "count": 1,
  }

- Expected  - 1
+ Received  + 1
</failure>
    </testcase>
  </testsuite>
</testsuites>"#,
    )
    .unwrap();
    fs::write(
        root.join("node-junit.xml"),
        r#"<?xml version="1.0" encoding="utf-8"?>
<testsuites>
  <testsuite name="diff" tests="1" failures="1" errors="0" skipped="0" time="0.01">
    <testcase name="reports deep diff" classname="test" time="0.002">
      <failure type="testCodeFailure" message="Expected values to be strictly deep-equal:&#10;+ actual - expected">
Error [ERR_TEST_FAILURE]: Expected values to be strictly deep-equal:
+ actual - expected

  {
+   count: 1
-   count: 2
  }
      </failure>
    </testcase>
  </testsuite>
</testsuites>"#,
    )
    .unwrap();
    fs::write(
        root.join("truncated.xml"),
        "<testsuites><testsuite name=\"broken\"><testcase name=\"x\" time=\"0.01\">",
    )
    .unwrap();
    fs::write(
        root.join("skipped.xml"),
        "<testsuites><testsuite name=\"s\" tests=\"1\" skipped=\"1\"><testcase name=\"only\" classname=\"s\" time=\"0\"><skipped /></testcase></testsuite></testsuites>",
    )
    .unwrap();

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait(&app, id);
    let results = &progress["chapters"]["ex-1"]["lastCheckResults"];
    assert_eq!(results[0]["status"], "failed", "{}", results[0]);
    let diffs = results[0]["diffs"].as_array().unwrap();
    let to_be = diffs
        .iter()
        .find(|d| d["label"] == "pipeline > fails toBe")
        .unwrap_or_else(|| panic!("missing toBe diff: {diffs:?}"));
    assert_eq!(to_be["expected"], "\"world\"");
    assert_eq!(to_be["actual"], "\"hello\"");
    let to_eq = diffs
        .iter()
        .find(|d| d["label"] == "pipeline > fails toEqual")
        .unwrap_or_else(|| panic!("missing toEqual diff: {diffs:?}"));
    assert!(to_eq["expected"].as_str().unwrap().contains("\"count\": 2"));
    assert!(to_eq["actual"].as_str().unwrap().contains("\"count\": 1"));

    assert_eq!(results[1]["status"], "failed", "{}", results[1]);
    let node_diff = results[1]["diffs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["label"] == "test > reports deep diff")
        .unwrap_or_else(|| panic!("missing Node 22 diff: {}", results[1]));
    assert!(
        node_diff["expected"].as_str().unwrap().contains("count: 2"),
        "unexpected Node 22 expected diff: {node_diff}"
    );
    assert!(
        node_diff["actual"].as_str().unwrap().contains("count: 1"),
        "unexpected Node 22 actual diff: {node_diff}"
    );

    assert_eq!(results[2]["status"], "failed");
    assert_eq!(results[2]["diffs"][0]["label"], "jsTsTestReport");
    assert_eq!(results[3]["status"], "failed");
    assert_eq!(results[3]["diffs"][0]["label"], "jsTsTestReport");

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-2"}),
    );
    let timed = wait(&app, id);
    let t = &timed["chapters"]["ex-2"]["lastCheckResults"][0];
    assert_eq!(t["status"], "timedOut", "{t}");
    let duration_ms = t["durationMs"].as_u64().unwrap();
    assert!(
        (4_500..=15_000).contains(&duration_ms),
        "expected ~5s default JS/TS timeout, got {duration_ms}ms"
    );
}

#[test]
fn python_adapter_parses_passes_assertion_diffs_collector_syntax_errors_and_8s_timeout() {
    let (temp, app, id) = fixture_with_language(
        json!([
            {"chapterId":"ex-1","checks":[
                {"id":"py-pass","title":"Pytest pass","type":"test_suite","command":"cat pass.json","requireTests":["test_clean_ok","tests/test_clean.py::test_clean_ok"]}
            ]},
            {"chapterId":"ex-2","checks":[
                {"id":"py-diff","title":"Pytest diff","type":"test_suite","command":"cat fail.json"},
                {"id":"py-syntax","title":"Pytest syntax error","type":"test_suite","command":"cat syntax.json"},
                {"id":"py-bad","title":"Pytest truncated","type":"test_suite","command":"printf '{broken'"}
            ]},
            {"chapterId":"ex-3","checks":[
                {"id":"py-timeout","title":"Default 8s timeout","type":"test_suite","command":"sleep 20"}
            ]}
        ]),
        "# Python Seam\n\n## Exercise 1\n\n## Exercise 2\n\n## Exercise 3\n",
        Some("python"),
    );
    let root = temp.path().join("work");
    fs::write(
        root.join("pass.json"),
        json!({
            "exitcode": 0,
            "collectors": [{"nodeid": "tests/test_clean.py", "outcome": "passed"}],
            "tests": [{
                "nodeid": "tests/test_clean.py::test_clean_ok",
                "outcome": "passed",
                "setup": {"duration": 0.001, "outcome": "passed"},
                "call": {"duration": 0.014, "outcome": "passed"},
                "teardown": {"duration": 0.001, "outcome": "passed"}
            }]
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        root.join("fail.json"),
        json!({
            "exitcode": 1,
            "collectors": [],
            "tests": [
                {
                    "nodeid": "tests/test_clean.py::test_scalar_assert",
                    "outcome": "failed",
                    "call": {
                        "duration": 0.005,
                        "outcome": "failed",
                        "crash": {"message": "AssertionError: assert {'books': 79.0} == {'books': 99.0}"},
                        "longrepr": "E       AssertionError: assert {'books': 79.0} == {'books': 99.0}"
                    }
                },
                {
                    "nodeid": "tests/test_clean.py::test_full_diff",
                    "outcome": "failed",
                    "call": {
                        "duration": 0.003,
                        "outcome": "failed",
                        "longrepr": "E         Full diff:\nE           [\nE         -   'books',\nE         +   'draft',\nE           ]"
                    }
                },
                {
                    "nodeid": "tests/test_clean.py::test_source_context_not_misread",
                    "outcome": "failed",
                    "call": {
                        "duration": 0.002,
                        "outcome": "failed",
                        "crash": {"message": "AssertionError: assert [] == ['ord-1']"},
                        "longrepr": "    def test_source_context_not_misread():\n        assert invalid == []\n>       assert valid == ['ord-1']\nE       AssertionError: assert [] == ['ord-1']"
                    }
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        root.join("syntax.json"),
        json!({
            "exitcode": 2,
            "collectors": [{
                "nodeid": "tests/test_broken.py",
                "outcome": "failed",
                "longrepr": "  File \"src/cleaner.py\", line 4\n    def bad(:\n            ^\nSyntaxError: invalid syntax"
            }],
            "tests": []
        })
        .to_string(),
    )
    .unwrap();

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let pass = wait(&app, id);
    let r1 = &pass["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(r1["status"], "passed", "{r1}");
    assert_eq!(
        r1["testCases"][0]["name"],
        "tests/test_clean.py::test_clean_ok"
    );
    assert_eq!(r1["testCases"][0]["durationMs"], 16);
    assert_eq!(pass["completedExercises"], 1);

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-2"}),
    );
    let fail = wait(&app, id);
    let results = &fail["chapters"]["ex-2"]["lastCheckResults"];
    assert_eq!(results[0]["status"], "failed", "{}", results[0]);
    let diffs = results[0]["diffs"].as_array().unwrap();
    let scalar = diffs
        .iter()
        .find(|d| d["label"] == "tests/test_clean.py::test_scalar_assert")
        .unwrap();
    assert_eq!(scalar["expected"], "{'books': 99.0}");
    assert_eq!(scalar["actual"], "{'books': 79.0}");
    let full = diffs
        .iter()
        .find(|d| d["label"] == "tests/test_clean.py::test_full_diff")
        .unwrap();
    assert!(full["expected"].as_str().unwrap().contains("'books'"));
    assert!(full["actual"].as_str().unwrap().contains("'draft'"));
    let ctx_diff = diffs
        .iter()
        .find(|d| d["label"] == "tests/test_clean.py::test_source_context_not_misread")
        .unwrap();
    assert_eq!(ctx_diff["expected"], "['ord-1']");
    assert_eq!(ctx_diff["actual"], "[]");

    assert_eq!(results[1]["status"], "failed", "{}", results[1]);
    assert_eq!(results[1]["diffs"][0]["label"], "pytestReport");
    assert!(
        results[1]["diffs"][0]["actual"]
            .as_str()
            .unwrap()
            .contains("SyntaxError")
    );
    assert_eq!(results[2]["status"], "failed", "{}", results[2]);
    assert_eq!(results[2]["diffs"][0]["label"], "pytestReport");

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-3"}),
    );
    let timed = wait(&app, id);
    let t = &timed["chapters"]["ex-3"]["lastCheckResults"][0];
    assert_eq!(t["status"], "timedOut", "{t}");
    let duration_ms = t["durationMs"].as_u64().unwrap();
    assert!(
        (7_500..=18_000).contains(&duration_ms),
        "expected ~8s default Python timeout, got {duration_ms}ms"
    );
}

#[test]
fn java_adapter_parses_passes_junit5_diffs_invalid_xml_and_enforces_12s_timeout() {
    let (temp, app, id) = fixture_with_language(
        json!([
            {"chapterId":"ex-1","checks":[
                {"id":"java-pass","title":"JUnit 5 pass","type":"test_suite","command":"cat pass.xml","requireTests":["allowsBurst","com.codelab.ratelimiter.Ex1TokenBucketTest#allowsBurst"]}
            ]},
            {"chapterId":"ex-2","checks":[
                {"id":"java-diff","title":"JUnit 5 diff","type":"test_suite","command":"cat fail.xml"},
                {"id":"java-err","title":"JUnit 5 suite error","type":"test_suite","command":"cat suite-error.xml"},
                {"id":"java-bad","title":"JUnit 5 malformed","type":"test_suite","command":"printf '<testsuite'"}
            ]},
            {"chapterId":"ex-3","checks":[
                {"id":"java-timeout","title":"Default 12s timeout","type":"test_suite","command":"sleep 25"}
            ]}
        ]),
        "# Java Seam\n\n## Exercise 1\n\n## Exercise 2\n\n## Exercise 3\n",
        Some("java"),
    );
    let root = temp.path().join("work");
    fs::write(
        root.join("pass.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuite name="JUnit Jupiter" tests="1" skipped="0" failures="0" errors="0" time="0.032">
  <testcase name="allowsBurst()" classname="com.codelab.ratelimiter.Ex1TokenBucketTest" time="0.015" />
</testsuite>"#,
    )
    .unwrap();
    fs::write(
        root.join("fail.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuite name="JUnit Jupiter" tests="2" skipped="0" failures="2" errors="0" time="0.041">
  <testcase name="preventsOverAllocation()" classname="com.codelab.ratelimiter.Ex1TokenBucketTest" time="0.025">
    <failure message="expected: &lt;10&gt; but was: &lt;13&gt;" type="org.opentest4j.AssertionFailedError"><![CDATA[org.opentest4j.AssertionFailedError: expected: <10> but was: <13>
	at com.codelab.ratelimiter.Ex1TokenBucketTest.preventsOverAllocation(Ex1TokenBucketTest.java:42)
]]></failure>
  </testcase>
  <testcase name="reportsRetryAfter()" classname="com.codelab.ratelimiter.Ex2RefillingTokenBucketTest" time="0.011">
    <failure message="retry delay ==&gt; expected: &lt;250&gt; but was: &lt;0&gt;" type="org.opentest4j.AssertionFailedError"><![CDATA[org.opentest4j.AssertionFailedError: retry delay ==> expected: <250> but was: <0>
	at com.codelab.ratelimiter.Ex2RefillingTokenBucketTest.reportsRetryAfter(Ex2RefillingTokenBucketTest.java:39)
]]></failure>
  </testcase>
</testsuite>"#,
    )
    .unwrap();
    fs::write(
        root.join("suite-error.xml"),
        r#"<testsuite name="JUnit Jupiter" tests="1" failures="0" errors="1" time="0.01"></testsuite>"#,
    )
    .unwrap();

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let pass = wait(&app, id);
    let r1 = &pass["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(r1["status"], "passed", "{r1}");
    assert_eq!(
        r1["testCases"][0]["name"],
        "com.codelab.ratelimiter.Ex1TokenBucketTest#allowsBurst"
    );
    assert_eq!(r1["testCases"][0]["durationMs"], 15);
    assert_eq!(pass["completedExercises"], 1);

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-2"}),
    );
    let fail = wait(&app, id);
    let results = &fail["chapters"]["ex-2"]["lastCheckResults"];
    assert_eq!(results[0]["status"], "failed", "{}", results[0]);
    let diffs = results[0]["diffs"].as_array().unwrap();
    let d1 = diffs
        .iter()
        .find(|d| d["label"] == "com.codelab.ratelimiter.Ex1TokenBucketTest#preventsOverAllocation")
        .unwrap();
    assert_eq!(d1["expected"], "10");
    assert_eq!(d1["actual"], "13");
    let d2 = diffs
        .iter()
        .find(|d| {
            d["label"] == "com.codelab.ratelimiter.Ex2RefillingTokenBucketTest#reportsRetryAfter"
        })
        .unwrap();
    assert_eq!(d2["expected"], "250");
    assert_eq!(d2["actual"], "0");

    assert_eq!(results[1]["status"], "failed", "{}", results[1]);
    assert_eq!(results[1]["diffs"][0]["label"], "junitReport");
    assert_eq!(results[2]["status"], "failed", "{}", results[2]);
    assert_eq!(results[2]["diffs"][0]["label"], "junitReport");

    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-3"}),
    );
    let timed = wait_for_checks(&app, id, Duration::from_secs(30));
    let t = &timed["chapters"]["ex-3"]["lastCheckResults"][0];
    assert_eq!(t["status"], "timedOut", "{t}");
    let duration_ms = t["durationMs"].as_u64().unwrap();
    assert!(
        (11_500..=22_000).contains(&duration_ms),
        "expected ~12s default Java timeout, got {duration_ms}ms"
    );
}

#[cfg(unix)]
fn refresh_preflight_blocking(app: &App) {
    let events = app.subscribe();
    send(app, "refreshPreflight", Value::Null);
    loop {
        if let codelab_core::AppEvent::PreflightUpdated(report) =
            events.recv_timeout(Duration::from_secs(10)).unwrap()
            && report.checked
            && !report.probing
        {
            break;
        }
    }
}

#[cfg(unix)]
#[test]
fn managed_runtimes_drive_bun_pytest_and_two_phase_javac_junit_including_compile_errors() {
    use std::os::unix::fs::PermissionsExt;

    let catalog: Value =
        serde_json::from_str(include_str!("../src/runtime/artifacts.json")).unwrap();
    let bun_ver = catalog["bunVersion"].as_str().unwrap();
    let py_ver = catalog["pythonVersion"].as_str().unwrap();
    let java_ver = catalog["javaVersion"].as_str().unwrap();
    let junit_name = catalog["junit"]["url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap();

    // 1. Managed Java two-phase javac -> junit (passing + compile error short-circuits Phase 2)
    let (temp, app, id) = fixture_with_language(
        json!([
            {"chapterId":"ex-1","checks":[
                {"id":"java-managed","title":"Managed Java","type":"test_suite","command":"junit --select-class com.example.DemoTest","requireTests":["works"]}
            ]}
        ]),
        "# Managed Java\n\n## Exercise 1\n",
        Some("java"),
    );
    let java_home = if cfg!(target_os = "macos") {
        temp.path().join(format!(
            "data/runtimes/java-{java_ver}/toolchain/Contents/Home/bin"
        ))
    } else {
        temp.path()
            .join(format!("data/runtimes/java-{java_ver}/toolchain/bin"))
    };
    fs::create_dir_all(&java_home).unwrap();
    let junit_jar = temp
        .path()
        .join(format!("data/runtimes/java-{java_ver}/{junit_name}"));
    fs::write(&junit_jar, "fake junit jar").unwrap();

    let javac = java_home.join("javac");
    fs::write(
        &javac,
        concat!(
            "#!/bin/sh\n",
            "if grep -q 'BROKEN_SYNTAX' src/Demo.java 2>/dev/null; then\n",
            "  printf '%s\\n' \"src/Demo.java:1: error: ';' expected\" >&2\n",
            "  exit 1\n",
            "fi\n",
            "exit 0\n",
        ),
    )
    .unwrap();
    fs::set_permissions(&javac, fs::Permissions::from_mode(0o755)).unwrap();

    let java = java_home.join("java");
    fs::write(
        &java,
        concat!(
            "#!/bin/sh\n",
            "printf 'invoked' > java-ran.marker\n",
            "REPORTS=''\n",
            "while [ $# -gt 0 ]; do\n",
            "  if [ \"$1\" = '--reports-dir' ]; then shift; REPORTS=\"$1\"; fi\n",
            "  shift\n",
            "done\n",
            "cat <<'XML' > \"$REPORTS/TEST-junit-jupiter.xml\"\n",
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<testsuite name=\"JUnit Jupiter\" tests=\"1\" failures=\"0\" errors=\"0\" skipped=\"0\" time=\"0.01\">\n",
            "  <testcase name=\"works()\" classname=\"com.example.DemoTest\" time=\"0.008\" />\n",
            "</testsuite>\n",
            "XML\n",
        ),
    )
    .unwrap();
    fs::set_permissions(&java, fs::Permissions::from_mode(0o755)).unwrap();

    let root = temp.path().join("work");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/Demo.java"), "class Demo { BROKEN_SYNTAX }\n").unwrap();

    send(
        &app,
        "setRuntimePreference",
        json!({"language":"Java","preferManaged":true}),
    );
    refresh_preflight_blocking(&app);

    // Compile error fails Phase 1 and must NOT invoke Phase 2 java
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let compiled_err = wait(&app, id);
    let r_err = &compiled_err["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(r_err["status"], "failed", "{r_err}");
    assert!(
        r_err["rawOutput"]
            .as_str()
            .unwrap()
            .contains("error: ';' expected")
    );
    assert!(
        !root.join("java-ran.marker").exists(),
        "Phase 2 java must not run when javac fails"
    );

    // Fix syntax -> Phase 1 & Phase 2 succeed
    fs::write(root.join("src/Demo.java"), "class Demo {}\n").unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let passed = wait(&app, id);
    let r_ok = &passed["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(r_ok["status"], "passed", "{r_ok}");
    assert_eq!(r_ok["testCases"][0]["name"], "com.example.DemoTest#works");
    assert_eq!(passed["completedExercises"], 1);

    // Removing all .java sources reports code-only DomainError JAVA_SOURCES_NOT_FOUND.
    fs::remove_file(root.join("src/Demo.java")).unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let no_src = wait(&app, id);
    let r_no_src = &no_src["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(r_no_src["status"], "failed", "{r_no_src}");
    assert_eq!(
        r_no_src["error"]["code"], "JAVA_SOURCES_NOT_FOUND",
        "{r_no_src}"
    );

    // Symlinked workspace/lib/junit*.jar must be rejected (canonical root containment).
    let outside = TempDir::new().unwrap();
    let outside_jar = outside
        .path()
        .join("junit-platform-console-standalone-1.13.4.jar");
    fs::write(&outside_jar, "outside jar").unwrap();
    fs::create_dir_all(root.join("lib")).unwrap();
    std::os::unix::fs::symlink(
        &outside_jar,
        root.join("lib/junit-platform-console-standalone-1.13.4.jar"),
    )
    .unwrap();
    fs::remove_file(&junit_jar).unwrap();
    fs::write(root.join("src/Demo.java"), "class Demo {}\n").unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let sym_err = wait(&app, id);
    let r_sym = &sym_err["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(r_sym["status"], "failed", "{r_sym}");
    assert_eq!(
        r_sym["error"]["code"], "JAVA_JUNIT_LAUNCHER_UNAVAILABLE",
        "{r_sym}"
    );

    // 2. Managed Bun & Managed Python invoke selected managed binaries with temp report files
    let (temp_py, app_py, id_py) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[{"id":"py","title":"Managed Py","type":"test_suite","requireTests":["test_managed_py"]}]}]),
        "# Managed Py\n\n## Exercise 1\n",
        Some("python"),
    );
    let py_os = if cfg!(target_os = "macos") {
        "darwin"
    } else {
        std::env::consts::OS
    };
    let py_libc = if cfg!(target_os = "linux") {
        "gnu"
    } else {
        "none"
    };
    let py_base = temp_py.path().join(format!(
        "data/runtimes/python-{py_ver}/python/cpython-{py_ver}-{py_os}-{}-{py_libc}",
        std::env::consts::ARCH
    ));
    let py_exe = py_base.join("bin/python3.12");
    let uv_exe = temp_py
        .path()
        .join(format!("data/runtimes/python-{py_ver}/uv/uv"));
    let site = py_base.join("lib/python3.12/site-packages");
    fs::create_dir_all(py_exe.parent().unwrap()).unwrap();
    fs::create_dir_all(uv_exe.parent().unwrap()).unwrap();
    fs::create_dir_all(site.join("pytest")).unwrap();
    fs::create_dir_all(site.join("pytest_jsonreport")).unwrap();
    fs::write(&uv_exe, "#!/bin/sh\nexit 0\n").unwrap();
    fs::write(site.join("pytest/__main__.py"), "").unwrap();
    fs::write(site.join("pytest_jsonreport/plugin.py"), "").unwrap();
    fs::write(
        &py_exe,
        concat!(
            "#!/bin/sh\n",
            "OUT=''\n",
            "for arg in \"$@\"; do\n",
            "  case \"$arg\" in --json-report-file=*) OUT=\"${arg#--json-report-file=}\" ;; esac\n",
            "done\n",
            "cat <<'JSON' > \"$OUT\"\n",
            "{\"exitcode\":0,\"collectors\":[],\"tests\":[{\"nodeid\":\"tests/test_m.py::test_managed_py\",\"outcome\":\"passed\",\"call\":{\"duration\":0.004,\"outcome\":\"passed\"}}]}\n",
            "JSON\n",
        ),
    )
    .unwrap();
    fs::set_permissions(&py_exe, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&uv_exe, fs::Permissions::from_mode(0o755)).unwrap();

    send(
        &app_py,
        "setRuntimePreference",
        json!({"language":"Python","preferManaged":true}),
    );
    refresh_preflight_blocking(&app_py);
    send(
        &app_py,
        "runChecks",
        json!({"workspaceId":id_py,"chapterId":"ex-1"}),
    );
    let py_done = wait(&app_py, id_py);
    let r_py = &py_done["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(r_py["status"], "passed", "{r_py}");
    assert_eq!(
        r_py["testCases"][0]["name"],
        "tests/test_m.py::test_managed_py"
    );
    assert!(!temp_py.path().join("work/.report.json").exists());

    // 3. Managed Bun fixture
    let (temp_js, app_js, id_js) = fixture_with_language(
        json!([{"chapterId":"ex-1","checks":[{"id":"js","title":"Managed Bun","type":"test_suite","requireTests":["managed_bun_case"]}]}]),
        "# Managed Bun\n\n## Exercise 1\n",
        Some("ts"),
    );
    let bun_exe = temp_js
        .path()
        .join(format!("data/runtimes/bun-{bun_ver}/toolchain/bun"));
    fs::create_dir_all(bun_exe.parent().unwrap()).unwrap();
    fs::write(
        &bun_exe,
        concat!(
            "#!/bin/sh\n",
            "OUT=''\n",
            "for arg in \"$@\"; do\n",
            "  case \"$arg\" in --reporter-outfile=*) OUT=\"${arg#--reporter-outfile=}\" ;; esac\n",
            "done\n",
            "cat <<'XML' > \"$OUT\"\n",
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<testsuites name=\"bun test\"><testsuite name=\"suite\"><testcase name=\"managed_bun_case\" classname=\"suite\" time=\"0.002\" /></testsuite></testsuites>\n",
            "XML\n",
        ),
    )
    .unwrap();
    fs::set_permissions(&bun_exe, fs::Permissions::from_mode(0o755)).unwrap();

    send(
        &app_js,
        "setRuntimePreference",
        json!({"language":"JS/TS","preferManaged":true}),
    );
    refresh_preflight_blocking(&app_js);
    send(
        &app_js,
        "runChecks",
        json!({"workspaceId":id_js,"chapterId":"ex-1"}),
    );
    let js_done = wait(&app_js, id_js);
    let r_js = &js_done["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(r_js["status"], "passed", "{r_js}");
    assert_eq!(r_js["testCases"][0]["name"], "suite > managed_bun_case");
}

fn apply_chapter_reference_solution(app: &App, workspace_id: u64, chapter_id: &str) {
    let preview = send(
        app,
        "previewChapterSolution",
        json!({"workspaceId": workspace_id, "chapterId": chapter_id}),
    );
    let comparisons = preview["payload"]
        .as_array()
        .unwrap_or_else(|| panic!("expected comparisons for {chapter_id}: {preview}"));
    assert!(
        !comparisons.is_empty(),
        "chapter {chapter_id} must expose Reference Solution files"
    );
    for file in comparisons {
        let applied = send(
            app,
            "applyWorkspaceComparison",
            json!({
                "workspaceId": workspace_id,
                "path": file["current"]["path"],
                "text": file["proposed"],
                "expectedRevision": file["current"]["revision"],
            }),
        );
        assert_eq!(
            applied["type"], "workspaceFile",
            "failed applying solution file for {chapter_id}: {applied}"
        );
    }
}

#[test]
fn builtin_go_and_ts_courses_fail_initially_then_pass_after_applying_reference_solutions() {
    // 1. Go built-in Course Pack: ex-3 fails on untouched Starter -> apply Reference Solution -> passes!
    {
        let temp = TempDir::new().unwrap();
        let app = App::open(AppConfig {
            data_dir: temp.path().join("data"),
        })
        .unwrap();
        send(
            &app,
            "openCourse",
            json!({"kind":"builtin","courseId":"go-goframe-practice"}),
        );
        let root = temp.path().join("work");
        fs::create_dir(&root).unwrap();
        let id = send(&app, "bindWorkspace", json!({"path":root}))["payload"]["id"]
            .as_u64()
            .unwrap();

        let initial = run_go_checks(&app, id, "ex-3");
        assert_eq!(
            initial["chapters"]["ex-3"]["lastCheckResults"][0]["status"], "failed",
            "{initial}"
        );
        assert_eq!(initial["completedExercises"], 0);

        apply_chapter_reference_solution(&app, id, "ex-3");
        let after = run_go_checks(&app, id, "ex-3");
        assert_eq!(
            after["chapters"]["ex-3"]["lastCheckResults"][0]["status"], "passed",
            "{after}"
        );
        assert_eq!(after["completedExercises"], 1);
    }

    // 2. JS/TS built-in Course Pack: ALL 8 exercises fail initially -> apply Reference Solution -> ALL 8 pass!
    {
        let temp = TempDir::new().unwrap();
        let app = App::open(AppConfig {
            data_dir: temp.path().join("data"),
        })
        .unwrap();
        send(
            &app,
            "openCourse",
            json!({"kind":"builtin","courseId":"ts-data-pipeline"}),
        );
        let root = temp.path().join("work");
        fs::create_dir(&root).unwrap();
        let id = send(&app, "bindWorkspace", json!({"path":root}))["payload"]["id"]
            .as_u64()
            .unwrap();

        // Transparent test files exist in Workspace immediately after Starter copy
        for name in [
            "tests/ex1-filter.test.ts",
            "tests/ex2-normalize.test.ts",
            "tests/ex3-dedup.test.ts",
            "tests/ex4-aggregate.test.ts",
            "tests/ex5-paginate.test.ts",
            "tests/ex6-settled.test.ts",
            "tests/ex7-concurrency.test.ts",
            "tests/ex8-pipeline.test.ts",
        ] {
            assert!(root.join(name).is_file(), "missing transparent test {name}");
        }

        for index in 1..=8 {
            let chapter_id = format!("ex-{index}");
            send(
                &app,
                "runChecks",
                json!({"workspaceId":id,"chapterId":chapter_id}),
            );
            let before = wait(&app, id);
            let before_result = &before["chapters"][&chapter_id]["lastCheckResults"][0];
            assert_eq!(
                before_result["status"], "failed",
                "ts-data-pipeline {chapter_id} should fail on Starter: {before_result}"
            );

            apply_chapter_reference_solution(&app, id, &chapter_id);
            send(
                &app,
                "runChecks",
                json!({"workspaceId":id,"chapterId":chapter_id}),
            );
            let after = wait(&app, id);
            let after_result = &after["chapters"][&chapter_id]["lastCheckResults"][0];
            assert_eq!(
                after_result["status"], "passed",
                "ts-data-pipeline {chapter_id} should pass after Reference Solution: {after_result}"
            );
            assert_eq!(after["completedExercises"], index as u64);
        }
    }
}

#[cfg(unix)]
#[test]
fn builtin_python_course_runs_real_python_tests_fail_then_pass_all_six_exercises() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().unwrap();
    let catalog: Value =
        serde_json::from_str(include_str!("../src/runtime/artifacts.json")).unwrap();
    let py_ver = catalog["pythonVersion"].as_str().unwrap();
    let py_os = if cfg!(target_os = "macos") {
        "darwin"
    } else {
        std::env::consts::OS
    };
    let py_libc = if cfg!(target_os = "linux") {
        "gnu"
    } else {
        "none"
    };
    let py_base = temp.path().join(format!(
        "data/runtimes/python-{py_ver}/python/cpython-{py_ver}-{py_os}-{}-{py_libc}",
        std::env::consts::ARCH
    ));
    let py_exe = py_base.join("bin/python3.12");
    let uv_exe = temp
        .path()
        .join(format!("data/runtimes/python-{py_ver}/uv/uv"));
    let site = py_base.join("lib/python3.12/site-packages");
    fs::create_dir_all(py_exe.parent().unwrap()).unwrap();
    fs::create_dir_all(uv_exe.parent().unwrap()).unwrap();
    fs::create_dir_all(site.join("pytest")).unwrap();
    fs::create_dir_all(site.join("pytest_jsonreport")).unwrap();
    fs::write(&uv_exe, "#!/bin/sh\nexit 0\n").unwrap();
    fs::write(site.join("pytest/__main__.py"), "").unwrap();
    fs::write(site.join("pytest_jsonreport/plugin.py"), "").unwrap();

    // Execute real Python test files via host python3 and emit pytest-json-report JSON.
    let runner_py = py_base.join("run_pytest_shim.py");
    fs::write(
        &runner_py,
        r#"import importlib.util, json, pathlib, sys, time, traceback

report_file = None
targets = []
args = sys.argv[1:]
i = 0
while i < len(args):
    arg = args[i]
    if arg == "-m" and i + 1 < len(args) and args[i + 1] == "pytest":
        i += 2
        continue
    if arg.startswith("--json-report-file="):
        report_file = arg.split("=", 1)[1]
    elif arg == "--json-report":
        pass
    elif not arg.startswith("-"):
        targets.append(arg)
    i += 1

cwd = pathlib.Path.cwd()
src_dir = cwd / "src"
if str(src_dir) not in sys.path:
    sys.path.insert(0, str(src_dir))

tests_out = []
failed = 0
for target in targets:
    path = pathlib.Path(target)
    spec = importlib.util.spec_from_file_location(path.stem, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    for name in sorted(dir(mod)):
        if not name.startswith("test_"):
            continue
        fn = getattr(mod, name)
        if not callable(fn):
            continue
        nodeid = f"{target}::{name}"
        t0 = time.perf_counter()
        try:
            fn()
            dt = time.perf_counter() - t0
            tests_out.append({
                "nodeid": nodeid,
                "outcome": "passed",
                "call": {"duration": dt, "outcome": "passed"},
            })
        except Exception as exc:
            dt = time.perf_counter() - t0
            failed += 1
            tests_out.append({
                "nodeid": nodeid,
                "outcome": "failed",
                "call": {
                    "duration": dt,
                    "outcome": "failed",
                    "crash": {"message": f"{type(exc).__name__}: {exc}"},
                    "longrepr": traceback.format_exc(),
                },
            })

payload = {"exitcode": 1 if failed else 0, "collectors": [], "tests": tests_out}
if report_file:
    pathlib.Path(report_file).write_text(json.dumps(payload), encoding="utf-8")
else:
    print(json.dumps(payload))
sys.exit(1 if failed else 0)
"#,
    )
    .unwrap();
    fs::write(
        &py_exe,
        format!("#!/bin/sh\nexec python3 '{}' \"$@\"\n", runner_py.display()),
    )
    .unwrap();
    fs::set_permissions(&py_exe, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&uv_exe, fs::Permissions::from_mode(0o755)).unwrap();

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    send(
        &app,
        "setRuntimePreference",
        json!({"language":"Python","preferManaged":true}),
    );
    refresh_preflight_blocking(&app);

    send(
        &app,
        "openCourse",
        json!({"kind":"builtin","courseId":"python-data-clean"}),
    );
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    let id = send(&app, "bindWorkspace", json!({"path":root}))["payload"]["id"]
        .as_u64()
        .unwrap();

    for name in [
        "tests/test_ex1_cleaner.py",
        "tests/test_ex2_csv_loader.py",
        "tests/test_ex3_dedup.py",
        "tests/test_ex4_analytics.py",
        "tests/test_ex5_anomaly.py",
        "tests/test_ex6_report.py",
    ] {
        assert!(root.join(name).is_file(), "missing transparent test {name}");
    }

    for index in 1..=6 {
        let chapter_id = format!("ex-{index}");
        send(
            &app,
            "runChecks",
            json!({"workspaceId":id,"chapterId":chapter_id}),
        );
        let before = wait(&app, id);
        let before_result = &before["chapters"][&chapter_id]["lastCheckResults"][0];
        assert_eq!(
            before_result["status"], "failed",
            "python-data-clean {chapter_id} should fail on Starter: {before_result}"
        );

        apply_chapter_reference_solution(&app, id, &chapter_id);
        send(
            &app,
            "runChecks",
            json!({"workspaceId":id,"chapterId":chapter_id}),
        );
        let after = wait(&app, id);
        let after_result = &after["chapters"][&chapter_id]["lastCheckResults"][0];
        assert_eq!(
            after_result["status"], "passed",
            "python-data-clean {chapter_id} should pass after Reference Solution: {after_result}"
        );
        assert_eq!(after["completedExercises"], index as u64);
    }
}

#[cfg(unix)]
#[test]
fn builtin_java_course_fails_initially_then_passes_all_seven_exercises_after_reference_solutions() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().unwrap();
    let catalog: Value =
        serde_json::from_str(include_str!("../src/runtime/artifacts.json")).unwrap();
    let java_ver = catalog["javaVersion"].as_str().unwrap();
    let junit_name = catalog["junit"]["url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap();

    let java_bin = if cfg!(target_os = "macos") {
        temp.path().join(format!(
            "data/runtimes/java-{java_ver}/toolchain/Contents/Home/bin"
        ))
    } else {
        temp.path()
            .join(format!("data/runtimes/java-{java_ver}/toolchain/bin"))
    };
    fs::create_dir_all(&java_bin).unwrap();
    let junit_jar = temp
        .path()
        .join(format!("data/runtimes/java-{java_ver}/{junit_name}"));
    fs::write(&junit_jar, "junit5").unwrap();

    let javac = java_bin.join("javac");
    fs::write(&javac, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&javac, fs::Permissions::from_mode(0o755)).unwrap();

    // Emits failure XML when the target Java file still contains Starter `TODO:`, and passing XML once Reference Solution is applied.
    let java = java_bin.join("java");
    fs::write(
        &java,
        r#"#!/bin/sh
REPORTS=""
CLS=""
while [ $# -gt 0 ]; do
  case "$1" in
    --reports-dir) shift; REPORTS="$1" ;;
    --select-class) shift; CLS="$1" ;;
  esac
  shift
done
case "$CLS" in
  *Ex1TokenBucketTest)
    SRC="src/main/java/com/codelab/ratelimiter/TokenBucket.java"
    TESTS="allowsRequestsUpToCapacityAndRejectsBeyond rejectsNonPositiveCapacity preventsOverAllocationUnderConcurrency"
    ;;
  *Ex2RefillingTokenBucketTest)
    SRC="src/main/java/com/codelab/ratelimiter/RefillingTokenBucket.java"
    TESTS="refillsTokensAsClockAdvancesWithoutExceedingCapacity reportsRetryAfterMillisWhenEmpty consumesBurstThenRefillsIncrementally"
    ;;
  *Ex3FixedWindowLimiterTest)
    SRC="src/main/java/com/codelab/ratelimiter/FixedWindowLimiter.java"
    TESTS="enforcesLimitWithinSingleWindow resetsQuotaAtNextWindowBoundary returnsExactRetryAfterUntilWindowReset"
    ;;
  *Ex4SlidingWindowLogLimiterTest)
    SRC="src/main/java/com/codelab/ratelimiter/SlidingWindowLogLimiter.java"
    TESTS="preventsBoundaryBurstAcrossFixedWindowEdges evictsExpiredTimestampsAndComputesRetryAfter tracksRemainingCapacityAccurately"
    ;;
  *Ex5TenantRateLimiterTest)
    SRC="src/main/java/com/codelab/ratelimiter/TenantRateLimiter.java"
    TESTS="isolatesQuotasBetweenDistinctTenants rejectsNullOrBlankTenants safelyInitializesSharedTenantUnderConcurrency"
    ;;
  *Ex6VirtualThreadLoadRunnerTest)
    SRC="src/main/java/com/codelab/ratelimiter/VirtualThreadLoadRunner.java"
    TESTS="executesAllRequestsOnVirtualThreadsWithoutOverAllocation countsAllowedAndRejectedConsistently rejectsNegativeRequestCount"
    ;;
  *Ex7CompositeRateLimiterTest)
    SRC="src/main/java/com/codelab/ratelimiter/CompositeRateLimiter.java"
    TESTS="enforcesBurstLimitBeforeSustainedLimit doesNotConsumeBurstQuotaWhenSustainedLimitRejects returnsLargerRetryAfterWhenBothWindowsBlock"
    ;;
  *) exit 2 ;;
esac
if grep -q 'TODO:' "$SRC"; then
  {
    printf '<?xml version="1.0" encoding="UTF-8"?>\n<testsuite name="JUnit Jupiter" tests="3" failures="3" errors="0" skipped="0" time="0.02">\n'
    for m in $TESTS; do
      printf '  <testcase name="%s()" classname="%s" time="0.005"><failure message="expected: &lt;true&gt; but was: &lt;false&gt;">expected: &lt;true&gt; but was: &lt;false&gt;</failure></testcase>\n' "$m" "$CLS"
    done
    printf '</testsuite>\n'
  } > "$REPORTS/TEST-junit-jupiter.xml"
  exit 1
else
  {
    printf '<?xml version="1.0" encoding="UTF-8"?>\n<testsuite name="JUnit Jupiter" tests="3" failures="0" errors="0" skipped="0" time="0.02">\n'
    for m in $TESTS; do
      printf '  <testcase name="%s()" classname="%s" time="0.004" />\n' "$m" "$CLS"
    done
    printf '</testsuite>\n'
  } > "$REPORTS/TEST-junit-jupiter.xml"
  exit 0
fi
"#,
    )
    .unwrap();
    fs::set_permissions(&java, fs::Permissions::from_mode(0o755)).unwrap();

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    send(
        &app,
        "setRuntimePreference",
        json!({"language":"Java","preferManaged":true}),
    );
    refresh_preflight_blocking(&app);

    send(
        &app,
        "openCourse",
        json!({"kind":"builtin","courseId":"java-rate-limiter"}),
    );
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    let id = send(&app, "bindWorkspace", json!({"path":root}))["payload"]["id"]
        .as_u64()
        .unwrap();

    for name in [
        "src/test/java/com/codelab/ratelimiter/Ex1TokenBucketTest.java",
        "src/test/java/com/codelab/ratelimiter/Ex2RefillingTokenBucketTest.java",
        "src/test/java/com/codelab/ratelimiter/Ex3FixedWindowLimiterTest.java",
        "src/test/java/com/codelab/ratelimiter/Ex4SlidingWindowLogLimiterTest.java",
        "src/test/java/com/codelab/ratelimiter/Ex5TenantRateLimiterTest.java",
        "src/test/java/com/codelab/ratelimiter/Ex6VirtualThreadLoadRunnerTest.java",
        "src/test/java/com/codelab/ratelimiter/Ex7CompositeRateLimiterTest.java",
    ] {
        assert!(root.join(name).is_file(), "missing transparent test {name}");
    }

    for index in 1..=7 {
        let chapter_id = format!("ex-{index}");
        send(
            &app,
            "runChecks",
            json!({"workspaceId":id,"chapterId":chapter_id}),
        );
        let before = wait(&app, id);
        let before_result = &before["chapters"][&chapter_id]["lastCheckResults"][0];
        assert_eq!(
            before_result["status"], "failed",
            "java-rate-limiter {chapter_id} should fail on Starter: {before_result}"
        );
        assert!(
            before_result["diffs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["expected"] == "true" && d["actual"] == "false"),
            "expected structured Expected vs Actual diff: {before_result}"
        );

        apply_chapter_reference_solution(&app, id, &chapter_id);
        send(
            &app,
            "runChecks",
            json!({"workspaceId":id,"chapterId":chapter_id}),
        );
        let after = wait(&app, id);
        let after_result = &after["chapters"][&chapter_id]["lastCheckResults"][0];
        assert_eq!(
            after_result["status"], "passed",
            "java-rate-limiter {chapter_id} should pass after Reference Solution: {after_result}"
        );
        assert_eq!(after["completedExercises"], index as u64);
    }
}

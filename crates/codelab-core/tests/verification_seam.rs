use codelab_core::{App, AppCommand, AppConfig};
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
    for language in [None, Some("python")] {
        let (temp, app, id) = fixture_with_language(
            json!([{"chapterId":"ex-1","checks":[
                {"id":"suite","title":"Unsupported","type":"test_suite","command":"printf 'printf ran > unwanted.txt' | sh"},
                {"id":"command","title":"Other Check","type":"command_test","command":"printf checked"}
            ]}]),
            "# Course\n\n## Exercise 1\n",
            language,
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

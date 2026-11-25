use codelab_core::{App, AppCommand, AppConfig};
use serde_json::{Value, json};
use std::{
    fs,
    time::{Duration, Instant},
};
use tempfile::TempDir;

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
    let temp = TempDir::new().unwrap();
    let pack = temp.path().join("pack");
    fs::create_dir(&pack).unwrap();
    fs::write(pack.join("README.md"), manual).unwrap();
    fs::write(pack.join("codelab.json"), r#"{"schemaVersion":"1.0","courseId":"verify","title":"Verification","entryFile":"README.md"}"#).unwrap();
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
    let until = Instant::now() + Duration::from_secs(20);
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
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
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
    let (temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[{"id":"go","title":"Go tests","type":"test_suite","command":"go test -json -race ./...","timeoutSec":60}]}]),
        "# Course\n\n## Exercise 1\n\n### Check\n",
    );
    let root = temp.path().join("work");
    fs::write(root.join("go.mod"), "module seam.test/check\n\ngo 1.22\n").unwrap();
    fs::write(root.join("main_test.go"),"package check\nimport \"testing\"\nfunc TestContract(t *testing.T) { t.Error(\"expected: 201 actual: 200\") }\n").unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait(&app, id);
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "failed");
    assert_eq!(
        result["testCases"][0]["name"],
        "seam.test/check/TestContract"
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
    use std::net::TcpListener;
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
    let external = TcpListener::bind(("127.0.0.1", port)).unwrap();
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
    use std::net::{TcpListener, TcpStream};
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
    assert!(
        TcpListener::bind(("127.0.0.1", port)).is_ok(),
        "grandchild retained its port"
    );
}

#[cfg(windows)]
mod windows_timeout {
    use super::*;
    use std::{
        net::TcpListener,
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
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            match TcpListener::bind(("127.0.0.1", port)) {
                Ok(_) => break,
                Err(error) => {
                    assert!(
                        Instant::now() < until,
                        "descendant retained port {port}: {error}"
                    );
                    thread::sleep(Duration::from_millis(20));
                }
            }
        }
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
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-3"}),
    );
    let progress = wait(&app, id);
    assert_eq!(progress["completedExercises"], 0);
    let result = &progress["chapters"]["ex-3"]["lastCheckResults"][0];
    assert_eq!(result["status"], "failed");
    assert!(
        result["diffs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["label"] == "TestTaskHTTPContract")
    );
}

fn service_command(port: u16, body: &str) -> String {
    let executable = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    format!(
        "CODELAB_SEAM_PORT={port} CODELAB_SEAM_BODY={body} '{executable}' --exact verification_service_fixture --nocapture"
    )
}
#[test]
fn verification_service_fixture() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let Ok(port) = std::env::var("CODELAB_SEAM_PORT") else {
        return;
    };
    let listener = TcpListener::bind(("127.0.0.1", port.parse::<u16>().unwrap())).unwrap();
    let body = std::env::var("CODELAB_SEAM_BODY").unwrap();
    let body = if body == "from_source" {
        fs::read_to_string("server.py").unwrap()
    } else {
        body
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
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
    }
}
#[test]
fn different_managed_service_on_the_same_port_is_restarted_with_the_required_launch() {
    use std::net::TcpListener;
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
    use std::net::{TcpListener, TcpStream};
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
    let until = Instant::now() + Duration::from_secs(3);
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    app.shutdown().unwrap();
    assert!(TcpListener::bind(("127.0.0.1", port)).is_ok());
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
    let (temp, app, id) = fixture(
        json!([{"chapterId":"ex-1","checks":[{"id":"go","title":"required","type":"test_suite","command":"go test -json -race ./...","requireTests":["TestRequired"]}]}]),
        "# Course\n\n## Exercise 1\n",
    );
    fs::write(
        temp.path().join("work/go.mod"),
        "module seam.test/skip\n\ngo 1.22\n",
    )
    .unwrap();
    fs::write(temp.path().join("work/main_test.go"),"package skip\nimport \"testing\"\nfunc TestRequired(t *testing.T) { t.Skip(\"not implemented\") }\n").unwrap();
    send(
        &app,
        "runChecks",
        json!({"workspaceId":id,"chapterId":"ex-1"}),
    );
    let progress = wait(&app, id);
    let result = &progress["chapters"]["ex-1"]["lastCheckResults"][0];
    assert_eq!(result["status"], "failed");
    assert_eq!(result["testCases"][0]["status"], "skipped");
    assert_eq!(progress["completedExercises"], 0);
}

#[test]
fn daemon_label_takes_precedence_over_an_earlier_block_with_the_same_port() {
    use std::net::TcpListener;
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

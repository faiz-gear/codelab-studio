use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use codelab_core::{
    AcpAgentPreset, App, AppCommand, AppConfig, AppEvent, AppResponse, BackendEvent,
    BackendEventSink, BackendPermissionPrompt, ContextDimension, ContextQuote, ContextQuoteSource,
    ContextSnapshotInput, DomainError, ErrorCode, ExerciseTarget, MAX_CONTEXT_INPUT_TOKENS,
    OpenCourseRequest, PermissionPolicy, RespondTutorPermission, RunChecks, RunCommandBlock,
    SendTutorPrompt, StoreKind, TeachingStyle, TerminalRead, TrustSource, TrustSourceKind,
    TutorBackend, TutorBackendSession, TutorHandshakeRequest, TutorHandshakeResult,
    TutorLaunchSpec, TutorMessageStatus, TutorPermissionKind, TutorPermissionStatus, TutorRole,
    TutorSessionSnapshot, TutorSessionStatus, TutorSettingsPatch, TutorTurnInput,
    UpdateSettingsPatch, WorkspaceTutorRequest,
};
use tempfile::TempDir;

const FAKE_ACP_AGENT_JS: &str = r#"
const fs = require("node:fs");
const path = require("node:path");
const readline = require("node:readline");
const { spawn } = require("node:child_process");

function readFlag(prefix, envName) {
  const arg = process.argv.slice(2).find((item) => item.startsWith(prefix));
  if (arg) {
    return arg.slice(prefix.length);
  }
  return process.env[envName] || "";
}

const logPath = readFlag("--log=", "FAKE_ACP_LOG");
const pidPath = readFlag("--pids=", "FAKE_ACP_PIDS");
const markerPath = readFlag("--marker=", "FAKE_ACP_MARKER");
const detachGrandchild = process.argv.slice(2).includes("--detach");

if (markerPath) {
  fs.appendFileSync(markerPath, `spawn:${process.pid}\n`);
}

let grandchild = null;
if (pidPath) {
  if (detachGrandchild) {
    const helper = `
      const fs = require("node:fs");
      const net = require("node:net");
      const target = process.argv[1];
      const parentPid = Number(process.argv[2]);
      const server = net.createServer();
      server.listen(0, "127.0.0.1", () => {
        const tmp = target + ".tmp";
        fs.writeFileSync(tmp, JSON.stringify({ pid: parentPid, childPid: process.pid, childPort: server.address().port }));
        fs.renameSync(tmp, target);
      });
    `;
    grandchild = spawn(process.execPath, ["-e", helper, pidPath, String(process.pid)], {
      stdio: "ignore",
      detached: true,
    });
    grandchild.unref();
  } else {
    grandchild = spawn(process.execPath, ["-e", "setInterval(() => {}, 1000)"], {
      stdio: "ignore",
    });
    fs.writeFileSync(pidPath, JSON.stringify({ pid: process.pid, childPid: grandchild.pid }));
  }
}

function logEvent(obj) {
  if (logPath) {
    fs.appendFileSync(logPath, JSON.stringify(obj) + "\n");
  }
}

function send(msg) {
  process.stdout.write(JSON.stringify(msg) + "\n");
}

let nextAgentId = 100;
const pendingAgent = new Map();
let activeTurn = null;
let sessionSeq = 0;

function requestClient(method, params) {
  return new Promise((resolve) => {
    const id = `agent-${nextAgentId++}`;
    pendingAgent.set(id, resolve);
    send({ jsonrpc: "2.0", id, method, params });
  });
}

function sendChunk(text) {
  send({
    jsonrpc: "2.0",
    method: "session/update",
    params: {
      update: {
        sessionUpdate: "agent_message_chunk",
        content: { type: "text", text },
      },
    },
  });
}

const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });

rl.on("line", async (line) => {
  if (!line.trim()) return;
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    return;
  }

  if (!msg.method && msg.id !== undefined) {
    const resolver = pendingAgent.get(msg.id);
    if (resolver) {
      pendingAgent.delete(msg.id);
      resolver(msg);
    }
    return;
  }

  if (msg.method === "initialize") {
    logEvent({ method: "initialize", pid: process.pid });
    send({
      jsonrpc: "2.0",
      id: msg.id,
      result: {
        protocolVersion: 1,
        agentInfo: { name: "Fake ACP Agent", version: "1.0.0" },
        agentCapabilities: { loadSession: false },
      },
    });
    return;
  }

  if (msg.method === "session/new") {
    sessionSeq += 1;
    const sessionId = `sess-${process.pid}-${sessionSeq}`;
    logEvent({ method: "session/new", sessionId, cwd: msg.params?.cwd });
    send({ jsonrpc: "2.0", id: msg.id, result: { sessionId } });
    return;
  }

  if (msg.method === "session/cancel") {
    logEvent({ method: "session/cancel" });
    if (activeTurn) {
      activeTurn.cancelled = true;
      if (activeTurn.timer) clearTimeout(activeTurn.timer);
      const rpcId = activeTurn.rpcId;
      activeTurn = null;
      send({ jsonrpc: "2.0", id: rpcId, result: { stopReason: "cancelled" } });
    }
    return;
  }

  if (msg.method === "session/prompt") {
    const promptText = msg.params?.prompt?.[0]?.text || "";
    logEvent({ method: "session/prompt", sessionId: msg.params?.sessionId, text: promptText });

    if (promptText.includes("CMD:CRASH")) {
      sendChunk("partial-before-crash");
      setTimeout(() => process.exit(42), 30);
      return;
    }

    if (promptText.includes("CMD:SLOW")) {
      sendChunk("streaming-part-1");
      activeTurn = {
        rpcId: msg.id,
        cancelled: false,
        timer: setTimeout(() => {
          if (activeTurn && !activeTurn.cancelled) {
            sendChunk("-part-2");
            const rpcId = activeTurn.rpcId;
            activeTurn = null;
            send({ jsonrpc: "2.0", id: rpcId, result: { stopReason: "end_turn" } });
          }
        }, 5000),
      };
      return;
    }

    if (promptText.includes("CMD:READ_SAFE")) {
      const resp = await requestClient("fs/read_text_file", { path: "main.go" });
      if (resp.result?.content) {
        sendChunk(`READ_OK:${resp.result.content}`);
      } else {
        sendChunk(`READ_DENIED:${resp.error?.message || "err"}`);
      }
      send({ jsonrpc: "2.0", id: msg.id, result: { stopReason: "end_turn" } });
      return;
    }

    if (promptText.includes("CMD:READ_SENSITIVE:")) {
      const target = promptText.split("CMD:READ_SENSITIVE:")[1].trim().split(/\s+/)[0];
      const resp = await requestClient("fs/read_text_file", { path: target });
      sendChunk(resp.error ? "SENSITIVE_BLOCKED" : `LEAKED:${resp.result?.content}`);
      send({ jsonrpc: "2.0", id: msg.id, result: { stopReason: "end_turn" } });
      return;
    }

    if (promptText.includes("CMD:READ_OUTSIDE:")) {
      const target = promptText.split("CMD:READ_OUTSIDE:")[1].trim().split(/\s+/)[0];
      const resp = await requestClient("fs/read_text_file", { path: target });
      sendChunk(resp.result?.content ? `OUTSIDE_OK:${resp.result.content}` : "OUTSIDE_DENIED");
      send({ jsonrpc: "2.0", id: msg.id, result: { stopReason: "end_turn" } });
      return;
    }

    if (promptText.includes("CMD:WRITE_FS")) {
      const resp = await requestClient("fs/write_text_file", {
        path: "main.go",
        content: "package main\n// updated by ACP fs/write\n",
      });
      sendChunk(resp.error ? "WRITE_DENIED" : "WRITE_OK");
      send({ jsonrpc: "2.0", id: msg.id, result: { stopReason: "end_turn" } });
      return;
    }

    if (promptText.includes("CMD:WRITE_DIRECT")) {
      const resp = await requestClient("session/request_permission", {
        sessionId: msg.params?.sessionId,
        toolCall: {
          toolCallId: "tc-write",
          title: "Edit main.go",
          kind: "edit",
          locations: [{ path: "main.go" }],
        },
        options: [
          { optionId: "allow_once", name: "Allow", kind: "allow_once" },
          { optionId: "reject_once", name: "Deny", kind: "reject_once" },
        ],
      });
      const allowed = resp.result?.outcome?.optionId === "allow_once";
      if (allowed) {
        fs.writeFileSync(path.join(process.cwd(), "main.go"), "package main\n// direct disk write\n");
        sendChunk("DIRECT_WRITE_OK");
      } else {
        sendChunk("DIRECT_WRITE_DENIED");
      }
      send({ jsonrpc: "2.0", id: msg.id, result: { stopReason: "end_turn" } });
      return;
    }

    if (promptText.includes("CMD:MULTI_LOC_SENSITIVE")) {
      const resp = await requestClient("session/request_permission", {
        sessionId: msg.params?.sessionId,
        toolCall: {
          toolCallId: "tc-multi",
          title: "Read multiple files",
          kind: "read",
          locations: [{ path: "main.go" }, { path: ".env" }],
        },
        options: [
          { optionId: "allow_always", name: "Always", kind: "allow_always" },
          { optionId: "allow_once", name: "Allow", kind: "allow_once" },
          { optionId: "reject_once", name: "Deny", kind: "reject_once" },
        ],
      });
      const opt = resp.result?.outcome?.optionId || resp.result?.outcome?.outcome;
      sendChunk(`MULTI_RESULT:${opt}`);
      send({ jsonrpc: "2.0", id: msg.id, result: { stopReason: "end_turn" } });
      return;
    }

    if (promptText.includes("CMD:ALLOW_ALWAYS_FIRST")) {
      const resp = await requestClient("session/request_permission", {
        sessionId: msg.params?.sessionId,
        toolCall: {
          toolCallId: "tc-order",
          title: "Read main.go",
          kind: "read",
          locations: [{ path: "main.go" }],
        },
        options: [
          { optionId: "perm_always", name: "Always", kind: "allow_always" },
          { optionId: "perm_once", name: "Once", kind: "allow_once" },
          { optionId: "perm_reject", name: "Deny", kind: "reject_once" },
        ],
      });
      const opt = resp.result?.outcome?.optionId || resp.result?.outcome?.outcome;
      sendChunk(`OPTION:${opt}`);
      send({ jsonrpc: "2.0", id: msg.id, result: { stopReason: "end_turn" } });
      return;
    }

    if (promptText.includes("CMD:EXEC:")) {
      const cmd = promptText.split("CMD:EXEC:")[1].trim();
      const resp = await requestClient("session/request_permission", {
        sessionId: msg.params?.sessionId,
        toolCall: {
          toolCallId: "tc-exec",
          title: "Run shell command",
          kind: "execute",
          rawInput: { command: cmd },
        },
        options: [
          { optionId: "allow_once", name: "Allow", kind: "allow_once" },
          { optionId: "reject_once", name: "Deny", kind: "reject_once" },
        ],
      });
      const allowed = resp.result?.outcome?.optionId === "allow_once";
      sendChunk(allowed ? "EXEC_ALLOWED" : "EXEC_DENIED");
      send({ jsonrpc: "2.0", id: msg.id, result: { stopReason: "end_turn" } });
      return;
    }

    const styleMatch = promptText.match(/\[Teaching Style: (socratic|direct)\]/);
    const style = styleMatch ? styleMatch[1] : "unknown";
    sendChunk(`style=${style}; `);
    sendChunk(`session=${msg.params?.sessionId}`);
    send({ jsonrpc: "2.0", id: msg.id, result: { stopReason: "end_turn" } });
  }
});
"#;

fn path_str(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

fn write_fake_agent(dir: &Path) -> PathBuf {
    let script = dir.join("fake-acp-agent.cjs");
    fs::write(&script, FAKE_ACP_AGENT_JS).unwrap();
    script
}

fn node_command(script: &Path) -> String {
    format!("node \"{}\"", path_str(script))
}

fn open_trusted_workspace(temp: &TempDir, app: &App, name: &str) -> (u64, PathBuf) {
    let AppResponse::LoadedCourse(_) = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::Builtin {
            course_id: "go-goframe-practice".into(),
        }))
        .unwrap()
    else {
        panic!("expected LoadedCourse");
    };
    let ws_root = temp.path().join(name);
    fs::create_dir_all(&ws_root).unwrap();
    let AppResponse::WorkspaceBound(summary) = app
        .execute(AppCommand::BindWorkspace {
            path: path_str(&ws_root),
        })
        .unwrap()
    else {
        panic!("expected WorkspaceBound");
    };
    (summary.id, ws_root)
}

fn configure_agent(app: &App, command: &str, policy: PermissionPolicy, style: TeachingStyle) {
    let AppResponse::Settings(settings) = app
        .execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
            tutor: Some(TutorSettingsPatch {
                preset: Some(AcpAgentPreset::Custom),
                command: Some(command.to_owned()),
                permission_policy: Some(policy),
                teaching_style: Some(style),
            }),
            ..Default::default()
        }))
        .unwrap()
    else {
        panic!("expected Settings");
    };
    assert_eq!(settings.tutor.permission_policy, policy);
    assert_eq!(settings.tutor.teaching_style, style);
}

fn wait_for_snapshot(
    app: &App,
    workspace_id: u64,
    timeout: Duration,
    predicate: impl Fn(&TutorSessionSnapshot) -> bool,
) -> TutorSessionSnapshot {
    let deadline = Instant::now() + timeout;
    loop {
        let AppResponse::TutorSession(snapshot) = app
            .execute(AppCommand::GetTutorSession(WorkspaceTutorRequest {
                workspace_id,
            }))
            .unwrap()
        else {
            panic!("expected TutorSession");
        };
        if predicate(&snapshot) {
            return snapshot;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for tutor snapshot condition; latest: {snapshot:#?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn untrusted_course_never_spawns_acp_process_and_blocks_handshake_and_session() {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let marker = temp.path().join("spawn-marker.txt");
    let course_md = temp.path().join("untrusted.md");
    fs::write(&course_md, "# External Course\n\n## Ex 1\n\nHello.\n").unwrap();

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    app.execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
        path: path_str(&course_md),
    }))
    .unwrap();

    // Even with a valid command, an untrusted course blocks both handshake and prompt.
    let cmd = format!(
        "{} \"--marker={}\"",
        node_command(&script),
        path_str(&marker)
    );
    let handshake_err = app
        .execute(AppCommand::TestTutorHandshake(TutorHandshakeRequest {
            preset: AcpAgentPreset::Custom,
            command: cmd,
        }))
        .unwrap_err();
    assert_eq!(handshake_err.code, ErrorCode::CourseUntrusted);

    let prompt_err = app
        .execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
            workspace_id: 1,
            prompt: "help".into(),
            teaching_style: None,
            context_preamble: None,
            context: None,
        }))
        .unwrap_err();
    assert_eq!(prompt_err.code, ErrorCode::CourseUntrusted);
    assert!(!marker.exists(), "untrusted course must never spawn ACP");
}

#[test]
fn lazy_start_handshake_workspace_reuse_style_switch_clear_and_atomic_history() {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let marker = temp.path().join("lazy-marker.txt");
    let log_file = temp.path().join("acp.log");
    let agent_cmd = format!(
        "{} \"--marker={}\" \"--log={}\"",
        node_command(&script),
        path_str(&marker),
        path_str(&log_file)
    );

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let (workspace_id, ws_root) = open_trusted_workspace(&temp, &app, "work");
    configure_agent(
        &app,
        &agent_cmd,
        PermissionPolicy::GuidedReadonly,
        TeachingStyle::Socratic,
    );

    // Opening a trusted Workspace must NOT pre-spawn ACP (strict lazy startup).
    let AppResponse::TutorSession(initial) = app
        .execute(AppCommand::GetTutorSession(WorkspaceTutorRequest {
            workspace_id,
        }))
        .unwrap()
    else {
        panic!("expected TutorSession");
    };
    assert!(!initial.process_running);
    assert_eq!(initial.status, TutorSessionStatus::Idle);
    assert!(!marker.exists(), "opening workspace must not pre-start ACP");

    // Handshake test spawns once, verifies JSON-RPC initialize, and exits immediately.
    let AppResponse::TutorHandshake(handshake) = app
        .execute(AppCommand::TestTutorHandshake(TutorHandshakeRequest {
            preset: AcpAgentPreset::Custom,
            command: agent_cmd.clone(),
        }))
        .unwrap()
    else {
        panic!("expected TutorHandshake");
    };
    assert_eq!(handshake.agent_name.as_deref(), Some("Fake ACP Agent"));
    assert_eq!(handshake.protocol_version.as_deref(), Some("1"));

    // First prompt lazily starts the ACP process and uses default socratic style.
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "first question".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let after_first = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 2
    });
    assert!(after_first.process_running);
    assert_eq!(after_first.agent_name.as_deref(), Some("Fake ACP Agent"));
    let first_reply = &after_first.messages[1].content;
    assert!(first_reply.contains("style=socratic"));
    let first_session_id = first_reply.split("session=").nth(1).unwrap().to_owned();

    // Switch teaching style to direct; second prompt MUST reuse the exact same ACP session.
    app.execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
        tutor: Some(TutorSettingsPatch {
            teaching_style: Some(TeachingStyle::Direct),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .unwrap();
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "second question".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let after_second = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 4
    });
    assert!(after_second.session_reused);
    let second_reply = &after_second.messages[3].content;
    assert!(second_reply.contains("style=direct"));
    assert!(
        second_reply.contains(&format!("session={first_session_id}")),
        "expected workspace-level ACP session reuse, got {second_reply}"
    );

    // Verify tutor-history.json persisted atomically on disk and reloads in a new App instance.
    let history_path = ws_root.join(".codelab/tutor-history.json");
    assert!(history_path.is_file());
    app.shutdown().unwrap();

    let app2 = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let (ws2, _) = open_trusted_workspace(&temp, &app2, "work");
    let AppResponse::TutorSession(reloaded) = app2
        .execute(AppCommand::GetTutorSession(WorkspaceTutorRequest {
            workspace_id: ws2,
        }))
        .unwrap()
    else {
        panic!("expected TutorSession");
    };
    assert!(!reloaded.process_running, "reloaded workspace stays lazy");
    assert_eq!(reloaded.messages.len(), 4);

    // Clear conversation destroys ACP session & clears persisted history; next prompt starts new session.
    configure_agent(
        &app2,
        &agent_cmd,
        PermissionPolicy::GuidedReadonly,
        TeachingStyle::Socratic,
    );
    app2.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id: ws2,
        prompt: "before clear".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let before_clear = wait_for_snapshot(&app2, ws2, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 6
    });
    let old_sid = before_clear.messages[5]
        .content
        .split("session=")
        .nth(1)
        .unwrap()
        .to_owned();

    let AppResponse::TutorSession(cleared) = app2
        .execute(AppCommand::ClearTutorSession(WorkspaceTutorRequest {
            workspace_id: ws2,
        }))
        .unwrap()
    else {
        panic!("expected TutorSession");
    };
    assert!(cleared.messages.is_empty());
    assert!(!cleared.process_running);

    app2.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id: ws2,
        prompt: "after clear".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let after_clear = wait_for_snapshot(&app2, ws2, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 2
    });
    let new_sid = after_clear.messages[1]
        .content
        .split("session=")
        .nth(1)
        .unwrap()
        .to_owned();
    assert_ne!(old_sid, new_sid, "clear must rebuild ACP session");
    app2.shutdown().unwrap();

    // Corrupt tutor-history.json quarantines and recovers cleanly.
    fs::write(&history_path, "{corrupt json").unwrap();
    let app3 = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    app3.execute(AppCommand::OpenCourse(OpenCourseRequest::Builtin {
        course_id: "go-goframe-practice".into(),
    }))
    .unwrap();
    let AppResponse::WorkspaceBound(summary) = app3
        .execute(AppCommand::BindWorkspace {
            path: path_str(&ws_root),
        })
        .unwrap()
    else {
        panic!("expected WorkspaceBound");
    };
    assert!(
        summary
            .recovery_notices
            .iter()
            .any(|n| n.store == StoreKind::TutorHistory
                && n.error_code == ErrorCode::StorageCorrupted)
    );
}

#[test]
fn permission_policy_matrix_write_diff_and_workspace_change_events() {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let agent_cmd = node_command(&script);
    let outside_file = temp.path().join("outside.txt");
    fs::write(&outside_file, "EXTERNAL_DATA").unwrap();

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let events = app.subscribe();
    let (workspace_id, ws_root) = open_trusted_workspace(&temp, &app, "work");
    fs::write(ws_root.join("main.go"), "package main\n// original\n").unwrap();
    fs::write(ws_root.join(".env"), "SECRET_TOKEN=topsecret").unwrap();
    fs::write(ws_root.join("api_secret.json"), "{\"s\":1}").unwrap();
    fs::write(ws_root.join("private.key"), "KEYDATA").unwrap();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(ws_root.join(".env"), ws_root.join("alias.txt")).unwrap();
        std::os::unix::fs::symlink(&outside_file, ws_root.join("escape.txt")).unwrap();
    }

    configure_agent(
        &app,
        &agent_cmd,
        PermissionPolicy::GuidedReadonly,
        TeachingStyle::Direct,
    );

    // 1. guided_readonly: workspace non-sensitive read -> AutoAllowed (no prompt).
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:READ_SAFE".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let snap = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 2
    });
    assert!(snap.messages[1].content.contains("READ_OK:package main"));
    assert_eq!(
        snap.messages[1].permissions[0].status,
        TutorPermissionStatus::AutoAllowed
    );

    // 2. Sensitive files (.env, *secret*, *key*, symlink to .env) -> BlockedSensitive automatically.
    let sensitive_targets: &[&str] = if cfg!(unix) {
        &[".env", "api_secret.json", "private.key", "alias.txt"]
    } else {
        &[".env", "api_secret.json", "private.key"]
    };
    for (idx, target) in sensitive_targets.iter().enumerate() {
        let expected_len = 2 + (idx + 1) * 2;
        app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
            workspace_id,
            prompt: format!("CMD:READ_SENSITIVE:{target}"),
            teaching_style: None,
            context_preamble: None,
            context: None,
        }))
        .unwrap();
        let snap = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
            s.status == TutorSessionStatus::Ready && s.messages.len() == expected_len
        });
        let assistant = snap.messages.last().unwrap();
        assert_eq!(assistant.content, "SENSITIVE_BLOCKED");
        assert_eq!(
            assistant.permissions[0].status,
            TutorPermissionStatus::BlockedSensitive
        );
        assert!(assistant.permissions[0].sensitive);
    }

    // 3. Outside-workspace read -> Pending approval card (outside_workspace == true).
    let outside_target = path_str(&outside_file);
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: format!("CMD:READ_OUTSIDE:{outside_target}"),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let awaiting = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::AwaitingPermission
    });
    let perm = awaiting.messages.last().unwrap().permissions[0].clone();
    assert_eq!(perm.kind, TutorPermissionKind::Read);
    assert!(perm.outside_workspace);
    assert_eq!(perm.status, TutorPermissionStatus::Pending);
    // Deny outside read.
    app.execute(AppCommand::RespondTutorPermission(RespondTutorPermission {
        workspace_id,
        request_id: perm.id,
        allow: false,
    }))
    .unwrap();
    let after_deny = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready
    });
    assert_eq!(
        after_deny.messages.last().unwrap().content,
        "OUTSIDE_DENIED"
    );
    assert_eq!(
        after_deny.messages.last().unwrap().permissions[0].status,
        TutorPermissionStatus::Denied
    );

    // 4. Workspace write via fs/write_text_file -> Pending approval -> Approved captures before/after Diff & emits WorkspaceFilesChanged.
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:WRITE_FS".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let awaiting_write = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::AwaitingPermission
    });
    let write_perm = awaiting_write.messages.last().unwrap().permissions[0].clone();
    assert_eq!(write_perm.kind, TutorPermissionKind::Write);
    assert!(!write_perm.outside_workspace);
    app.execute(AppCommand::RespondTutorPermission(RespondTutorPermission {
        workspace_id,
        request_id: write_perm.id,
        allow: true,
    }))
    .unwrap();
    let after_write = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready
    });
    let write_msg = after_write.messages.last().unwrap();
    assert_eq!(write_msg.content, "WRITE_OK");
    assert_eq!(write_msg.file_changes.len(), 1);
    assert_eq!(write_msg.file_changes[0].path, "main.go");
    assert_eq!(
        write_msg.file_changes[0].before_text.as_deref(),
        Some("package main\n// original\n")
    );
    assert_eq!(
        write_msg.file_changes[0].after_text,
        "package main\n// updated by ACP fs/write\n"
    );
    assert_eq!(
        fs::read_to_string(ws_root.join("main.go")).unwrap(),
        "package main\n// updated by ACP fs/write\n"
    );

    // Verify WorkspaceFilesChanged event emitted for editor refresh / conflict detection.
    let mut saw_file_change_event = false;
    while let Ok(ev) = events.try_recv() {
        if let AppEvent::WorkspaceFilesChanged(changed) = ev
            && changed.workspace_id == workspace_id
            && changed.paths.iter().any(|p| p == "main.go")
        {
            saw_file_change_event = true;
        }
    }
    assert!(saw_file_change_event);

    // 5. Workspace direct disk write after session/request_permission -> captures diff too!
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:WRITE_DIRECT".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let awaiting_direct = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::AwaitingPermission
    });
    let direct_perm = awaiting_direct.messages.last().unwrap().permissions[0].clone();
    app.execute(AppCommand::RespondTutorPermission(RespondTutorPermission {
        workspace_id,
        request_id: direct_perm.id,
        allow: true,
    }))
    .unwrap();
    let after_direct = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready
    });
    let direct_msg = after_direct.messages.last().unwrap();
    assert_eq!(direct_msg.content, "DIRECT_WRITE_OK");
    assert_eq!(direct_msg.file_changes.len(), 1);
    assert_eq!(
        direct_msg.file_changes[0].after_text,
        "package main\n// direct disk write\n"
    );

    // 6. Command execution: normal vs high-risk command flagging.
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:EXEC:rm -rf /tmp/danger".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let awaiting_risk = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::AwaitingPermission
    });
    let risk_perm = awaiting_risk.messages.last().unwrap().permissions[0].clone();
    assert_eq!(risk_perm.kind, TutorPermissionKind::Execute);
    assert!(risk_perm.high_risk, "rm -rf must be flagged high_risk");
    app.execute(AppCommand::RespondTutorPermission(RespondTutorPermission {
        workspace_id,
        request_id: risk_perm.id,
        allow: false,
    }))
    .unwrap();
    wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready
    });

    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:EXEC:go test ./...".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let awaiting_safe_cmd = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::AwaitingPermission
    });
    let safe_cmd_perm = awaiting_safe_cmd.messages.last().unwrap().permissions[0].clone();
    assert_eq!(safe_cmd_perm.kind, TutorPermissionKind::Execute);
    assert!(!safe_cmd_perm.high_risk);
    app.execute(AppCommand::RespondTutorPermission(RespondTutorPermission {
        workspace_id,
        request_id: safe_cmd_perm.id,
        allow: true,
    }))
    .unwrap();
    let after_safe_cmd = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready
    });
    assert_eq!(
        after_safe_cmd.messages.last().unwrap().content,
        "EXEC_ALLOWED"
    );

    // 7. Switch policy to strict_ask_all -> even safe workspace read MUST prompt user!
    app.execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
        tutor: Some(TutorSettingsPatch {
            permission_policy: Some(PermissionPolicy::StrictAskAll),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .unwrap();
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:READ_SAFE".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let awaiting_strict = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::AwaitingPermission
    });
    let strict_perm = awaiting_strict.messages.last().unwrap().permissions[0].clone();
    assert_eq!(strict_perm.kind, TutorPermissionKind::Read);
    assert!(!strict_perm.outside_workspace);
    assert_eq!(strict_perm.status, TutorPermissionStatus::Pending);
    app.execute(AppCommand::RespondTutorPermission(RespondTutorPermission {
        workspace_id,
        request_id: strict_perm.id,
        allow: true,
    }))
    .unwrap();
    let after_strict = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready
    });
    assert!(
        after_strict
            .messages
            .last()
            .unwrap()
            .content
            .contains("READ_OK:")
    );
}

#[test]
fn streaming_stop_generation_crash_recovery_and_process_tree_cleanup() {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let pid_file = temp.path().join("pids.json");
    let agent_cmd = format!(
        "{} \"--pids={}\"",
        node_command(&script),
        path_str(&pid_file)
    );

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let (workspace_id, _) = open_trusted_workspace(&temp, &app, "work-1");
    configure_agent(
        &app,
        &agent_cmd,
        PermissionPolicy::GuidedReadonly,
        TeachingStyle::Socratic,
    );

    // 1. Streaming interruption via StopTutorTurn.
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:SLOW".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.messages
            .last()
            .is_some_and(|m| m.content.contains("streaming-part-1"))
    });
    let AppResponse::TutorSession(stopped) = app
        .execute(AppCommand::StopTutorTurn(WorkspaceTutorRequest {
            workspace_id,
        }))
        .unwrap()
    else {
        panic!("expected TutorSession");
    };
    assert_eq!(
        stopped.messages.last().unwrap().status,
        TutorMessageStatus::Cancelled
    );
    assert!(
        stopped
            .messages
            .last()
            .unwrap()
            .content
            .contains("streaming-part-1")
    );

    // 2. Crash mid-turn -> Failed with AcpProcessExited, can_retry == true, RetryTutorTurn recovers!
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:CRASH".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let crashed = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Error
            && !s.process_running
            && s.messages
                .last()
                .is_some_and(|m| m.status == TutorMessageStatus::Failed)
    });
    assert!(crashed.can_retry);
    assert_eq!(
        crashed.last_error.as_ref().map(|e| e.code),
        Some(ErrorCode::AcpProcessExited)
    );

    // Sending a fresh prompt or retrying respawns cleanly.
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "recover now".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let recovered = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.process_running
    });
    assert!(
        recovered
            .messages
            .last()
            .unwrap()
            .content
            .contains("style=socratic")
    );

    // 3. Process-tree cleanup on switching workspace.
    let pids: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&pid_file).unwrap()).unwrap();
    assert!(pids["pid"].as_i64().unwrap() > 0);
    assert!(pids["childPid"].as_i64().unwrap() > 0);

    let ws2_root = temp.path().join("work-2");
    fs::create_dir_all(&ws2_root).unwrap();
    app.execute(AppCommand::BindWorkspace {
        path: path_str(&ws2_root),
    })
    .unwrap();

    #[cfg(unix)]
    {
        let agent_pid = pids["pid"].as_i64().unwrap() as i32;
        let grandchild_pid = pids["childPid"].as_i64().unwrap() as i32;
        wait_process_dead(agent_pid);
        wait_process_dead(grandchild_pid);
    }

    // 4. Process-tree cleanup on App::shutdown().
    let AppResponse::WorkspaceBound(ws3) = app
        .execute(AppCommand::BindWorkspace {
            path: path_str(&ws2_root),
        })
        .unwrap()
    else {
        panic!("expected WorkspaceBound");
    };
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id: ws3.id,
        prompt: "alive before shutdown".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    wait_for_snapshot(&app, ws3.id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.process_running
    });
    let pids2: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&pid_file).unwrap()).unwrap();
    assert!(pids2["pid"].as_i64().unwrap() > 0);
    assert!(pids2["childPid"].as_i64().unwrap() > 0);
    app.shutdown().unwrap();

    #[cfg(unix)]
    {
        let agent_pid2 = pids2["pid"].as_i64().unwrap() as i32;
        let grandchild_pid2 = pids2["childPid"].as_i64().unwrap() as i32;
        wait_process_dead(agent_pid2);
        wait_process_dead(grandchild_pid2);
    }
}

#[cfg(unix)]
fn wait_process_dead(pid: i32) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let alive = unsafe { libc::kill(pid, 0) == 0 };
        if !alive {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "ACP process {pid} leaked after cleanup"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

struct InMemoryTutorBackend {
    starts: AtomicUsize,
    shutdowns: Arc<AtomicUsize>,
    fail_first_turn: Arc<Mutex<bool>>,
}

struct InMemorySession {
    sink: BackendEventSink,
    shutdowns: Arc<AtomicUsize>,
    fail_first_turn: Arc<Mutex<bool>>,
}

impl TutorBackend for InMemoryTutorBackend {
    fn handshake(&self, spec: &TutorLaunchSpec) -> Result<TutorHandshakeResult, DomainError> {
        Ok(TutorHandshakeResult {
            preset: spec.preset,
            command: spec.command.clone(),
            protocol_version: Some("1".into()),
            agent_name: Some("Memory Mock Agent".into()),
            agent_version: Some("0.1".into()),
        })
    }

    fn start_session(
        &self,
        _spec: &TutorLaunchSpec,
        sink: BackendEventSink,
    ) -> Result<Box<dyn TutorBackendSession>, DomainError> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(InMemorySession {
            sink,
            shutdowns: Arc::clone(&self.shutdowns),
            fail_first_turn: Arc::clone(&self.fail_first_turn),
        }))
    }
}

impl TutorBackendSession for InMemorySession {
    fn agent_name(&self) -> Option<String> {
        Some("Memory Mock Agent".into())
    }

    fn send_turn(&self, input: TutorTurnInput) -> Result<(), DomainError> {
        let should_fail = {
            let mut flag = self.fail_first_turn.lock().unwrap();
            let current = *flag;
            *flag = false;
            current
        };
        if should_fail {
            (self.sink)(BackendEvent::ProcessExited {
                exit_code: Some(1),
                unexpected: true,
            });
            return Ok(());
        }
        (self.sink)(BackendEvent::PermissionRequested {
            turn_id: input.turn_id.clone(),
            request: BackendPermissionPrompt {
                request_id: "mem-perm-1".into(),
                kind: TutorPermissionKind::Read,
                title: "Read main.go".into(),
                target_path: Some("main.go".into()),
                target_paths: vec!["main.go".into()],
                command: None,
                raw_input: None,
                write_content: None,
                is_fs_method: false,
            },
        });
        (self.sink)(BackendEvent::Chunk {
            turn_id: input.turn_id.clone(),
            delta: format!("memory-reply:{:?}:{}", input.teaching_style, input.prompt),
        });
        (self.sink)(BackendEvent::TurnCompleted {
            turn_id: input.turn_id,
            stop_reason: "end_turn".into(),
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

    fn shutdown(&self) {
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn in_memory_tutor_backend_adapter_seam_supports_retry_and_trust_revocation_cleanup() {
    let temp = TempDir::new().unwrap();
    let course_md = temp.path().join("course.md");
    fs::write(&course_md, "# Local\n\n## Ex 1\n\nBody.\n").unwrap();

    let shutdowns = Arc::new(AtomicUsize::new(0));
    let backend = Arc::new(InMemoryTutorBackend {
        starts: AtomicUsize::new(0),
        shutdowns: Arc::clone(&shutdowns),
        fail_first_turn: Arc::new(Mutex::new(true)),
    });

    let app = App::open_with_tutor_backend(
        AppConfig {
            data_dir: temp.path().join("data"),
        },
        backend.clone(),
    )
    .unwrap();
    app.execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
        path: path_str(&course_md),
    }))
    .unwrap();
    app.execute(AppCommand::GrantTrust(TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: path_str(&course_md),
    }))
    .unwrap();
    let ws_root = temp.path().join("work");
    fs::create_dir_all(&ws_root).unwrap();
    fs::write(ws_root.join("main.go"), "package main\n").unwrap();
    let AppResponse::WorkspaceBound(ws) = app
        .execute(AppCommand::BindWorkspace {
            path: path_str(&ws_root),
        })
        .unwrap()
    else {
        panic!("expected WorkspaceBound");
    };

    assert_eq!(backend.starts.load(Ordering::SeqCst), 0);

    // First turn crashes synchronously -> RetryTutorTurn respawns and succeeds!
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id: ws.id,
        prompt: "explain pointer".into(),
        teaching_style: Some(TeachingStyle::Socratic),
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let AppResponse::TutorSession(after_fail) = app
        .execute(AppCommand::GetTutorSession(WorkspaceTutorRequest {
            workspace_id: ws.id,
        }))
        .unwrap()
    else {
        panic!("expected TutorSession");
    };
    assert!(after_fail.can_retry);
    assert_eq!(backend.starts.load(Ordering::SeqCst), 1);

    let AppResponse::TutorSession(after_retry) = app
        .execute(AppCommand::RetryTutorTurn(WorkspaceTutorRequest {
            workspace_id: ws.id,
        }))
        .unwrap()
    else {
        panic!("expected TutorSession");
    };
    assert_eq!(backend.starts.load(Ordering::SeqCst), 2);
    assert_eq!(after_retry.status, TutorSessionStatus::Ready);
    let last = after_retry.messages.last().unwrap();
    assert_eq!(last.role, TutorRole::Assistant);
    assert!(
        last.content
            .contains("memory-reply:Socratic:explain pointer")
    );
    assert_eq!(
        last.permissions[0].status,
        TutorPermissionStatus::AutoAllowed
    );

    // Revoking trust tears down the active TutorBackendSession immediately.
    app.execute(AppCommand::RevokeTrust(TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: path_str(&course_md),
    }))
    .unwrap();
    assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
}

#[test]
fn permission_policy_handles_key_named_ancestors_multi_location_allow_once_and_mid_turn_agent_switch()
 {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let agent_cmd = node_command(&script);

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    // Workspace placed under a parent directory containing `key` ("mickey") must NOT falsely block workspace reads.
    let (workspace_id, ws_root) = open_trusted_workspace(&temp, &app, "mickey/work");
    fs::write(ws_root.join("main.go"), "package main\n").unwrap();
    fs::write(ws_root.join(".env"), "TOKEN=1\n").unwrap();

    configure_agent(
        &app,
        &agent_cmd,
        PermissionPolicy::GuidedReadonly,
        TeachingStyle::Direct,
    );

    // 1. Ancestor containing "key" does not block non-sensitive workspace read; allow_once preferred over allow_always.
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:ALLOW_ALWAYS_FIRST".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let snap = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 2
    });
    assert_eq!(snap.messages[1].content, "OPTION:perm_once");
    assert_eq!(
        snap.messages[1].permissions[0].status,
        TutorPermissionStatus::AutoAllowed
    );

    // 2. Multi-location read containing both main.go and .env MUST be BlockedSensitive.
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:MULTI_LOC_SENSITIVE".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let snap_multi = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 4
    });
    assert_eq!(
        snap_multi.messages[3].permissions[0].status,
        TutorPermissionStatus::BlockedSensitive
    );
    assert_eq!(snap_multi.messages[3].content, "MULTI_RESULT:reject_once");

    // 3. Switching Agent mid-turn cancels the in-flight streaming message cleanly instead of stranding Streaming.
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:SLOW".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.messages
            .last()
            .is_some_and(|m| m.content.contains("streaming-part-1"))
    });
    app.execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
        tutor: Some(TutorSettingsPatch {
            preset: Some(AcpAgentPreset::GeminiCli),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .unwrap();
    let AppResponse::TutorSession(after_switch) = app
        .execute(AppCommand::GetTutorSession(WorkspaceTutorRequest {
            workspace_id,
        }))
        .unwrap()
    else {
        panic!("expected TutorSession");
    };
    assert!(!after_switch.process_running);
    assert_eq!(
        after_switch.messages.last().unwrap().status,
        TutorMessageStatus::Cancelled
    );
}

fn read_acp_prompt_logs(log_path: &Path) -> Vec<serde_json::Value> {
    fs::read_to_string(log_path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|entry| entry.get("method").and_then(|m| m.as_str()) == Some("session/prompt"))
        .collect()
}

fn wait_for_terminal_errors(app: &App, workspace_id: u64, min_lines: usize) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let AppResponse::Terminals(updates) = app
            .execute(AppCommand::ReadTerminals(TerminalRead {
                workspace_id,
                terminal_id: None,
                cursor: 0,
                include_recent_errors: Some(true),
            }))
            .unwrap()
        else {
            panic!("expected Terminals");
        };
        if updates.recent_errors.len() >= min_lines
            && updates.terminals.iter().all(|term| !term.running)
        {
            return updates.recent_errors;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {min_lines} terminal errors; got {}",
            updates.recent_errors.len()
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_verification_done(app: &App, workspace_id: u64) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let AppResponse::Progress(progress) = app
            .execute(AppCommand::GetProgress(
                codelab_core::WorkspaceTerminalRequest { workspace_id },
            ))
            .unwrap()
        else {
            panic!("expected Progress");
        };
        if progress.running_chapter_id.is_none()
            && progress
                .chapters
                .values()
                .any(|ch| !ch.last_check_results.is_empty())
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for verification completion"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn create_context_test_pack(
    dir: &Path,
    chapter_body: &str,
    error_script: &str,
    check_command: &str,
) -> PathBuf {
    let pack = dir.join("ctx-pack");
    fs::create_dir_all(pack.join("starter")).unwrap();
    fs::write(
        pack.join("codelab.json"),
        serde_json::json!({
            "schemaVersion": "1.0",
            "courseId": "ctx-test-pack",
            "title": "Context Snapshot Test Pack",
            "description": "Tests 4D Context Snapshot and 6k truncation",
            "language": "Go",
            "entryFile": "COURSE.md",
            "starterDir": "starter",
            "checksFile": ".codelab-checks.json"
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("COURSE.md"),
        format!(
            "# Context Snapshot Test Pack\n\n\
             ## Exercise 1: Build Health Endpoint {{id=ex-1}}\n\n\
             {chapter_body}\n\n\
             ```bash\n\
             {error_script}\n\
             ```\n\n\
             ## Exercise 2: Add Task Router {{id=ex-2}}\n\n\
             Implement POST /tasks handler and return 201 Created with JSON payload.\n"
        ),
    )
    .unwrap();
    fs::write(
        pack.join(".codelab-checks.json"),
        serde_json::json!({
            "schemaVersion": "1.0",
            "chapters": [
                {
                    "chapterId": "ex-1",
                    "checks": [
                        {
                            "id": "c1",
                            "title": "Health endpoint check",
                            "type": "command_test",
                            "command": check_command,
                            "expectExitCode": 0,
                            "timeoutSec": 5
                        }
                    ]
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("starter/main.go"),
        "package main\n\nfunc healthHandler() string {\n\treturn \"broken\"\n}\n",
    )
    .unwrap();
    pack
}

#[test]
fn context_snapshot_builds_four_dimensions_and_chip_removal_omits_dimension_from_acp_prompt() {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let log_file = temp.path().join("acp-ctx.log");
    let agent_cmd = format!(
        "{} \"--log={}\"",
        node_command(&script),
        path_str(&log_file)
    );

    let pack_dir = create_context_test_pack(
        temp.path(),
        "CHAPTER_MARKER_HEALTH: Build the GET /healthz handler returning 200 OK.",
        "printf 'error: bind failed on :8080\\nerror: address already in use\\n'",
        "printf 'VERIFY_RAW_MARKER: expected status 200 got 500\\n' && exit 1",
    );

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    app.execute(AppCommand::GrantTrust(TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: path_str(&pack_dir),
    }))
    .unwrap();
    let AppResponse::LoadedCourse(course) = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalPack {
            path: path_str(&pack_dir),
        }))
        .unwrap()
    else {
        panic!("expected LoadedCourse");
    };
    let ex1_cmd_id = course.chapters[0].commands[0].id.clone();

    let ws_root = temp.path().join("work-ctx");
    fs::create_dir_all(&ws_root).unwrap();
    let AppResponse::WorkspaceBound(ws) = app
        .execute(AppCommand::BindWorkspace {
            path: path_str(&ws_root),
        })
        .unwrap()
    else {
        panic!("expected WorkspaceBound");
    };
    configure_agent(
        &app,
        &agent_cmd,
        PermissionPolicy::GuidedReadonly,
        TeachingStyle::Socratic,
    );

    // Trigger real terminal error lines and a real failing verification check.
    app.execute(AppCommand::RunCommandBlock(RunCommandBlock {
        workspace_id: ws.id,
        command_id: ex1_cmd_id,
        confirmation: None,
        restart_terminal_id: None,
        drafts: Vec::new(),
    }))
    .unwrap();
    wait_for_terminal_errors(&app, ws.id, 2);

    app.execute(AppCommand::RunChecks(RunChecks {
        target: ExerciseTarget {
            workspace_id: ws.id,
            chapter_id: "ex-1".into(),
        },
        drafts: Vec::new(),
        confirmation: None,
    }))
    .unwrap();
    wait_for_verification_done(&app, ws.id);

    // 1. Build 4D Context Snapshot with all dimensions included.
    let quote = ContextQuote {
        source: ContextQuoteSource::Editor,
        file_path: Some("main.go".into()),
        start_line: Some(3),
        end_line: Some(5),
        label: None,
        text: "func healthHandler() string {\n\treturn \"broken\"\n}".into(),
    };
    let AppResponse::TutorContext(full_ctx) = app
        .execute(AppCommand::BuildTutorContext(ContextSnapshotInput {
            workspace_id: ws.id,
            chapter_id: Some("ex-1".into()),
            active_file_path: Some("main.go".into()),
            active_file_text: None,
            active_file_start_line: Some(3),
            active_file_end_line: Some(5),
            quote: Some(quote.clone()),
            excluded_dimensions: Vec::new(),
            prompt: Some("Why is my health check failing?".into()),
            teaching_style: Some(TeachingStyle::Socratic),
        }))
        .unwrap()
    else {
        panic!("expected TutorContext");
    };

    assert_eq!(full_ctx.max_tokens, MAX_CONTEXT_INPUT_TOKENS);
    assert!(full_ctx.estimated_tokens > 0 && full_ctx.estimated_tokens <= 6_000);
    assert!(!full_ctx.sensitive_filtered);
    assert_eq!(full_ctx.quote.as_ref(), Some(&quote));
    let dims: Vec<ContextDimension> = full_ctx.chips.iter().map(|c| c.dimension).collect();
    assert!(dims.contains(&ContextDimension::Chapter));
    assert!(dims.contains(&ContextDimension::ActiveFile));
    assert!(dims.contains(&ContextDimension::TerminalErrors));
    assert!(dims.contains(&ContextDimension::Verification));
    assert!(full_ctx.rendered_preamble.contains("CHAPTER_MARKER_HEALTH"));
    assert!(full_ctx.rendered_preamble.contains("func healthHandler()"));
    assert!(
        full_ctx
            .rendered_preamble
            .contains("address already in use")
    );
    assert!(full_ctx.rendered_preamble.contains("Health endpoint check"));

    // 2. Remove Chapter & TerminalErrors chips and send prompt -> verify ACP prompt excludes both!
    let excluded = vec![ContextDimension::Chapter, ContextDimension::TerminalErrors];
    let ctx_without_two = ContextSnapshotInput {
        workspace_id: ws.id,
        chapter_id: Some("ex-1".into()),
        active_file_path: Some("main.go".into()),
        active_file_text: None,
        active_file_start_line: Some(3),
        active_file_end_line: Some(5),
        quote: Some(quote.clone()),
        excluded_dimensions: excluded.clone(),
        prompt: Some("Why is my health check failing?".into()),
        teaching_style: Some(TeachingStyle::Socratic),
    };
    let AppResponse::TutorContext(reduced_ctx) = app
        .execute(AppCommand::BuildTutorContext(ctx_without_two.clone()))
        .unwrap()
    else {
        panic!("expected TutorContext");
    };
    let reduced_dims: Vec<ContextDimension> =
        reduced_ctx.chips.iter().map(|c| c.dimension).collect();
    assert!(!reduced_dims.contains(&ContextDimension::Chapter));
    assert!(!reduced_dims.contains(&ContextDimension::TerminalErrors));
    assert!(reduced_dims.contains(&ContextDimension::ActiveFile));
    assert!(reduced_dims.contains(&ContextDimension::Verification));
    assert!(reduced_ctx.estimated_tokens < full_ctx.estimated_tokens);

    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id: ws.id,
        prompt: "Why is my health check failing?".into(),
        teaching_style: Some(TeachingStyle::Socratic),
        context_preamble: None,
        context: Some(ctx_without_two),
    }))
    .unwrap();
    let snap = wait_for_snapshot(&app, ws.id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 2
    });
    assert_eq!(snap.messages[0].quote.as_ref(), Some(&quote));
    assert_eq!(snap.messages[0].context_dimensions, reduced_dims);

    let prompts = read_acp_prompt_logs(&log_file);
    assert_eq!(prompts.len(), 1);
    let sent_text = prompts[0]["text"].as_str().unwrap();
    assert!(sent_text.contains("[Teaching Style: socratic]"));
    assert!(sent_text.contains("func healthHandler()"));
    assert!(sent_text.contains("Health endpoint check"));
    assert!(
        !sent_text.contains("CHAPTER_MARKER_HEALTH"),
        "removed Chapter chip must not be sent in ACP prompt"
    );
    assert!(
        !sent_text.contains("address already in use"),
        "removed TerminalErrors chip must not be sent in ACP prompt"
    );
    app.shutdown().unwrap();
}

#[test]
fn context_snapshot_filters_sensitive_files_and_symlinks_never_leaking_secrets() {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let log_file = temp.path().join("acp-sensitive.log");
    let agent_cmd = format!(
        "{} \"--log={}\"",
        node_command(&script),
        path_str(&log_file)
    );
    let outside_file = temp.path().join("outside-secret.txt");
    fs::write(&outside_file, "OUTSIDE_LEAK_TOKEN_999").unwrap();

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    // Parent directory containing "key" ("mickey") must NOT falsely block legitimate workspace files.
    let (workspace_id, ws_root) = open_trusted_workspace(&temp, &app, "mickey/work");
    fs::write(ws_root.join("main.go"), "package main\n// SAFE_CODE_OK\n").unwrap();
    fs::write(ws_root.join(".env"), "DB_PASS=SUPER_SECRET_ENV_VALUE").unwrap();
    fs::write(ws_root.join(".env.local"), "LOCAL_TOKEN=SECRET_LOCAL_ENV").unwrap();
    fs::create_dir_all(ws_root.join("config")).unwrap();
    fs::write(
        ws_root.join("config/api_secret.json"),
        "{\"token\":\"API_SECRET_JSON_VALUE\"}",
    )
    .unwrap();
    fs::write(ws_root.join("private.key"), "BEGIN_PRIVATE_KEY_VALUE").unwrap();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(ws_root.join(".env"), ws_root.join("innocent.txt")).unwrap();
        std::os::unix::fs::symlink(&outside_file, ws_root.join("escape.txt")).unwrap();
    }

    configure_agent(
        &app,
        &agent_cmd,
        PermissionPolicy::GuidedReadonly,
        TeachingStyle::Direct,
    );

    // Non-sensitive workspace file under mickey/work MUST be included normally.
    let AppResponse::TutorContext(safe_ctx) = app
        .execute(AppCommand::BuildTutorContext(ContextSnapshotInput {
            workspace_id,
            chapter_id: Some("ex-1".into()),
            active_file_path: Some("main.go".into()),
            ..Default::default()
        }))
        .unwrap()
    else {
        panic!("expected TutorContext");
    };
    assert!(!safe_ctx.sensitive_filtered);
    assert!(safe_ctx.rendered_preamble.contains("SAFE_CODE_OK"));

    let sensitive_cases: &[(&str, &str)] = if cfg!(unix) {
        &[
            (".env", "SUPER_SECRET_ENV_VALUE"),
            (".env.local", "SECRET_LOCAL_ENV"),
            ("config/api_secret.json", "API_SECRET_JSON_VALUE"),
            ("private.key", "BEGIN_PRIVATE_KEY_VALUE"),
            ("innocent.txt", "SUPER_SECRET_ENV_VALUE"),
        ]
    } else {
        &[
            (".env", "SUPER_SECRET_ENV_VALUE"),
            (".env.local", "SECRET_LOCAL_ENV"),
            ("config/api_secret.json", "API_SECRET_JSON_VALUE"),
            ("private.key", "BEGIN_PRIVATE_KEY_VALUE"),
        ]
    };

    for (path, secret_val) in sensitive_cases {
        let input = ContextSnapshotInput {
            workspace_id,
            chapter_id: Some("ex-1".into()),
            active_file_path: Some((*path).into()),
            active_file_text: Some(format!("DRAFT_{secret_val}")),
            quote: Some(ContextQuote {
                source: ContextQuoteSource::Editor,
                file_path: Some((*path).into()),
                start_line: Some(1),
                end_line: Some(1),
                label: None,
                text: format!("QUOTE_{secret_val}"),
            }),
            ..Default::default()
        };
        let AppResponse::TutorContext(ctx) = app
            .execute(AppCommand::BuildTutorContext(input.clone()))
            .unwrap()
        else {
            panic!("expected TutorContext");
        };
        assert!(
            ctx.sensitive_filtered,
            "expected sensitive_filtered for {path}"
        );
        assert!(
            ctx.quote.is_none(),
            "sensitive quote must be stripped for {path}"
        );
        assert!(
            !ctx.chips
                .iter()
                .any(|c| c.dimension == ContextDimension::ActiveFile),
            "sensitive ActiveFile chip must be omitted for {path}"
        );
        assert!(
            !ctx.rendered_preamble.contains(secret_val),
            "secret leaked in preamble for {path}"
        );
    }

    // Outside-workspace path must also never enter context.
    let outside_target = if cfg!(unix) {
        "escape.txt".to_owned()
    } else {
        path_str(&outside_file)
    };
    let AppResponse::TutorContext(outside_ctx) = app
        .execute(AppCommand::BuildTutorContext(ContextSnapshotInput {
            workspace_id,
            chapter_id: Some("ex-1".into()),
            active_file_path: Some(outside_target),
            active_file_text: Some("OUTSIDE_LEAK_TOKEN_999".into()),
            ..Default::default()
        }))
        .unwrap()
    else {
        panic!("expected TutorContext");
    };
    assert!(
        !outside_ctx
            .rendered_preamble
            .contains("OUTSIDE_LEAK_TOKEN_999")
    );

    // Sending a prompt with `.env` attached must NOT leak secrets to the ACP agent!
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "Check my config".into(),
        teaching_style: Some(TeachingStyle::Direct),
        context_preamble: None,
        context: Some(ContextSnapshotInput {
            workspace_id,
            chapter_id: Some("ex-1".into()),
            active_file_path: Some(".env".into()),
            active_file_text: Some("DB_PASS=SUPER_SECRET_ENV_VALUE".into()),
            ..Default::default()
        }),
    }))
    .unwrap();
    wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 2
    });
    let prompts = read_acp_prompt_logs(&log_file);
    let sent_text = prompts[0]["text"].as_str().unwrap();
    assert!(!sent_text.contains("SUPER_SECRET_ENV_VALUE"));
    app.shutdown().unwrap();
}

#[test]
fn context_snapshot_enforces_6k_token_budget_protecting_selection_and_latest_error_tail() {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let log_file = temp.path().join("acp-budget.log");
    let agent_cmd = format!(
        "{} \"--log={}\"",
        node_command(&script),
        path_str(&log_file)
    );

    // Huge chapter (~18,000 ASCII chars -> ~4,500 tokens)
    let mut huge_chapter = String::from("CHAPTER_HEAD_KEEP ");
    for i in 0..220 {
        huge_chapter.push_str(&format!(
            "Paragraph {i:03}: detailed architectural explanation about HTTP server lifecycle and routing conventions.\n"
        ));
    }
    huge_chapter.push_str("CHAPTER_TAIL_LOW_PRIORITY_DROP");

    // Emit 210 error lines (~26,000 chars -> ~6,500 tokens alone if unpruned!)
    // Line 1..209 have filler errors; final line 210 is the latest critical tail error.
    let error_script = "node -e 'for (let i = 1; i <= 209; i++) { console.log(\"error: noisy historic stack frame #\" + String(i).padStart(3, \"0\") + \" \" + \"x\".repeat(110)); } console.log(\"error: LATEST_CRITICAL_TAIL_ERROR_210 address already in use :8080\");'";

    // Failing check with massive raw output (~14,000 chars -> ~3,500 tokens)
    let check_cmd = "node -e 'for (let i = 0; i < 140; i++) { console.log(\"VERIFY_RAW_NOISE_\" + i + \" \" + \"y\".repeat(80)); } process.exit(1);'";

    let pack_dir = create_context_test_pack(temp.path(), &huge_chapter, error_script, check_cmd);

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    app.execute(AppCommand::GrantTrust(TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: path_str(&pack_dir),
    }))
    .unwrap();
    let AppResponse::LoadedCourse(course) = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalPack {
            path: path_str(&pack_dir),
        }))
        .unwrap()
    else {
        panic!("expected LoadedCourse");
    };
    let ex1_cmd_id = course.chapters[0].commands[0].id.clone();

    let ws_root = temp.path().join("work-budget");
    fs::create_dir_all(&ws_root).unwrap();
    let AppResponse::WorkspaceBound(ws) = app
        .execute(AppCommand::BindWorkspace {
            path: path_str(&ws_root),
        })
        .unwrap()
    else {
        panic!("expected WorkspaceBound");
    };

    // Huge source file (~20,000 ASCII chars -> ~5,000 tokens)
    let mut huge_code = String::from("package main\n\n");
    for i in 1..=260 {
        huge_code.push_str(&format!(
            "func helperFunc{i:03}() string {{ return \"padding_code_segment_{i:03}_abcdef1234567890\" }}\n"
        ));
    }
    fs::write(ws_root.join("main.go"), &huge_code).unwrap();

    configure_agent(
        &app,
        &agent_cmd,
        PermissionPolicy::GuidedReadonly,
        TeachingStyle::Direct,
    );

    // Run terminal error producer & failing check.
    app.execute(AppCommand::RunCommandBlock(RunCommandBlock {
        workspace_id: ws.id,
        command_id: ex1_cmd_id,
        confirmation: None,
        restart_terminal_id: None,
        drafts: Vec::new(),
    }))
    .unwrap();
    wait_for_terminal_errors(&app, ws.id, 180);

    app.execute(AppCommand::RunChecks(RunChecks {
        target: ExerciseTarget {
            workspace_id: ws.id,
            chapter_id: "ex-1".into(),
        },
        drafts: Vec::new(),
        confirmation: None,
    }))
    .unwrap();
    wait_for_verification_done(&app, ws.id);

    let selection_text =
        "EXACT_USER_SELECTION_KEEP_VERBATIM: func helperFunc130() string { return \"selected\" }";
    let input = ContextSnapshotInput {
        workspace_id: ws.id,
        chapter_id: Some("ex-1".into()),
        active_file_path: Some("main.go".into()),
        active_file_text: None,
        active_file_start_line: Some(130),
        active_file_end_line: Some(132),
        quote: Some(ContextQuote {
            source: ContextQuoteSource::Editor,
            file_path: Some("main.go".into()),
            start_line: Some(130),
            end_line: Some(132),
            label: None,
            text: selection_text.into(),
        }),
        excluded_dimensions: Vec::new(),
        prompt: Some("Diagnose why port 8080 failed".into()),
        teaching_style: Some(TeachingStyle::Direct),
    };

    let AppResponse::TutorContext(snapshot) = app
        .execute(AppCommand::BuildTutorContext(input.clone()))
        .unwrap()
    else {
        panic!("expected TutorContext");
    };
    assert!(snapshot.truncated, "oversized context must be truncated");
    assert!(
        snapshot.estimated_tokens <= MAX_CONTEXT_INPUT_TOKENS,
        "estimated tokens {} exceeded 6000 budget",
        snapshot.estimated_tokens
    );

    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id: ws.id,
        prompt: "Diagnose why port 8080 failed".into(),
        teaching_style: Some(TeachingStyle::Direct),
        context_preamble: None,
        context: Some(input),
    }))
    .unwrap();
    wait_for_snapshot(&app, ws.id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 2
    });

    let prompts = read_acp_prompt_logs(&log_file);
    assert_eq!(prompts.len(), 1);
    let sent_text = prompts[0]["text"].as_str().unwrap();

    // Independent size sanity check: 6,000 ASCII tokens ~= 24,000 chars max.
    assert!(
        sent_text.chars().count() <= 24_200,
        "actual ACP prompt exceeded 6k token char ceiling: {} chars",
        sent_text.chars().count()
    );
    // Protected content MUST survive verbatim!
    assert!(
        sent_text.contains(selection_text),
        "user selection must NEVER be truncated"
    );
    assert!(
        sent_text.contains("LATEST_CRITICAL_TAIL_ERROR_210 address already in use :8080"),
        "latest terminal error tail must NEVER be truncated"
    );
    // Lowest priority content must have been truncated first!
    assert!(
        !sent_text.contains("CHAPTER_TAIL_LOW_PRIORITY_DROP"),
        "low-priority chapter tail should be truncated when over 6k budget"
    );
    app.shutdown().unwrap();
}

#[test]
fn chapter_switch_and_style_switch_reuse_workspace_acp_session_with_fresh_context_snapshot() {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let log_file = temp.path().join("acp-reuse.log");
    let agent_cmd = format!(
        "{} \"--log={}\"",
        node_command(&script),
        path_str(&log_file)
    );

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let (workspace_id, _ws_root) = open_trusted_workspace(&temp, &app, "work-reuse");
    configure_agent(
        &app,
        &agent_cmd,
        PermissionPolicy::GuidedReadonly,
        TeachingStyle::Socratic,
    );

    // Turn 1: ex-0 + Socratic
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "First turn question".into(),
        teaching_style: Some(TeachingStyle::Socratic),
        context_preamble: None,
        context: Some(ContextSnapshotInput {
            workspace_id,
            chapter_id: Some("ex-0".into()),
            ..Default::default()
        }),
    }))
    .unwrap();
    let after_first = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 2
    });
    let first_sid = after_first.messages[1]
        .content
        .split("session=")
        .nth(1)
        .unwrap()
        .to_owned();

    // Switch Chapter to ex-1 AND switch teaching style to Direct -> MUST reuse same ACP session!
    app.execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
        tutor: Some(TutorSettingsPatch {
            teaching_style: Some(TeachingStyle::Direct),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .unwrap();
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "Second turn on next chapter".into(),
        teaching_style: Some(TeachingStyle::Direct),
        context_preamble: None,
        context: Some(ContextSnapshotInput {
            workspace_id,
            chapter_id: Some("ex-1".into()),
            ..Default::default()
        }),
    }))
    .unwrap();
    let after_second = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.messages.len() == 4
    });
    assert!(after_second.session_reused);
    assert!(
        after_second.messages[3]
            .content
            .contains(&format!("session={first_sid}")),
        "switching Chapter and style must reuse Workspace ACP session"
    );

    let prompts = read_acp_prompt_logs(&log_file);
    assert_eq!(prompts.len(), 2);
    let p1 = prompts[0]["text"].as_str().unwrap();
    let p2 = prompts[1]["text"].as_str().unwrap();
    assert!(p1.contains("[Teaching Style: socratic]"));
    assert!(p1.contains("ex-0"));
    assert!(p2.contains("[Teaching Style: direct]"));
    assert!(p2.contains("ex-1"));
    assert_eq!(prompts[0]["sessionId"], prompts[1]["sessionId"]);

    // A large (>120 lines) file that still fits well below 6k tokens must NOT falsely flag truncated.
    let mut medium_file = String::from("package main\n");
    for i in 1..=140 {
        medium_file.push_str(&format!("// line {i}\n"));
    }
    fs::write(_ws_root.join("medium.go"), &medium_file).unwrap();
    let AppResponse::TutorContext(medium_ctx) = app
        .execute(AppCommand::BuildTutorContext(ContextSnapshotInput {
            workspace_id,
            chapter_id: Some("ex-1".into()),
            active_file_path: Some("medium.go".into()),
            ..Default::default()
        }))
        .unwrap()
    else {
        panic!("expected TutorContext");
    };
    assert!(
        !medium_ctx.truncated,
        "windowing a file under 6k tokens must not falsely flag 6k budget truncation"
    );

    // RetryTutorTurn rebuilds a fresh budgeted Context Snapshot with the latest style & file edits.
    fs::write(
        _ws_root.join("medium.go"),
        "package main\n// RETRY_FRESH_EDIT_MARKER\n",
    )
    .unwrap();
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:CRASH".into(),
        teaching_style: Some(TeachingStyle::Direct),
        context_preamble: None,
        context: Some(ContextSnapshotInput {
            workspace_id,
            chapter_id: Some("ex-1".into()),
            active_file_path: Some("medium.go".into()),
            ..Default::default()
        }),
    }))
    .unwrap();
    wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| s.can_retry);
    fs::write(
        _ws_root.join("medium.go"),
        "package main\n// RETRY_UPDATED_AFTER_CRASH\n",
    )
    .unwrap();
    app.execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
        tutor: Some(TutorSettingsPatch {
            teaching_style: Some(TeachingStyle::Socratic),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .unwrap();
    app.execute(AppCommand::RetryTutorTurn(
        codelab_core::WorkspaceTutorRequest { workspace_id },
    ))
    .unwrap();
    wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.messages.len() == 8 && s.can_retry
    });
    let retry_prompt = read_acp_prompt_logs(&log_file).last().unwrap()["text"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(retry_prompt.contains("[Teaching Style: socratic]"));
    assert!(retry_prompt.contains("RETRY_UPDATED_AFTER_CRASH"));
    app.shutdown().unwrap();
}

#[test]
fn detached_acp_grandchild_is_reaped_on_crash_and_shutdown() {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let pid_file = temp.path().join("detached-pids.json");
    let agent_cmd = format!(
        "{} \"--pids={}\" --detach",
        node_command(&script),
        path_str(&pid_file)
    );

    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let (workspace_id, _) = open_trusted_workspace(&temp, &app, "work-detached");
    configure_agent(
        &app,
        &agent_cmd,
        PermissionPolicy::GuidedReadonly,
        TeachingStyle::Socratic,
    );

    // 1. Start session so the detached grandchild is definitely listening, then trigger CMD:CRASH.
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "warm detached".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.process_running
    });
    let crash_pids = wait_for_pid_file(&pid_file);
    let crash_port = crash_pids["childPort"].as_u64().unwrap() as u16;
    assert!(crash_pids["pid"].as_i64().unwrap() > 0);
    assert!(crash_pids["childPid"].as_i64().unwrap() > 0);
    assert!(
        std::net::TcpListener::bind(("127.0.0.1", crash_port)).is_err(),
        "detached grandchild must hold port {crash_port} before crash"
    );

    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "CMD:CRASH".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    let crashed = wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Error && !s.process_running && s.can_retry
    });
    let err = crashed.last_error.as_ref().expect("crash error");
    assert_eq!(err.code, ErrorCode::AcpProcessExited);
    assert_eq!(err.details.get("code").map(String::as_str), Some("42"));

    wait_port_released(crash_port);
    #[cfg(unix)]
    {
        wait_process_dead(crash_pids["pid"].as_i64().unwrap() as i32);
        wait_process_dead(crash_pids["childPid"].as_i64().unwrap() as i32);
    }

    // 2. Explicit shutdown reaps a fresh detached grandchild on every platform.
    fs::remove_file(&pid_file).unwrap();
    app.execute(AppCommand::SendTutorPrompt(SendTutorPrompt {
        workspace_id,
        prompt: "hello detached".into(),
        teaching_style: None,
        context_preamble: None,
        context: None,
    }))
    .unwrap();
    wait_for_snapshot(&app, workspace_id, Duration::from_secs(5), |s| {
        s.status == TutorSessionStatus::Ready && s.process_running
    });
    let shutdown_pids = wait_for_pid_file(&pid_file);
    let shutdown_port = shutdown_pids["childPort"].as_u64().unwrap() as u16;
    assert!(shutdown_pids["pid"].as_i64().unwrap() > 0);
    assert!(shutdown_pids["childPid"].as_i64().unwrap() > 0);
    assert!(
        std::net::TcpListener::bind(("127.0.0.1", shutdown_port)).is_err(),
        "detached grandchild must hold port {shutdown_port} before shutdown"
    );

    app.shutdown().unwrap();

    wait_port_released(shutdown_port);
    #[cfg(unix)]
    {
        wait_process_dead(shutdown_pids["pid"].as_i64().unwrap() as i32);
        wait_process_dead(shutdown_pids["childPid"].as_i64().unwrap() as i32);
    }
}

#[test]
fn trailing_separator_program_is_not_resolved_from_path() {
    let temp = TempDir::new().unwrap();
    let script = write_fake_agent(temp.path());
    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let err = app
        .execute(AppCommand::TestTutorHandshake(TutorHandshakeRequest {
            preset: AcpAgentPreset::Custom,
            command: format!("node/ \"{}\"", path_str(&script)),
        }))
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::AcpAgentStartFailed);
}

fn wait_for_pid_file(path: &Path) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(text) = fs::read_to_string(path)
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
        {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for detached grandchild pid file {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn wait_port_released(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "detached ACP grandchild kept port {port} bound after cleanup"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

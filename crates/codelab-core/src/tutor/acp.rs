use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use crate::error::{DomainError, ErrorCode};
use crate::execution::{Directory, Process, spawn_piped};
use crate::filesystem::open_canonical_dir;

use super::backend::{
    BackendEvent, BackendEventSink, BackendPermissionPrompt, TutorBackend, TutorBackendSession,
    TutorLaunchSpec, TutorTurnInput, compose_turn_prompt,
};
use super::model::{AcpAgentPreset, TutorHandshakeResult, TutorPermissionKind};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const EXIT_CODE_GRACE: Duration = Duration::from_millis(250);
const MAX_RPC_LINE_BYTES: usize = 4 * 1024 * 1024;
const ACP_AUTH_REQUIRED_CODE: i64 = -32000;
const CLIENT_NAME: &str = "codelab-studio";
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

type StdinWriter = Box<dyn Write + Send>;

pub struct AcpStdioBackend;

impl TutorBackend for AcpStdioBackend {
    fn handshake(&self, spec: &TutorLaunchSpec) -> Result<TutorHandshakeResult, DomainError> {
        let (program, args) = split_command_line(&spec.command)?;
        let (child, stdout, stdin) = spawn_acp_process(spec, &program, &args)?;

        let child = Arc::new(Mutex::new(Some(child)));
        let stdin = Arc::new(Mutex::new(Some(stdin)));
        let pending: PendingClientMap = Arc::new(Mutex::new(HashMap::new()));

        let reader_pending = Arc::clone(&pending);
        let reader_child = Arc::clone(&child);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(line) = read_bounded_line(&mut reader, MAX_RPC_LINE_BYTES) {
                let Ok(value) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if value.get("method").is_none()
                    && let Some(id) = value.get("id").and_then(Value::as_u64)
                    && let Some(sender) = reader_pending
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .remove(&id)
                {
                    let _ = sender.send(value);
                }
            }
            reader_pending
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .clear();
            let _ = reader_child;
        });

        let outcome = match call_handshake_rpc(
            &stdin,
            &pending,
            1,
            &initialize_request(1),
            HANDSHAKE_TIMEOUT,
        ) {
            None => Err(handshake_failed_error(spec)),
            Some(init_response) => match parse_initialize_result(spec, &init_response) {
                Err(err) => Err(err),
                Ok(result) => {
                    if advertises_auth_status(&init_response) {
                        let auth_req = json!({
                            "jsonrpc": "2.0",
                            "id": 2,
                            "method": "auth/status",
                            "params": {},
                        });
                        if let Some(auth_resp) =
                            call_handshake_rpc(&stdin, &pending, 2, &auth_req, HANDSHAKE_TIMEOUT)
                        {
                            check_auth_status_response(spec, &auth_resp).map(|()| result)
                        } else {
                            Ok(result)
                        }
                    } else {
                        Ok(result)
                    }
                }
            },
        };

        // Always terminate and reap the handshake child immediately.
        stdin.lock().unwrap_or_else(|p| p.into_inner()).take();
        if let Some(mut isolated) = child.lock().unwrap_or_else(|p| p.into_inner()).take() {
            isolated.terminate();
        }
        outcome
    }

    fn start_session(
        &self,
        spec: &TutorLaunchSpec,
        sink: BackendEventSink,
    ) -> Result<Box<dyn TutorBackendSession>, DomainError> {
        let session = AcpStdioSession::spawn(spec, sink)?;
        Ok(Box::new(session))
    }
}

type PendingClientMap = Arc<Mutex<HashMap<u64, mpsc::Sender<Value>>>>;

struct PendingAgentRequest {
    rpc_id: Value,
    method: String,
    turn_id: String,
    allow_option_id: Option<String>,
    reject_option_id: Option<String>,
}

struct ActiveTurnState {
    turn_id: String,
    cancelled: bool,
}

struct AcpStdioSession {
    stdin: Arc<Mutex<Option<StdinWriter>>>,
    child: Arc<Mutex<Option<Process>>>,
    pending_client: PendingClientMap,
    pending_agent: Arc<Mutex<HashMap<String, PendingAgentRequest>>>,
    active_turn: Arc<Mutex<Option<ActiveTurnState>>>,
    shutting_down: Arc<AtomicBool>,
    next_rpc_id: AtomicU64,
    preset: AcpAgentPreset,
    command: String,
    session_id: String,
    agent_name: Option<String>,
    sink: BackendEventSink,
}

impl AcpStdioSession {
    fn spawn(spec: &TutorLaunchSpec, sink: BackendEventSink) -> Result<Self, DomainError> {
        let (program, args) = split_command_line(&spec.command)?;
        let (child, stdout, stdin) = spawn_acp_process(spec, &program, &args)?;

        let child = Arc::new(Mutex::new(Some(child)));
        let stdin = Arc::new(Mutex::new(Some(stdin)));
        let pending_client: PendingClientMap = Arc::new(Mutex::new(HashMap::new()));
        let pending_agent: Arc<Mutex<HashMap<String, PendingAgentRequest>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let active_turn: Arc<Mutex<Option<ActiveTurnState>>> = Arc::new(Mutex::new(None));
        let shutting_down = Arc::new(AtomicBool::new(false));
        let next_agent_req = Arc::new(AtomicU64::new(1));

        let router = AgentRequestRouter {
            stdin: Arc::clone(&stdin),
            pending_agent: Arc::clone(&pending_agent),
            active_turn: Arc::clone(&active_turn),
            next_agent_req: Arc::clone(&next_agent_req),
            sink: Arc::clone(&sink),
        };
        {
            let child = Arc::clone(&child);
            let pending_client = Arc::clone(&pending_client);
            let pending_agent = Arc::clone(&pending_agent);
            let active_turn = Arc::clone(&active_turn);
            let shutting_down = Arc::clone(&shutting_down);
            let sink = Arc::clone(&sink);
            thread::Builder::new()
                .name("codelab-acp-reader".into())
                .spawn(move || {
                    let mut reader = BufReader::new(stdout);
                    while let Some(line) = read_bounded_line(&mut reader, MAX_RPC_LINE_BYTES) {
                        let Ok(value) = serde_json::from_str::<Value>(&line) else {
                            continue;
                        };
                        let has_method = value.get("method").and_then(Value::as_str);
                        let has_id = value.get("id").filter(|id| !id.is_null());

                        match (has_method, has_id) {
                            (None, Some(id)) => {
                                if let Some(num_id) = id.as_u64()
                                    && let Some(sender) = pending_client
                                        .lock()
                                        .unwrap_or_else(|p| p.into_inner())
                                        .remove(&num_id)
                                {
                                    let _ = sender.send(value);
                                }
                            }
                            (Some(method), None) => {
                                if method == "session/update"
                                    && let Some(delta) = extract_update_delta(&value)
                                    && !delta.is_empty()
                                {
                                    let turn_id = active_turn
                                        .lock()
                                        .unwrap_or_else(|p| p.into_inner())
                                        .as_ref()
                                        .filter(|t| !t.cancelled)
                                        .map(|t| t.turn_id.clone());
                                    if let Some(turn_id) = turn_id {
                                        sink(BackendEvent::Chunk { turn_id, delta });
                                    }
                                }
                            }
                            (Some(method), Some(rpc_id)) => {
                                router.handle(
                                    method,
                                    rpc_id.clone(),
                                    value.get("params").unwrap_or(&Value::Null),
                                );
                            }
                            (None, None) => {}
                        }
                    }

                    // Stdout closed: reap process and unblock any waiting RPCs.
                    pending_client
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .clear();
                    pending_agent
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .clear();
                    let exit_code = child
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .as_mut()
                        .and_then(|process| process.wait_and_reap(EXIT_CODE_GRACE));
                    if !shutting_down.load(Ordering::Acquire) {
                        sink(BackendEvent::ProcessExited {
                            exit_code,
                            unexpected: true,
                        });
                    }
                })
                .map_err(|_| {
                    DomainError::new(ErrorCode::AcpAgentStartFailed)
                        .with_detail("preset", spec.preset.id())
                        .with_detail("command", spec.command.clone())
                })?;
        }

        let mut session = Self {
            stdin,
            child,
            pending_client,
            pending_agent,
            active_turn,
            shutting_down,
            next_rpc_id: AtomicU64::new(1),
            preset: spec.preset,
            command: spec.command.clone(),
            session_id: String::new(),
            agent_name: None,
            sink,
        };

        let init_response = session.call_rpc(
            "initialize",
            initialize_params(),
            HANDSHAKE_TIMEOUT,
            ErrorCode::AcpHandshakeFailed,
            spec,
        );
        let handshake = match init_response.and_then(|resp| parse_initialize_result(spec, &resp)) {
            Ok(info) => info,
            Err(err) => {
                session.shutdown();
                return Err(err);
            }
        };
        session.agent_name = handshake.agent_name;

        let cwd_str = spec.cwd.to_string_lossy().into_owned();
        let new_session_response = session.call_rpc(
            "session/new",
            json!({
                "cwd": cwd_str,
                "mcpServers": [],
            }),
            HANDSHAKE_TIMEOUT,
            ErrorCode::AcpHandshakeFailed,
            spec,
        );
        let session_id = match new_session_response.and_then(|resp| {
            if is_auth_required_error(&resp) {
                return Err(auth_required_error(spec.preset, &spec.command));
            }
            if resp.get("error").is_some() {
                return Err(handshake_failed_error(spec));
            }
            resp.get("result")
                .and_then(|r| r.get("sessionId").or_else(|| r.get("session_id")))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| handshake_failed_error(spec))
        }) {
            Ok(id) => id,
            Err(err) => {
                session.shutdown();
                return Err(err);
            }
        };
        session.session_id = session_id;
        Ok(session)
    }

    fn call_rpc(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
        error_code: ErrorCode,
        spec: &TutorLaunchSpec,
    ) -> Result<Value, DomainError> {
        let id = self.next_rpc_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        self.pending_client
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id, tx);
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        if write_json_line(&self.stdin, &req).is_err() {
            self.pending_client
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .remove(&id);
            return Err(DomainError::new(error_code)
                .with_detail("preset", spec.preset.id())
                .with_detail("command", spec.command.clone()));
        }
        rx.recv_timeout(timeout).map_err(|_| {
            self.pending_client
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .remove(&id);
            DomainError::new(error_code)
                .with_detail("preset", spec.preset.id())
                .with_detail("command", spec.command.clone())
        })
    }
}

impl TutorBackendSession for AcpStdioSession {
    fn agent_name(&self) -> Option<String> {
        self.agent_name.clone()
    }

    fn send_turn(&self, input: TutorTurnInput) -> Result<(), DomainError> {
        let rpc_id = self.next_rpc_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        self.pending_client
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(rpc_id, tx);
        *self.active_turn.lock().unwrap_or_else(|p| p.into_inner()) = Some(ActiveTurnState {
            turn_id: input.turn_id.clone(),
            cancelled: false,
        });

        let composed = compose_turn_prompt(&input);
        let req = json!({
            "jsonrpc": "2.0",
            "id": rpc_id,
            "method": "session/prompt",
            "params": {
                "sessionId": self.session_id,
                "prompt": [
                    {
                        "type": "text",
                        "text": composed,
                    }
                ]
            }
        });
        if write_json_line(&self.stdin, &req).is_err() {
            self.pending_client
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .remove(&rpc_id);
            *self.active_turn.lock().unwrap_or_else(|p| p.into_inner()) = None;
            return Err(DomainError::new(ErrorCode::AcpProcessExited));
        }

        let active_turn = Arc::clone(&self.active_turn);
        let shutting_down = Arc::clone(&self.shutting_down);
        let sink = Arc::clone(&self.sink);
        let preset = self.preset;
        let command = self.command.clone();
        let turn_id = input.turn_id;
        thread::Builder::new()
            .name("codelab-acp-turn".into())
            .spawn(move || {
                match rx.recv() {
                    Ok(response) => {
                        let was_cancelled = {
                            let mut guard = active_turn.lock().unwrap_or_else(|p| p.into_inner());
                            let cancelled = guard
                                .as_ref()
                                .is_some_and(|t| t.turn_id == turn_id && t.cancelled);
                            if guard.as_ref().is_some_and(|t| t.turn_id == turn_id) {
                                *guard = None;
                            }
                            cancelled
                        };
                        if was_cancelled {
                            sink(BackendEvent::TurnCancelled { turn_id });
                            return;
                        }
                        if response.get("error").is_some() {
                            let error = if is_auth_required_error(&response) {
                                auth_required_error(preset, &command)
                            } else {
                                DomainError::new(ErrorCode::AcpProtocolError)
                                    .with_detail("preset", preset.id())
                                    .with_detail("command", command)
                            };
                            sink(BackendEvent::TurnFailed { turn_id, error });
                            return;
                        }
                        let stop_reason = response
                            .get("result")
                            .and_then(|r| r.get("stopReason").or_else(|| r.get("stop_reason")))
                            .and_then(Value::as_str)
                            .unwrap_or("end_turn")
                            .to_owned();
                        if stop_reason == "cancelled" {
                            sink(BackendEvent::TurnCancelled { turn_id });
                        } else {
                            sink(BackendEvent::TurnCompleted {
                                turn_id,
                                stop_reason,
                            });
                        }
                    }
                    Err(_) => {
                        let mut guard = active_turn.lock().unwrap_or_else(|p| p.into_inner());
                        let was_cancelled = guard
                            .as_ref()
                            .is_some_and(|t| t.turn_id == turn_id && t.cancelled);
                        if guard.as_ref().is_some_and(|t| t.turn_id == turn_id) {
                            *guard = None;
                        }
                        if was_cancelled || shutting_down.load(Ordering::Acquire) {
                            // Already handled by cancel or intentional shutdown.
                        }
                    }
                }
            })
            .map_err(|_| DomainError::new(ErrorCode::AcpProtocolError))?;

        Ok(())
    }

    fn cancel_turn(&self, turn_id: &str) -> Result<(), DomainError> {
        {
            let mut guard = self.active_turn.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(active) = guard.as_mut()
                && active.turn_id == turn_id
            {
                active.cancelled = true;
            }
        }
        // Unblock any pending permission prompts for this turn with a cancelled outcome.
        let cancelled_reqs: Vec<PendingAgentRequest> = {
            let mut map = self.pending_agent.lock().unwrap_or_else(|p| p.into_inner());
            let keys: Vec<String> = map
                .iter()
                .filter(|(_, req)| req.turn_id == turn_id)
                .map(|(k, _)| k.clone())
                .collect();
            keys.into_iter().filter_map(|k| map.remove(&k)).collect()
        };
        for req in cancelled_reqs {
            let reply = if req.method == "session/request_permission" {
                json!({
                    "jsonrpc": "2.0",
                    "id": req.rpc_id,
                    "result": {
                        "outcome": { "outcome": "cancelled" }
                    }
                })
            } else {
                json!({
                    "jsonrpc": "2.0",
                    "id": req.rpc_id,
                    "error": { "code": -32800, "message": "Request cancelled" }
                })
            };
            let _ = write_json_line(&self.stdin, &reply);
        }

        let notification = json!({
            "jsonrpc": "2.0",
            "method": "session/cancel",
            "params": {
                "sessionId": self.session_id,
            }
        });
        let _ = write_json_line(&self.stdin, &notification);
        Ok(())
    }

    fn respond_permission(
        &self,
        request_id: &str,
        allowed: bool,
        fs_read_result: Option<Result<String, String>>,
        fs_write_result: Option<Result<(), String>>,
    ) -> Result<(), DomainError> {
        let Some(pending) = self
            .pending_agent
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(request_id)
        else {
            return Err(DomainError::new(ErrorCode::TutorPermissionNotFound));
        };

        let reply = match pending.method.as_str() {
            "session/request_permission" => {
                if allowed {
                    let option_id = pending
                        .allow_option_id
                        .unwrap_or_else(|| "allow_once".to_owned());
                    json!({
                        "jsonrpc": "2.0",
                        "id": pending.rpc_id,
                        "result": {
                            "outcome": {
                                "outcome": "selected",
                                "optionId": option_id,
                            }
                        }
                    })
                } else if let Some(reject_id) = pending.reject_option_id {
                    json!({
                        "jsonrpc": "2.0",
                        "id": pending.rpc_id,
                        "result": {
                            "outcome": {
                                "outcome": "selected",
                                "optionId": reject_id,
                            }
                        }
                    })
                } else {
                    json!({
                        "jsonrpc": "2.0",
                        "id": pending.rpc_id,
                        "result": {
                            "outcome": { "outcome": "cancelled" }
                        }
                    })
                }
            }
            "fs/read_text_file" => {
                if !allowed {
                    json!({
                        "jsonrpc": "2.0",
                        "id": pending.rpc_id,
                        "error": { "code": -32001, "message": "Permission denied" }
                    })
                } else {
                    match fs_read_result {
                        Some(Ok(content)) => json!({
                            "jsonrpc": "2.0",
                            "id": pending.rpc_id,
                            "result": { "content": content }
                        }),
                        Some(Err(msg)) => json!({
                            "jsonrpc": "2.0",
                            "id": pending.rpc_id,
                            "error": { "code": -32002, "message": msg }
                        }),
                        None => json!({
                            "jsonrpc": "2.0",
                            "id": pending.rpc_id,
                            "error": { "code": -32002, "message": "File read failed" }
                        }),
                    }
                }
            }
            "fs/write_text_file" => {
                if !allowed {
                    json!({
                        "jsonrpc": "2.0",
                        "id": pending.rpc_id,
                        "error": { "code": -32001, "message": "Permission denied" }
                    })
                } else {
                    match fs_write_result {
                        Some(Ok(())) => json!({
                            "jsonrpc": "2.0",
                            "id": pending.rpc_id,
                            "result": {}
                        }),
                        Some(Err(msg)) => json!({
                            "jsonrpc": "2.0",
                            "id": pending.rpc_id,
                            "error": { "code": -32002, "message": msg }
                        }),
                        None => json!({
                            "jsonrpc": "2.0",
                            "id": pending.rpc_id,
                            "error": { "code": -32002, "message": "File write failed" }
                        }),
                    }
                }
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": pending.rpc_id,
                "error": { "code": -32601, "message": "Method not found" }
            }),
        };

        write_json_line(&self.stdin, &reply)
            .map_err(|_| DomainError::new(ErrorCode::AcpProcessExited))
    }

    fn shutdown(&self) {
        self.shutting_down.store(true, Ordering::Release);
        self.stdin.lock().unwrap_or_else(|p| p.into_inner()).take();
        self.pending_client
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
        self.pending_agent
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
        if let Some(mut child) = self.child.lock().unwrap_or_else(|p| p.into_inner()).take() {
            child.terminate();
        }
    }
}

impl Drop for AcpStdioSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct AgentRequestRouter {
    stdin: Arc<Mutex<Option<StdinWriter>>>,
    pending_agent: Arc<Mutex<HashMap<String, PendingAgentRequest>>>,
    active_turn: Arc<Mutex<Option<ActiveTurnState>>>,
    next_agent_req: Arc<AtomicU64>,
    sink: BackendEventSink,
}

impl AgentRequestRouter {
    fn handle(&self, method: &str, rpc_id: Value, params: &Value) {
        let turn_id = self
            .active_turn
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .map(|t| t.turn_id.clone())
            .unwrap_or_else(|| "session".to_owned());

        let request_id = format!(
            "perm-{}",
            self.next_agent_req.fetch_add(1, Ordering::SeqCst)
        );

        match method {
            "session/request_permission" => {
                let parsed = parse_permission_params(params);
                self.pending_agent
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .insert(
                        request_id.clone(),
                        PendingAgentRequest {
                            rpc_id,
                            method: method.to_owned(),
                            turn_id: turn_id.clone(),
                            allow_option_id: parsed.allow_option_id,
                            reject_option_id: parsed.reject_option_id,
                        },
                    );
                (self.sink)(BackendEvent::PermissionRequested {
                    turn_id,
                    request: BackendPermissionPrompt {
                        request_id,
                        kind: parsed.kind,
                        title: parsed.title,
                        target_path: parsed.target_paths.first().cloned(),
                        target_paths: parsed.target_paths,
                        command: parsed.command,
                        raw_input: parsed.raw_input,
                        write_content: None,
                        is_fs_method: false,
                    },
                });
            }
            "fs/read_text_file" => {
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let title = path
                    .as_deref()
                    .map(|p| format!("Read {p}"))
                    .unwrap_or_else(|| "Read file".to_owned());
                self.pending_agent
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .insert(
                        request_id.clone(),
                        PendingAgentRequest {
                            rpc_id,
                            method: method.to_owned(),
                            turn_id: turn_id.clone(),
                            allow_option_id: None,
                            reject_option_id: None,
                        },
                    );
                let target_paths = path.iter().cloned().collect();
                (self.sink)(BackendEvent::PermissionRequested {
                    turn_id,
                    request: BackendPermissionPrompt {
                        request_id,
                        kind: TutorPermissionKind::Read,
                        title,
                        target_path: path,
                        target_paths,
                        command: None,
                        raw_input: Some(params.clone()),
                        write_content: None,
                        is_fs_method: true,
                    },
                });
            }
            "fs/write_text_file" => {
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let content = params
                    .get("content")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let title = path
                    .as_deref()
                    .map(|p| format!("Write {p}"))
                    .unwrap_or_else(|| "Write file".to_owned());
                self.pending_agent
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .insert(
                        request_id.clone(),
                        PendingAgentRequest {
                            rpc_id,
                            method: method.to_owned(),
                            turn_id: turn_id.clone(),
                            allow_option_id: None,
                            reject_option_id: None,
                        },
                    );
                let target_paths = path.iter().cloned().collect();
                (self.sink)(BackendEvent::PermissionRequested {
                    turn_id,
                    request: BackendPermissionPrompt {
                        request_id,
                        kind: TutorPermissionKind::Write,
                        title,
                        target_path: path,
                        target_paths,
                        command: None,
                        raw_input: Some(params.clone()),
                        write_content: content,
                        is_fs_method: true,
                    },
                });
            }
            _ => {
                let reply = json!({
                    "jsonrpc": "2.0",
                    "id": rpc_id,
                    "error": { "code": -32601, "message": "Method not found" }
                });
                let _ = write_json_line(&self.stdin, &reply);
            }
        }
    }
}

struct ParsedPermissionParams {
    kind: TutorPermissionKind,
    title: String,
    target_paths: Vec<String>,
    command: Option<String>,
    raw_input: Option<Value>,
    allow_option_id: Option<String>,
    reject_option_id: Option<String>,
}

fn parse_permission_params(params: &Value) -> ParsedPermissionParams {
    let tool_call = params.get("toolCall").or_else(|| params.get("tool_call"));
    let raw_input = tool_call
        .and_then(|tc| tc.get("rawInput").or_else(|| tc.get("raw_input")))
        .cloned()
        .or_else(|| params.get("rawInput").cloned());

    let raw_kind = tool_call
        .and_then(|tc| tc.get("kind"))
        .or_else(|| params.get("kind"))
        .or_else(|| params.get("operation"))
        .and_then(Value::as_str)
        .map(|s| s.to_ascii_lowercase());

    let mut target_paths = Vec::new();
    let mut push_path = |candidate: Option<&str>| {
        if let Some(path) = candidate.map(str::trim).filter(|s| !s.is_empty())
            && !target_paths.iter().any(|existing| existing == path)
        {
            target_paths.push(path.to_owned());
        }
    };
    if let Some(locations) = tool_call
        .and_then(|tc| tc.get("locations"))
        .and_then(Value::as_array)
    {
        for loc in locations {
            push_path(loc.get("path").and_then(Value::as_str));
        }
    }
    if let Some(inp) = raw_input.as_ref() {
        for key in ["path", "file_path", "filePath", "target", "destination"] {
            push_path(inp.get(key).and_then(Value::as_str));
        }
        if let Some(paths) = inp.get("paths").and_then(Value::as_array) {
            for item in paths {
                push_path(item.as_str());
            }
        }
    }
    push_path(params.get("path").and_then(Value::as_str));

    let command = raw_input
        .as_ref()
        .and_then(|inp| {
            inp.get("command")
                .or_else(|| inp.get("cmd"))
                .and_then(Value::as_str)
        })
        .or_else(|| params.get("command").and_then(Value::as_str))
        .map(str::to_owned);

    let kind = match raw_kind.as_deref() {
        Some("read" | "search") => TutorPermissionKind::Read,
        Some("edit" | "write" | "delete" | "move") => TutorPermissionKind::Write,
        Some("execute" | "terminal" | "command" | "shell") => TutorPermissionKind::Execute,
        _ => {
            if command.is_some() {
                TutorPermissionKind::Execute
            } else if raw_input
                .as_ref()
                .is_some_and(|inp| inp.get("content").is_some() || inp.get("edits").is_some())
            {
                TutorPermissionKind::Write
            } else if !target_paths.is_empty() && raw_kind.as_deref() == Some("read_file") {
                TutorPermissionKind::Read
            } else {
                // Unknown tool kinds must never be auto-allowed as read-only.
                TutorPermissionKind::Execute
            }
        }
    };

    let primary_path = target_paths.first().map(String::as_str);
    let title = tool_call
        .and_then(|tc| tc.get("title"))
        .or_else(|| params.get("title"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| match kind {
            TutorPermissionKind::Read => primary_path
                .map(|p| format!("Read {p}"))
                .unwrap_or_else(|| "Read file".to_owned()),
            TutorPermissionKind::Write => primary_path
                .map(|p| format!("Write {p}"))
                .unwrap_or_else(|| "Write file".to_owned()),
            TutorPermissionKind::Execute => command
                .as_deref()
                .map(|c| format!("Run {c}"))
                .unwrap_or_else(|| "Execute command".to_owned()),
        });

    let mut allow_once = None;
    let mut allow_fallback = None;
    let mut reject_once = None;
    let mut reject_fallback = None;
    if let Some(options) = params.get("options").and_then(Value::as_array) {
        for option in options {
            let Some(opt_id) = option
                .get("optionId")
                .or_else(|| option.get("option_id"))
                .or_else(|| option.get("id"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let opt_kind = option
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            if opt_kind == "allow_once" {
                allow_once.get_or_insert_with(|| opt_id.to_owned());
            } else if opt_kind.starts_with("allow") && opt_kind != "allow_always" {
                allow_fallback.get_or_insert_with(|| opt_id.to_owned());
            }
            if opt_kind == "reject_once" {
                reject_once.get_or_insert_with(|| opt_id.to_owned());
            } else if opt_kind.starts_with("reject") {
                reject_fallback.get_or_insert_with(|| opt_id.to_owned());
            }
        }
    }

    ParsedPermissionParams {
        kind,
        title,
        target_paths,
        command,
        raw_input,
        allow_option_id: allow_once.or(allow_fallback),
        reject_option_id: reject_once.or(reject_fallback),
    }
}

fn initialize_params() -> Value {
    json!({
        "protocolVersion": 1,
        "clientInfo": {
            "name": CLIENT_NAME,
            "version": CLIENT_VERSION,
        },
        "clientCapabilities": {
            "fs": {
                "readTextFile": true,
                "writeTextFile": true,
            },
            "terminal": false,
        }
    })
}

fn initialize_request(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": initialize_params(),
    })
}

fn extract_update_delta(notification: &Value) -> Option<String> {
    let params = notification.get("params")?;
    if let Some(update) = params.get("update") {
        let update_type = update
            .get("sessionUpdate")
            .or_else(|| update.get("type"))
            .and_then(Value::as_str);
        if let Some(kind) = update_type
            && kind != "agent_message_chunk"
            && kind != "message_chunk"
            && kind != "text"
        {
            return None;
        }
        if let Some(text) = update
            .get("content")
            .and_then(|c| c.get("text"))
            .and_then(Value::as_str)
        {
            return Some(text.to_owned());
        }
        if let Some(arr) = update.get("content").and_then(Value::as_array) {
            let joined: String = arr
                .iter()
                .filter_map(|item| item.get("text").and_then(Value::as_str))
                .collect();
            if !joined.is_empty() {
                return Some(joined);
            }
        }
        if let Some(text) = update.get("text").and_then(Value::as_str) {
            return Some(text.to_owned());
        }
    }
    params
        .get("delta")
        .or_else(|| params.get("text"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn call_handshake_rpc(
    stdin: &Mutex<Option<StdinWriter>>,
    pending: &PendingClientMap,
    id: u64,
    request: &Value,
    timeout: Duration,
) -> Option<Value> {
    let (tx, rx) = mpsc::channel();
    pending
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(id, tx);
    if write_json_line(stdin, request).is_err() {
        pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&id);
        return None;
    }
    match rx.recv_timeout(timeout) {
        Ok(value) => Some(value),
        Err(_) => {
            pending
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .remove(&id);
            None
        }
    }
}

fn is_auth_required_error(response: &Value) -> bool {
    response
        .get("error")
        .and_then(|err| err.get("code"))
        .and_then(Value::as_i64)
        == Some(ACP_AUTH_REQUIRED_CODE)
}

fn auth_required_error(preset: AcpAgentPreset, command: &str) -> DomainError {
    DomainError::new(ErrorCode::AcpAuthRequired)
        .with_detail("preset", preset.id())
        .with_detail("command", command)
}

fn handshake_failed_error(spec: &TutorLaunchSpec) -> DomainError {
    DomainError::new(ErrorCode::AcpHandshakeFailed)
        .with_detail("preset", spec.preset.id())
        .with_detail("command", spec.command.clone())
}

fn advertises_auth_status(init_response: &Value) -> bool {
    let Some(status) = init_response
        .get("result")
        .and_then(|r| r.get("agentCapabilities").or_else(|| r.get("capabilities")))
        .and_then(|caps| caps.get("auth"))
        .and_then(|auth| auth.get("status"))
    else {
        return false;
    };
    match status {
        Value::Bool(enabled) => *enabled,
        Value::Object(_) => true,
        _ => false,
    }
}

fn check_auth_status_response(spec: &TutorLaunchSpec, response: &Value) -> Result<(), DomainError> {
    if is_auth_required_error(response) {
        return Err(auth_required_error(spec.preset, &spec.command));
    }
    if response.get("error").is_some() {
        // Gracefully skip unsupported or non-auth errors (e.g. -32601 Method not found).
        return Ok(());
    }
    let Some(result) = response.get("result") else {
        return Ok(());
    };
    if result.get("authenticated").and_then(Value::as_bool) == Some(false)
        || result
            .get("loggedIn")
            .or_else(|| result.get("logged_in"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return Err(auth_required_error(spec.preset, &spec.command));
    }
    if result
        .get("authRequired")
        .or_else(|| result.get("auth_required"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        return Err(auth_required_error(spec.preset, &spec.command));
    }
    if let Some(status) = result
        .get("status")
        .or_else(|| result.get("state"))
        .and_then(Value::as_str)
    {
        let normalized = status.trim().to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "unauthenticated"
                | "auth_required"
                | "authrequired"
                | "not_authenticated"
                | "logged_out"
                | "loggedout"
                | "required"
        ) {
            return Err(auth_required_error(spec.preset, &spec.command));
        }
    }
    Ok(())
}

fn parse_initialize_result(
    spec: &TutorLaunchSpec,
    response: &Value,
) -> Result<TutorHandshakeResult, DomainError> {
    if is_auth_required_error(response) {
        return Err(auth_required_error(spec.preset, &spec.command));
    }
    if response.get("error").is_some() {
        return Err(handshake_failed_error(spec));
    }
    let Some(result) = response.get("result") else {
        return Err(handshake_failed_error(spec));
    };
    let protocol_version = result.get("protocolVersion").map(|v| {
        v.as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| v.to_string())
    });
    let agent_info = result.get("agentInfo").or_else(|| result.get("serverInfo"));
    let agent_name = agent_info
        .and_then(|info| info.get("name"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let agent_version = agent_info
        .and_then(|info| info.get("version"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    Ok(TutorHandshakeResult {
        preset: spec.preset,
        command: spec.command.clone(),
        protocol_version,
        agent_name,
        agent_version,
    })
}

fn write_json_line(stdin: &Mutex<Option<StdinWriter>>, payload: &Value) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(payload)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    bytes.push(b'\n');
    let mut guard = stdin.lock().unwrap_or_else(|p| p.into_inner());
    let writer = guard
        .as_mut()
        .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?;
    writer.write_all(&bytes)?;
    writer.flush()
}

fn read_bounded_line<R: BufRead>(reader: &mut R, cap: usize) -> Option<String> {
    let mut buf = Vec::new();
    let mut limited = reader.take(cap as u64);
    match limited.read_until(b'\n', &mut buf) {
        Ok(0) => None,
        Ok(_) => {
            if !buf.ends_with(b"\n") {
                // Drain the oversized remainder in bounded buffer chunks without allocating.
                loop {
                    let available = match reader.fill_buf() {
                        Ok([]) | Err(_) => break,
                        Ok(slice) => slice,
                    };
                    if let Some(pos) = available.iter().position(|&b| b == b'\n') {
                        reader.consume(pos + 1);
                        break;
                    }
                    let len = available.len();
                    reader.consume(len);
                }
                return Some(String::new());
            }
            let text = String::from_utf8_lossy(&buf);
            Some(text.trim_end_matches(['\r', '\n']).to_owned())
        }
        Err(_) => None,
    }
}

pub(crate) fn split_command_line(input: &str) -> Result<(String, Vec<String>), DomainError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(DomainError::new(ErrorCode::AcpCommandInvalid));
    }
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut quote: Option<char> = None;
    let mut chars = trimmed.chars().peekable();

    while let Some(ch) = chars.next() {
        match quote {
            None => match ch {
                c if c.is_whitespace() => {
                    if in_token {
                        tokens.push(std::mem::take(&mut current));
                        in_token = false;
                    }
                }
                '\'' | '"' => {
                    quote = Some(ch);
                    in_token = true;
                }
                '\\' => {
                    if let Some(&next) = chars.peek()
                        && (next == '"' || next == '\'' || next == '\\' || next.is_whitespace())
                    {
                        chars.next();
                        current.push(next);
                    } else {
                        current.push('\\');
                    }
                    in_token = true;
                }
                other => {
                    current.push(other);
                    in_token = true;
                }
            },
            Some('\'') => {
                if ch == '\'' {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            Some('"') => match ch {
                '"' => quote = None,
                '\\' => {
                    if let Some(&next) = chars.peek()
                        && (next == '"' || next == '\\')
                    {
                        chars.next();
                        current.push(next);
                    } else {
                        current.push('\\');
                    }
                }
                other => current.push(other),
            },
            Some(_) => unreachable!(),
        }
    }

    if quote.is_some() {
        return Err(DomainError::new(ErrorCode::AcpCommandInvalid));
    }
    if in_token {
        tokens.push(current);
    }
    let Some((program, args)) = tokens.split_first() else {
        return Err(DomainError::new(ErrorCode::AcpCommandInvalid));
    };
    if program.trim().is_empty() {
        return Err(DomainError::new(ErrorCode::AcpCommandInvalid));
    }
    Ok((program.clone(), args.to_vec()))
}

fn spawn_acp_process(
    spec: &TutorLaunchSpec,
    program: &str,
    args: &[String],
) -> Result<(Process, Box<dyn Read + Send>, StdinWriter), DomainError> {
    let os_args: Vec<OsString> = args.iter().map(OsString::from).collect();
    let canonical_cwd = fs::canonicalize(&spec.cwd).map_err(|_| {
        DomainError::new(ErrorCode::AcpAgentStartFailed)
            .with_detail("preset", spec.preset.id())
            .with_detail("command", program)
    })?;
    let handle = open_canonical_dir(&canonical_cwd).map_err(|_| {
        DomainError::new(ErrorCode::AcpAgentStartFailed)
            .with_detail("preset", spec.preset.id())
            .with_detail("command", program)
    })?;
    let directory = Directory {
        path: canonical_cwd,
        handle,
    };
    spawn_piped(PathBuf::from(program), os_args, directory)
        .map(|(process, (stdout, stdin))| (process, stdout, stdin))
        .map_err(|err| {
            let code = if err.kind() == io::ErrorKind::NotFound
                && !program.ends_with(std::path::is_separator)
            {
                ErrorCode::AcpAdapterNotFound
            } else {
                ErrorCode::AcpAgentStartFailed
            };
            DomainError::new(code)
                .with_detail("preset", spec.preset.id())
                .with_detail("command", program)
        })
}

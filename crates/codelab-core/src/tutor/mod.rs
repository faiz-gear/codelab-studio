pub mod acp;
pub mod backend;
pub(crate) mod context;
pub mod model;
pub mod policy;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use crate::error::{DomainError, ErrorCode};
use crate::storage::JsonStore;
use crate::workspace::{
    MAX_WORKSPACE_TEXT_BYTES, WorkspaceFileContents, WorkspaceFilesChanged, WorkspacePaths,
};

pub use self::acp::AcpStdioBackend;
pub use self::backend::{
    BackendEvent, BackendEventSink, BackendPermissionPrompt, TutorBackend, TutorBackendSession,
    TutorLaunchSpec, TutorTurnInput,
};
pub use self::model::{
    AcpAgentPreset, ContextChip, ContextDimension, ContextQuote, ContextQuoteSource,
    ContextSnapshot, ContextSnapshotInput, MAX_CONTEXT_INPUT_TOKENS, PermissionPolicy,
    RespondTutorPermission, SendTutorPrompt, TeachingStyle, TutorFileChange, TutorHandshakeRequest,
    TutorHandshakeResult, TutorHistory, TutorMessage, TutorMessageStatus, TutorPermissionKind,
    TutorPermissionRequest, TutorPermissionStatus, TutorRole, TutorSessionSnapshot,
    TutorSessionStatus, TutorSettings, TutorSettingsPatch, WorkspaceTutorRequest,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct ResolvedTurnContext {
    pub preamble: Option<String>,
    pub quote: Option<ContextQuote>,
    pub dimensions: Vec<ContextDimension>,
}
use self::policy::{PermissionEvaluation, PolicyDecision, evaluate_permission};

pub(crate) struct TutorWorkspaceAttachment {
    pub workspace_id: u64,
    pub root: PathBuf,
    pub files: Arc<WorkspacePaths>,
    pub last_saved_at: Arc<Mutex<DateTime<Utc>>>,
    pub history_store: JsonStore<TutorHistory>,
    pub history: TutorHistory,
}

struct PendingPermissionContext {
    turn_id: String,
    prompt: BackendPermissionPrompt,
    eval: PermissionEvaluation,
}

struct TutorRuntimeState {
    settings: TutorSettings,
    workspace: Option<TutorWorkspaceAttachment>,
    session: Option<Arc<dyn TutorBackendSession>>,
    session_generation: u64,
    session_turns_completed: u64,
    agent_name: Option<String>,
    status: TutorSessionStatus,
    active_turn_id: Option<String>,
    last_error: Option<DomainError>,
    pending_permissions: HashMap<String, PendingPermissionContext>,
    approved_grants: HashSet<(TutorPermissionKind, String)>,
    watched_writes: Vec<(String, Option<String>)>,
    last_turn_context: Option<ResolvedTurnContext>,
    last_context_input: Option<ContextSnapshotInput>,
    next_seq: u64,
}

pub(crate) struct TutorManager {
    backend: Arc<dyn TutorBackend>,
    on_session_updated: Arc<dyn Fn(TutorSessionSnapshot) + Send + Sync>,
    on_workspace_files_changed: Arc<dyn Fn(WorkspaceFilesChanged) + Send + Sync>,
    state: Arc<Mutex<TutorRuntimeState>>,
}

impl TutorManager {
    pub(crate) fn new(
        backend: Arc<dyn TutorBackend>,
        initial_settings: TutorSettings,
        on_session_updated: impl Fn(TutorSessionSnapshot) + Send + Sync + 'static,
        on_workspace_files_changed: impl Fn(WorkspaceFilesChanged) + Send + Sync + 'static,
    ) -> Self {
        Self {
            backend,
            on_session_updated: Arc::new(on_session_updated),
            on_workspace_files_changed: Arc::new(on_workspace_files_changed),
            state: Arc::new(Mutex::new(TutorRuntimeState {
                settings: initial_settings,
                workspace: None,
                session: None,
                session_generation: 0,
                session_turns_completed: 0,
                agent_name: None,
                status: TutorSessionStatus::Idle,
                active_turn_id: None,
                last_error: None,
                pending_permissions: HashMap::new(),
                approved_grants: HashSet::new(),
                watched_writes: Vec::new(),
                last_turn_context: None,
                last_context_input: None,
                next_seq: 1,
            })),
        }
    }

    pub(crate) fn handshake(
        &self,
        request: TutorHandshakeRequest,
        cwd: PathBuf,
    ) -> Result<TutorHandshakeResult, DomainError> {
        let resolved =
            if request.command.trim().is_empty() && request.preset != AcpAgentPreset::Custom {
                request.preset.default_command().to_owned()
            } else {
                request.command.trim().to_owned()
            };
        if resolved.is_empty() {
            return Err(DomainError::new(ErrorCode::AcpCommandInvalid));
        }
        let spec = TutorLaunchSpec {
            preset: request.preset,
            command: resolved,
            cwd,
        };
        self.backend.handshake(&spec)
    }

    pub(crate) fn attach_workspace(&self, attachment: TutorWorkspaceAttachment) {
        let (old_session, snapshot) = {
            let mut state = self.lock();
            let old = state.teardown_session_inner();
            state.workspace = Some(attachment);
            state.status = TutorSessionStatus::Idle;
            state.active_turn_id = None;
            state.last_error = None;
            state.last_turn_context = None;
            state.last_context_input = None;
            let snapshot = state.snapshot_opt();
            (old, snapshot)
        };
        if let Some(session) = old_session {
            session.shutdown();
        }
        if let Some(snapshot) = snapshot {
            (self.on_session_updated)(snapshot);
        }
    }

    pub(crate) fn detach_workspace(&self) {
        let old_session = {
            let mut state = self.lock();
            state.cancel_active_turn_in_history();
            let old = state.teardown_session_inner();
            state.workspace = None;
            state.status = TutorSessionStatus::Idle;
            state.active_turn_id = None;
            state.last_error = None;
            state.last_turn_context = None;
            state.last_context_input = None;
            old
        };
        if let Some(session) = old_session {
            session.shutdown();
        }
    }

    pub(crate) fn stop_process(&self) {
        let (old_session, snapshot) = {
            let mut state = self.lock();
            state.cancel_active_turn_in_history();
            let old = state.teardown_session_inner();
            state.active_turn_id = None;
            state.status = TutorSessionStatus::Idle;
            (old, state.snapshot_opt())
        };
        if let Some(session) = old_session {
            session.shutdown();
        }
        if let Some(snapshot) = snapshot {
            (self.on_session_updated)(snapshot);
        }
    }

    pub(crate) fn sync_settings(&self, next: TutorSettings) {
        let (old_session, snapshot) = {
            let mut state = self.lock();
            let agent_changed = state.settings.preset != next.preset
                || state.settings.resolved_command() != next.resolved_command();
            state.settings = next;
            let old = if agent_changed {
                state.cancel_active_turn_in_history();
                let old = state.teardown_session_inner();
                state.status = TutorSessionStatus::Idle;
                state.active_turn_id = None;
                state.last_error = None;
                old
            } else {
                None
            };
            (old, state.snapshot_opt())
        };
        if let Some(session) = old_session {
            session.shutdown();
        }
        if let Some(snapshot) = snapshot {
            (self.on_session_updated)(snapshot);
        }
    }

    pub(crate) fn snapshot(&self, workspace_id: u64) -> Result<TutorSessionSnapshot, DomainError> {
        let state = self.lock();
        state.require_workspace(workspace_id)?;
        Ok(state.snapshot_opt().expect("attached workspace"))
    }

    pub(crate) fn send_prompt(
        self: &Arc<Self>,
        request: SendTutorPrompt,
        resolved_context: Option<ResolvedTurnContext>,
    ) -> Result<TutorSessionSnapshot, DomainError> {
        let prompt_text = request.prompt.trim().to_owned();
        if prompt_text.is_empty() {
            return Err(DomainError::new(ErrorCode::TutorPromptEmpty));
        }
        let turn_context = resolved_context.unwrap_or(ResolvedTurnContext {
            preamble: request.context_preamble.clone(),
            quote: None,
            dimensions: Vec::new(),
        });

        let (needs_spawn, launch_spec, existing_session, turn_input, generation, initial_snapshot) = {
            let mut state = self.lock();
            state.require_workspace(request.workspace_id)?;
            if state.active_turn_id.is_some() {
                return Err(DomainError::new(ErrorCode::TutorBusy));
            }
            if let Some(style) = request.teaching_style {
                state.settings.teaching_style = style;
            }
            let command = state.settings.resolved_command();
            if command.is_empty() {
                return Err(DomainError::new(ErrorCode::AcpCommandInvalid));
            }

            let turn_id = state.next_id("turn");
            let user_msg_id = state.next_id("msg");
            let assistant_msg_id = state.next_id("msg");
            let now = Utc::now().to_rfc3339();
            let style = state.settings.teaching_style;

            let cwd = {
                let ws = state.workspace.as_mut().expect("checked workspace");
                ws.history.messages.push(TutorMessage {
                    id: user_msg_id,
                    turn_id: turn_id.clone(),
                    role: TutorRole::User,
                    content: prompt_text.clone(),
                    teaching_style: style,
                    status: TutorMessageStatus::Completed,
                    error: None,
                    quote: turn_context.quote.clone(),
                    context_dimensions: turn_context.dimensions.clone(),
                    permissions: Vec::new(),
                    file_changes: Vec::new(),
                    created_at: now.clone(),
                });
                ws.history.messages.push(TutorMessage {
                    id: assistant_msg_id,
                    turn_id: turn_id.clone(),
                    role: TutorRole::Assistant,
                    content: String::new(),
                    teaching_style: style,
                    status: TutorMessageStatus::Streaming,
                    error: None,
                    quote: None,
                    context_dimensions: Vec::new(),
                    permissions: Vec::new(),
                    file_changes: Vec::new(),
                    created_at: now,
                });
                ws.history_store.save_atomic(&ws.history)?;
                ws.root.clone()
            };

            state.active_turn_id = Some(turn_id.clone());
            state.last_error = None;
            state.approved_grants.clear();
            state.watched_writes.clear();
            state.last_turn_context = Some(turn_context.clone());
            state.last_context_input = request.context.clone();

            let existing = state.session.clone();
            let needs_spawn = existing.is_none();
            state.status = if needs_spawn {
                TutorSessionStatus::Starting
            } else {
                TutorSessionStatus::Streaming
            };
            let launch_spec = TutorLaunchSpec {
                preset: state.settings.preset,
                command,
                cwd,
            };
            let turn_input = TutorTurnInput {
                turn_id,
                prompt: prompt_text,
                teaching_style: style,
                context_preamble: turn_context.preamble,
            };
            let generation = state.session_generation;
            let snapshot = state.snapshot_opt().expect("attached workspace");
            (
                needs_spawn,
                launch_spec,
                existing,
                turn_input,
                generation,
                snapshot,
            )
        };

        (self.on_session_updated)(initial_snapshot);

        let session = if needs_spawn {
            let manager = Arc::downgrade(self);
            let sink: BackendEventSink = Arc::new(move |event| {
                if let Some(manager) = manager.upgrade() {
                    manager.handle_backend_event(generation, event);
                }
            });
            match self.backend.start_session(&launch_spec, sink) {
                Ok(new_session) => {
                    let arc_session: Arc<dyn TutorBackendSession> = Arc::from(new_session);
                    let agent_name = arc_session.agent_name();
                    let still_valid = {
                        let mut state = self.lock();
                        if state.session_generation != generation
                            || state.active_turn_id.as_deref() != Some(turn_input.turn_id.as_str())
                        {
                            false
                        } else {
                            state.session = Some(Arc::clone(&arc_session));
                            state.agent_name = agent_name;
                            state.status = TutorSessionStatus::Streaming;
                            true
                        }
                    };
                    if !still_valid {
                        arc_session.shutdown();
                        return self.snapshot(request.workspace_id);
                    }
                    arc_session
                }
                Err(error) => {
                    let failed_snapshot = {
                        let mut state = self.lock();
                        state.fail_turn(&turn_input.turn_id, error.clone());
                        state.snapshot_opt()
                    };
                    if let Some(snapshot) = failed_snapshot {
                        (self.on_session_updated)(snapshot);
                    }
                    return Err(error);
                }
            }
        } else {
            existing_session.expect("checked existing session")
        };

        if let Err(error) = session.send_turn(turn_input.clone()) {
            let (old, failed_snapshot) = {
                let mut state = self.lock();
                let old = state.teardown_session_inner();
                state.fail_turn(&turn_input.turn_id, error.clone());
                (old, state.snapshot_opt())
            };
            if let Some(old) = old {
                old.shutdown();
            }
            if let Some(snapshot) = failed_snapshot {
                (self.on_session_updated)(snapshot);
            }
            return Err(error);
        }

        self.snapshot(request.workspace_id)
    }

    pub(crate) fn stop_turn(&self, workspace_id: u64) -> Result<TutorSessionSnapshot, DomainError> {
        let (session_to_cancel, turn_id, snapshot) = {
            let mut state = self.lock();
            state.require_workspace(workspace_id)?;
            let Some(turn_id) = state.active_turn_id.clone() else {
                return Ok(state.snapshot_opt().expect("attached workspace"));
            };
            state.cancel_active_turn_in_history();
            state.status = if state.session.is_some() {
                TutorSessionStatus::Ready
            } else {
                TutorSessionStatus::Idle
            };
            let session = state.session.clone();
            let snapshot = state.snapshot_opt().expect("attached workspace");
            (session, turn_id, snapshot)
        };

        if let Some(session) = session_to_cancel {
            let _ = session.cancel_turn(&turn_id);
        }
        (self.on_session_updated)(snapshot.clone());
        Ok(snapshot)
    }

    pub(crate) fn retry_turn_seed(
        &self,
        workspace_id: u64,
    ) -> Result<(String, TeachingStyle, Option<ContextSnapshotInput>), DomainError> {
        let state = self.lock();
        state.require_workspace(workspace_id)?;
        if state.active_turn_id.is_some() {
            return Err(DomainError::new(ErrorCode::TutorBusy));
        }
        let ws = state.workspace.as_ref().expect("checked workspace");
        let last_user = ws
            .history
            .messages
            .iter()
            .rev()
            .find(|m| m.role == TutorRole::User)
            .ok_or_else(|| DomainError::new(ErrorCode::TutorPromptEmpty))?;
        let context_input = state.last_context_input.clone().or_else(|| {
            last_user.quote.as_ref().map(|quote| ContextSnapshotInput {
                workspace_id,
                quote: Some(quote.clone()),
                ..Default::default()
            })
        });
        Ok((
            last_user.content.clone(),
            state.settings.teaching_style,
            context_input,
        ))
    }

    pub(crate) fn retry_turn(
        self: &Arc<Self>,
        workspace_id: u64,
        refreshed_context: Option<(ContextSnapshotInput, ResolvedTurnContext)>,
    ) -> Result<TutorSessionSnapshot, DomainError> {
        let (prompt, teaching_style, fallback_context) = {
            let state = self.lock();
            state.require_workspace(workspace_id)?;
            if state.active_turn_id.is_some() {
                return Err(DomainError::new(ErrorCode::TutorBusy));
            }
            let ws = state.workspace.as_ref().expect("checked workspace");
            let last_user = ws
                .history
                .messages
                .iter()
                .rev()
                .find(|m| m.role == TutorRole::User)
                .ok_or_else(|| DomainError::new(ErrorCode::TutorPromptEmpty))?;
            (
                last_user.content.clone(),
                state.settings.teaching_style,
                state.last_turn_context.clone().unwrap_or_default(),
            )
        };

        let (context_input, resolved_context) = match refreshed_context {
            Some((input, resolved)) => (Some(input), resolved),
            None => (None, fallback_context),
        };

        self.send_prompt(
            SendTutorPrompt {
                workspace_id,
                prompt,
                teaching_style: Some(teaching_style),
                context_preamble: resolved_context.preamble.clone(),
                context: context_input,
            },
            Some(resolved_context),
        )
    }

    pub(crate) fn respond_permission(
        &self,
        request: RespondTutorPermission,
    ) -> Result<TutorSessionSnapshot, DomainError> {
        let (
            session,
            ctx,
            ws_files,
            ws_saved_at,
            workspace_id,
            updated_snapshot,
            fs_read_result,
            fs_write_result,
            changed_paths,
        ) = {
            let mut state = self.lock();
            state.require_workspace(request.workspace_id)?;
            let Some(ctx) = state.pending_permissions.remove(&request.request_id) else {
                return Err(DomainError::new(ErrorCode::TutorPermissionNotFound));
            };
            let Some(session) = state.session.clone() else {
                return Err(DomainError::new(ErrorCode::AcpProcessExited));
            };
            let (ws_files, ws_saved_at, workspace_id) = {
                let ws = state.workspace.as_ref().expect("checked workspace");
                (
                    Arc::clone(&ws.files),
                    Arc::clone(&ws.last_saved_at),
                    ws.workspace_id,
                )
            };

            let mut fs_read_result = None;
            let mut fs_write_result = None;
            let mut changed_paths = Vec::new();
            let mut recorded_change = None;
            let now = Utc::now().to_rfc3339();

            if request.allow {
                if ctx.prompt.is_fs_method {
                    match ctx.prompt.kind {
                        TutorPermissionKind::Read => {
                            fs_read_result = Some(execute_approved_read(&ws_files, &ctx.eval));
                        }
                        TutorPermissionKind::Write => {
                            let content = ctx.prompt.write_content.as_deref().unwrap_or("");
                            let (write_res, file_change) = execute_approved_write(
                                &ws_files,
                                &ws_saved_at,
                                &ctx.eval,
                                content,
                                &now,
                            );
                            if let Some(change) = file_change {
                                changed_paths.push(change.path.clone());
                                recorded_change = Some(change);
                            }
                            fs_write_result = Some(write_res);
                        }
                        TutorPermissionKind::Execute => {}
                    }
                } else {
                    if let Some(key) = grant_key(&ctx.eval, ctx.prompt.target_path.as_deref()) {
                        state.approved_grants.insert((ctx.prompt.kind, key));
                    }
                    if ctx.prompt.kind == TutorPermissionKind::Write
                        && let Some(rel) = ctx.eval.workspace_relative_path.as_deref()
                    {
                        let before = read_workspace_text(&ws_files, rel);
                        if !state.watched_writes.iter().any(|(p, _)| p == rel) {
                            state.watched_writes.push((rel.to_owned(), before));
                        }
                    }
                }
            }

            let new_status = if request.allow {
                TutorPermissionStatus::Approved
            } else {
                TutorPermissionStatus::Denied
            };
            {
                let ws = state.workspace.as_mut().expect("checked workspace");
                if let Some(change) = recorded_change {
                    record_file_change(ws, &ctx.turn_id, change);
                }
                update_permission_status(ws, &ctx.turn_id, &request.request_id, new_status, &now);
                let _ = ws.history_store.save_atomic(&ws.history);
            }

            if state.active_turn_id.is_some() {
                state.status = if state.pending_permissions.is_empty() {
                    TutorSessionStatus::Streaming
                } else {
                    TutorSessionStatus::AwaitingPermission
                };
            }
            let snapshot = state.snapshot_opt().expect("attached workspace");
            (
                session,
                ctx,
                ws_files,
                ws_saved_at,
                workspace_id,
                snapshot,
                fs_read_result,
                fs_write_result,
                changed_paths,
            )
        };

        let _ = (ws_files, ws_saved_at);
        if !changed_paths.is_empty() {
            (self.on_workspace_files_changed)(WorkspaceFilesChanged {
                workspace_id,
                paths: changed_paths,
            });
        }
        (self.on_session_updated)(updated_snapshot.clone());
        session.respond_permission(
            &ctx.prompt.request_id,
            request.allow,
            fs_read_result,
            fs_write_result,
        )?;
        Ok(updated_snapshot)
    }

    pub(crate) fn clear_session(
        &self,
        workspace_id: u64,
    ) -> Result<TutorSessionSnapshot, DomainError> {
        let (old_session, snapshot) = {
            let mut state = self.lock();
            state.require_workspace(workspace_id)?;
            let old = state.teardown_session_inner();
            let ws = state.workspace.as_mut().expect("checked workspace");
            ws.history.messages.clear();
            ws.history_store.save_atomic(&ws.history)?;
            state.status = TutorSessionStatus::Idle;
            state.active_turn_id = None;
            state.last_error = None;
            state.last_turn_context = None;
            state.last_context_input = None;
            let snapshot = state.snapshot_opt().expect("attached workspace");
            (old, snapshot)
        };
        if let Some(session) = old_session {
            session.shutdown();
        }
        (self.on_session_updated)(snapshot.clone());
        Ok(snapshot)
    }

    pub(crate) fn shutdown(&self) {
        self.detach_workspace();
    }

    fn handle_backend_event(&self, generation: u64, event: BackendEvent) {
        match event {
            BackendEvent::Chunk { turn_id, delta } => {
                let snapshot = {
                    let mut state = self.lock();
                    if state.session_generation != generation
                        || state.active_turn_id.as_deref() != Some(turn_id.as_str())
                    {
                        return;
                    }
                    if let Some(ws) = state.workspace.as_mut()
                        && let Some(msg) = ws.history.find_assistant_mut(&turn_id)
                    {
                        msg.content.push_str(&delta);
                    }
                    if state.pending_permissions.is_empty() {
                        state.status = TutorSessionStatus::Streaming;
                    }
                    state.snapshot_opt()
                };
                if let Some(snapshot) = snapshot {
                    (self.on_session_updated)(snapshot);
                }
            }
            BackendEvent::PermissionRequested { turn_id, request } => {
                self.handle_permission_requested(generation, turn_id, request);
            }
            BackendEvent::TurnCompleted { turn_id, .. } => {
                let (changed, snapshot) = {
                    let mut state = self.lock();
                    if state.session_generation != generation
                        || state.active_turn_id.as_deref() != Some(turn_id.as_str())
                    {
                        return;
                    }
                    let changed = state.collect_watched_writes(&turn_id);
                    state.active_turn_id = None;
                    state.pending_permissions.clear();
                    state.approved_grants.clear();
                    state.session_turns_completed = state.session_turns_completed.saturating_add(1);
                    state.status = TutorSessionStatus::Ready;
                    if let Some(ws) = state.workspace.as_mut() {
                        if let Some(msg) = ws.history.find_assistant_mut(&turn_id) {
                            msg.status = TutorMessageStatus::Completed;
                        }
                        let _ = ws.history_store.save_atomic(&ws.history);
                    }
                    (changed, state.snapshot_opt())
                };
                if let Some(event) = changed {
                    (self.on_workspace_files_changed)(event);
                }
                if let Some(snapshot) = snapshot {
                    (self.on_session_updated)(snapshot);
                }
            }
            BackendEvent::TurnCancelled { turn_id } => {
                let (changed, snapshot) = {
                    let mut state = self.lock();
                    if state.session_generation != generation {
                        return;
                    }
                    let changed = state.collect_watched_writes(&turn_id);
                    if state.active_turn_id.as_deref() == Some(turn_id.as_str()) {
                        state.active_turn_id = None;
                        state.pending_permissions.clear();
                        state.approved_grants.clear();
                        state.status = TutorSessionStatus::Ready;
                    }
                    if let Some(ws) = state.workspace.as_mut() {
                        if let Some(msg) = ws.history.find_assistant_mut(&turn_id)
                            && msg.status == TutorMessageStatus::Streaming
                        {
                            msg.status = TutorMessageStatus::Cancelled;
                        }
                        let _ = ws.history_store.save_atomic(&ws.history);
                    }
                    (changed, state.snapshot_opt())
                };
                if let Some(event) = changed {
                    (self.on_workspace_files_changed)(event);
                }
                if let Some(snapshot) = snapshot {
                    (self.on_session_updated)(snapshot);
                }
            }
            BackendEvent::TurnFailed { turn_id, code } => {
                let snapshot = {
                    let mut state = self.lock();
                    if state.session_generation != generation {
                        return;
                    }
                    state.fail_turn(&turn_id, DomainError::new(code));
                    state.snapshot_opt()
                };
                if let Some(snapshot) = snapshot {
                    (self.on_session_updated)(snapshot);
                }
            }
            BackendEvent::ProcessExited {
                exit_code,
                unexpected,
            } => {
                let snapshot = {
                    let mut state = self.lock();
                    if state.session_generation != generation {
                        return;
                    }
                    state.session = None;
                    state.session_turns_completed = 0;
                    state.pending_permissions.clear();
                    state.approved_grants.clear();
                    state.watched_writes.clear();
                    if unexpected {
                        let mut err = DomainError::new(ErrorCode::AcpProcessExited);
                        if let Some(code) = exit_code {
                            err = err.with_detail("code", code.to_string());
                        }
                        if let Some(active_turn) = state.active_turn_id.clone() {
                            state.fail_turn(&active_turn, err);
                        } else {
                            state.last_error = Some(err);
                            state.status = TutorSessionStatus::Error;
                        }
                    } else {
                        state.status = TutorSessionStatus::Idle;
                    }
                    state.snapshot_opt()
                };
                if let Some(snapshot) = snapshot {
                    (self.on_session_updated)(snapshot);
                }
            }
        }
    }

    fn handle_permission_requested(
        &self,
        generation: u64,
        turn_id: String,
        request: BackendPermissionPrompt,
    ) {
        enum ImmediateAction {
            Respond {
                session: Arc<dyn TutorBackendSession>,
                request_id: String,
                allowed: bool,
                fs_read: Option<Result<String, String>>,
                fs_write: Option<Result<(), String>>,
            },
            WaitForUser,
        }

        let (action, changed_event, snapshot) = {
            let mut state = self.lock();
            if state.session_generation != generation {
                return;
            }
            let Some(session) = state.session.clone() else {
                return;
            };
            let policy = state.settings.permission_policy;
            let Some(ws_files) = state.workspace.as_ref().map(|ws| Arc::clone(&ws.files)) else {
                return;
            };
            let target_paths: Vec<String> = if !request.target_paths.is_empty() {
                request.target_paths.clone()
            } else {
                request.target_path.iter().cloned().collect()
            };
            let eval = evaluate_permission(
                &ws_files,
                policy,
                request.kind,
                &target_paths,
                request.command.as_deref(),
            );
            let now = Utc::now().to_rfc3339();

            // Check if a matching one-shot approval grant was already granted in this turn
            // (e.g. session/request_permission followed by fs/write_text_file).
            let grant = grant_key(&eval, request.target_path.as_deref());
            let has_prior_grant = request.is_fs_method
                && eval.decision != PolicyDecision::BlockSensitive
                && grant
                    .as_ref()
                    .is_some_and(|key| state.approved_grants.remove(&(request.kind, key.clone())));

            let ws = state.workspace.as_mut().expect("checked workspace");
            let ws_files = Arc::clone(&ws.files);
            let ws_saved_at = Arc::clone(&ws.last_saved_at);
            let workspace_id = ws.workspace_id;

            let (action, changed_event) = match eval.decision {
                PolicyDecision::BlockSensitive => {
                    let record = build_permission_record(
                        &turn_id,
                        &request,
                        &eval,
                        TutorPermissionStatus::BlockedSensitive,
                        &now,
                        Some(now.clone()),
                    );
                    push_permission_record(ws, &turn_id, record);
                    let _ = ws.history_store.save_atomic(&ws.history);
                    (
                        ImmediateAction::Respond {
                            session,
                            request_id: request.request_id,
                            allowed: false,
                            fs_read: None,
                            fs_write: None,
                        },
                        None,
                    )
                }
                PolicyDecision::AutoAllow => {
                    let fs_read =
                        if request.is_fs_method && request.kind == TutorPermissionKind::Read {
                            Some(execute_approved_read(&ws_files, &eval))
                        } else {
                            None
                        };
                    let record = build_permission_record(
                        &turn_id,
                        &request,
                        &eval,
                        TutorPermissionStatus::AutoAllowed,
                        &now,
                        Some(now.clone()),
                    );
                    push_permission_record(ws, &turn_id, record);
                    let _ = ws.history_store.save_atomic(&ws.history);
                    (
                        ImmediateAction::Respond {
                            session,
                            request_id: request.request_id,
                            allowed: true,
                            fs_read,
                            fs_write: None,
                        },
                        None,
                    )
                }
                PolicyDecision::AskUser if has_prior_grant => {
                    let mut fs_read = None;
                    let mut fs_write = None;
                    let mut changed = None;
                    match request.kind {
                        TutorPermissionKind::Read => {
                            fs_read = Some(execute_approved_read(&ws_files, &eval));
                        }
                        TutorPermissionKind::Write => {
                            let content = request.write_content.as_deref().unwrap_or("");
                            let (res, file_change) = execute_approved_write(
                                &ws_files,
                                &ws_saved_at,
                                &eval,
                                content,
                                &now,
                            );
                            if let Some(change) = file_change {
                                let path = change.path.clone();
                                record_file_change(ws, &turn_id, change);
                                let _ = ws.history_store.save_atomic(&ws.history);
                                changed = Some(WorkspaceFilesChanged {
                                    workspace_id,
                                    paths: vec![path],
                                });
                            }
                            fs_write = Some(res);
                        }
                        TutorPermissionKind::Execute => {}
                    }
                    (
                        ImmediateAction::Respond {
                            session,
                            request_id: request.request_id,
                            allowed: true,
                            fs_read,
                            fs_write,
                        },
                        changed,
                    )
                }
                PolicyDecision::AskUser => {
                    let record = build_permission_record(
                        &turn_id,
                        &request,
                        &eval,
                        TutorPermissionStatus::Pending,
                        &now,
                        None,
                    );
                    push_permission_record(ws, &turn_id, record);
                    let _ = ws.history_store.save_atomic(&ws.history);
                    let req_id = request.request_id.clone();
                    state.pending_permissions.insert(
                        req_id,
                        PendingPermissionContext {
                            turn_id,
                            prompt: request,
                            eval,
                        },
                    );
                    state.status = TutorSessionStatus::AwaitingPermission;
                    (ImmediateAction::WaitForUser, None)
                }
            };

            (action, changed_event, state.snapshot_opt())
        };

        if let Some(event) = changed_event {
            (self.on_workspace_files_changed)(event);
        }
        if let Some(snapshot) = snapshot {
            (self.on_session_updated)(snapshot);
        }
        if let ImmediateAction::Respond {
            session,
            request_id,
            allowed,
            fs_read,
            fs_write,
        } = action
        {
            let _ = session.respond_permission(&request_id, allowed, fs_read, fs_write);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, TutorRuntimeState> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl TutorRuntimeState {
    fn require_workspace(&self, workspace_id: u64) -> Result<(), DomainError> {
        if self
            .workspace
            .as_ref()
            .is_some_and(|ws| ws.workspace_id == workspace_id)
        {
            Ok(())
        } else {
            Err(DomainError::new(ErrorCode::NoActiveWorkspace))
        }
    }

    fn next_id(&mut self, prefix: &str) -> String {
        let id = self.next_seq;
        self.next_seq += 1;
        format!("{prefix}-{id}")
    }

    fn teardown_session_inner(&mut self) -> Option<Arc<dyn TutorBackendSession>> {
        self.session_generation = self.session_generation.wrapping_add(1);
        self.session_turns_completed = 0;
        self.agent_name = None;
        self.pending_permissions.clear();
        self.approved_grants.clear();
        self.watched_writes.clear();
        self.session.take()
    }

    fn cancel_active_turn_in_history(&mut self) {
        let Some(turn_id) = self.active_turn_id.take() else {
            return;
        };
        self.pending_permissions
            .retain(|_, ctx| ctx.turn_id != turn_id);
        let now = Utc::now().to_rfc3339();
        if let Some(ws) = self.workspace.as_mut() {
            if let Some(msg) = ws.history.find_assistant_mut(&turn_id) {
                if msg.status == TutorMessageStatus::Streaming {
                    msg.status = TutorMessageStatus::Cancelled;
                }
                for perm in &mut msg.permissions {
                    if perm.status == TutorPermissionStatus::Pending {
                        perm.status = TutorPermissionStatus::Cancelled;
                        perm.resolved_at = Some(now.clone());
                    }
                }
            }
            let _ = ws.history_store.save_atomic(&ws.history);
        }
    }

    fn fail_turn(&mut self, turn_id: &str, error: DomainError) {
        if self.active_turn_id.as_deref() == Some(turn_id) {
            self.active_turn_id = None;
        }
        self.pending_permissions
            .retain(|_, ctx| ctx.turn_id != turn_id);
        self.last_error = Some(error.clone());
        self.status = TutorSessionStatus::Error;
        let now = Utc::now().to_rfc3339();
        if let Some(ws) = self.workspace.as_mut() {
            if let Some(msg) = ws.history.find_assistant_mut(turn_id) {
                msg.status = TutorMessageStatus::Failed;
                msg.error = Some(error);
                for perm in &mut msg.permissions {
                    if perm.status == TutorPermissionStatus::Pending {
                        perm.status = TutorPermissionStatus::Cancelled;
                        perm.resolved_at = Some(now.clone());
                    }
                }
            }
            let _ = ws.history_store.save_atomic(&ws.history);
        }
    }

    fn collect_watched_writes(&mut self, turn_id: &str) -> Option<WorkspaceFilesChanged> {
        let watched = std::mem::take(&mut self.watched_writes);
        if watched.is_empty() {
            return None;
        }
        let ws = self.workspace.as_mut()?;
        let ws_files = Arc::clone(&ws.files);
        let ws_saved_at = Arc::clone(&ws.last_saved_at);
        let workspace_id = ws.workspace_id;
        let now = Utc::now().to_rfc3339();
        let mut changed_paths = Vec::new();
        for (rel, before_text) in watched {
            let Some(after_text) = read_workspace_text(&ws_files, &rel) else {
                continue;
            };
            if before_text.as_deref() != Some(after_text.as_str()) {
                {
                    let mut guard = ws_saved_at.lock().unwrap_or_else(|p| p.into_inner());
                    *guard = (*guard).max(Utc::now());
                }
                record_file_change(
                    ws,
                    turn_id,
                    TutorFileChange {
                        path: rel.clone(),
                        before_text,
                        after_text,
                        created_at: now.clone(),
                    },
                );
                changed_paths.push(rel);
            }
        }
        if changed_paths.is_empty() {
            None
        } else {
            Some(WorkspaceFilesChanged {
                workspace_id,
                paths: changed_paths,
            })
        }
    }

    fn snapshot_opt(&self) -> Option<TutorSessionSnapshot> {
        let ws = self.workspace.as_ref()?;
        let can_retry = self.active_turn_id.is_none()
            && ws.history.messages.last().is_some_and(|m| {
                m.role == TutorRole::Assistant && m.status == TutorMessageStatus::Failed
            })
            && ws
                .history
                .messages
                .iter()
                .any(|m| m.role == TutorRole::User);
        Some(TutorSessionSnapshot {
            workspace_id: ws.workspace_id,
            status: self.status,
            process_running: self.session.is_some(),
            session_reused: self.session.is_some() && self.session_turns_completed > 0,
            preset: self.settings.preset,
            command: self.settings.resolved_command(),
            agent_name: self.agent_name.clone(),
            permission_policy: self.settings.permission_policy,
            teaching_style: self.settings.teaching_style,
            active_turn_id: self.active_turn_id.clone(),
            last_error: self.last_error.clone(),
            can_retry,
            messages: ws.history.messages.clone(),
        })
    }
}

fn build_permission_record(
    turn_id: &str,
    prompt: &BackendPermissionPrompt,
    eval: &PermissionEvaluation,
    status: TutorPermissionStatus,
    created_at: &str,
    resolved_at: Option<String>,
) -> TutorPermissionRequest {
    TutorPermissionRequest {
        id: prompt.request_id.clone(),
        turn_id: turn_id.to_owned(),
        kind: prompt.kind,
        title: prompt.title.clone(),
        target_path: eval
            .workspace_relative_path
            .clone()
            .or_else(|| prompt.target_path.clone()),
        command: prompt.command.clone(),
        outside_workspace: eval.outside_workspace,
        sensitive: eval.sensitive,
        high_risk: eval.high_risk,
        status,
        created_at: created_at.to_owned(),
        resolved_at,
    }
}

fn push_permission_record(
    ws: &mut TutorWorkspaceAttachment,
    turn_id: &str,
    record: TutorPermissionRequest,
) {
    if let Some(msg) = ws.history.find_assistant_mut(turn_id) {
        msg.permissions.push(record);
    }
}

fn update_permission_status(
    ws: &mut TutorWorkspaceAttachment,
    turn_id: &str,
    request_id: &str,
    status: TutorPermissionStatus,
    resolved_at: &str,
) {
    if let Some(msg) = ws.history.find_assistant_mut(turn_id)
        && let Some(perm) = msg.permissions.iter_mut().find(|p| p.id == request_id)
    {
        perm.status = status;
        perm.resolved_at = Some(resolved_at.to_owned());
    }
}

fn record_file_change(ws: &mut TutorWorkspaceAttachment, turn_id: &str, change: TutorFileChange) {
    if let Some(msg) = ws.history.find_assistant_mut(turn_id) {
        if let Some(existing) = msg.file_changes.iter_mut().find(|c| c.path == change.path) {
            existing.after_text = change.after_text;
            existing.created_at = change.created_at;
        } else {
            msg.file_changes.push(change);
        }
    }
}

fn grant_key(eval: &PermissionEvaluation, raw_path: Option<&str>) -> Option<String> {
    eval.workspace_relative_path
        .clone()
        .or_else(|| {
            eval.resolved_absolute_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
        })
        .or_else(|| raw_path.map(str::to_owned))
}

fn read_workspace_text(files: &WorkspacePaths, rel_path: &str) -> Option<String> {
    match files.read(rel_path).ok()?.contents {
        WorkspaceFileContents::Text { text } => Some(text),
        _ => None,
    }
}

fn execute_approved_read(
    files: &WorkspacePaths,
    eval: &PermissionEvaluation,
) -> Result<String, String> {
    if eval.sensitive {
        return Err("Sensitive file access blocked".to_owned());
    }
    if let Some(rel) = eval.workspace_relative_path.as_deref() {
        let file = files
            .read(rel)
            .map_err(|_| "Workspace file read failed".to_owned())?;
        return match file.contents {
            WorkspaceFileContents::Text { text } => Ok(text),
            WorkspaceFileContents::Missing => Err("File not found".to_owned()),
            WorkspaceFileContents::TooLarge => Err("File exceeds 2 MB text limit".to_owned()),
            WorkspaceFileContents::Binary => Err("File is binary".to_owned()),
        };
    }
    let Some(abs_path) = eval.resolved_absolute_path.as_deref() else {
        return Err("Invalid file path".to_owned());
    };
    read_bounded_external_text(abs_path)
}

fn read_bounded_external_text(path: &Path) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|_| "File read failed".to_owned())?;
    if !metadata.is_file() {
        return Err("Target is not a regular file".to_owned());
    }
    if metadata.len() > MAX_WORKSPACE_TEXT_BYTES as u64 {
        return Err("File exceeds 2 MB text limit".to_owned());
    }
    let file = fs::File::open(path).map_err(|_| "File read failed".to_owned())?;
    let mut buf = Vec::new();
    file.take((MAX_WORKSPACE_TEXT_BYTES + 1) as u64)
        .read_to_end(&mut buf)
        .map_err(|_| "File read failed".to_owned())?;
    if buf.len() > MAX_WORKSPACE_TEXT_BYTES {
        return Err("File exceeds 2 MB text limit".to_owned());
    }
    String::from_utf8(buf).map_err(|_| "File is not valid UTF-8 text".to_owned())
}

fn execute_approved_write(
    files: &WorkspacePaths,
    last_saved_at: &Mutex<DateTime<Utc>>,
    eval: &PermissionEvaluation,
    content: &str,
    now: &str,
) -> (Result<(), String>, Option<TutorFileChange>) {
    if eval.sensitive {
        return (Err("Sensitive file write blocked".to_owned()), None);
    }
    if content.len() > MAX_WORKSPACE_TEXT_BYTES {
        return (Err("File exceeds 2 MB text limit".to_owned()), None);
    }
    if let Some(rel) = eval.workspace_relative_path.as_deref() {
        let current = match files.read(rel) {
            Ok(file) => file,
            Err(_) => return (Err("Workspace file read failed".to_owned()), None),
        };
        let before_text = match &current.contents {
            WorkspaceFileContents::Text { text } => Some(text.clone()),
            WorkspaceFileContents::Missing => None,
            _ => return (Err("Workspace file is not editable text".to_owned()), None),
        };
        match files.write(rel, content, current.revision.as_deref(), true) {
            Ok(Ok(_)) => {
                {
                    let mut guard = last_saved_at.lock().unwrap_or_else(|p| p.into_inner());
                    *guard = (*guard).max(Utc::now());
                }
                let change = TutorFileChange {
                    path: rel.to_owned(),
                    before_text,
                    after_text: content.to_owned(),
                    created_at: now.to_owned(),
                };
                (Ok(()), Some(change))
            }
            _ => (Err("Workspace file write failed".to_owned()), None),
        }
    } else if let Some(abs_path) = eval.resolved_absolute_path.as_deref() {
        match fs::write(abs_path, content) {
            Ok(()) => (Ok(()), None),
            Err(_) => (Err("File write failed".to_owned()), None),
        }
    } else {
        (Err("Invalid file path".to_owned()), None)
    }
}

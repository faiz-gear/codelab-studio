use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;

use crate::error::{DomainError, ErrorCode};

use super::model::{AcpAgentPreset, TeachingStyle, TutorHandshakeResult, TutorPermissionKind};

#[derive(Debug, Clone)]
pub struct TutorLaunchSpec {
    pub preset: AcpAgentPreset,
    pub command: String,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TutorTurnInput {
    pub turn_id: String,
    pub prompt: String,
    pub teaching_style: TeachingStyle,
    pub context_preamble: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackendPermissionPrompt {
    pub request_id: String,
    pub kind: TutorPermissionKind,
    pub title: String,
    pub target_path: Option<String>,
    pub target_paths: Vec<String>,
    pub command: Option<String>,
    pub raw_input: Option<Value>,
    pub write_content: Option<String>,
    pub is_fs_method: bool,
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    Chunk {
        turn_id: String,
        delta: String,
    },
    PermissionRequested {
        turn_id: String,
        request: BackendPermissionPrompt,
    },
    TurnCompleted {
        turn_id: String,
        stop_reason: String,
    },
    TurnCancelled {
        turn_id: String,
    },
    TurnFailed {
        turn_id: String,
        code: ErrorCode,
    },
    ProcessExited {
        exit_code: Option<i32>,
        unexpected: bool,
    },
}

pub type BackendEventSink = Arc<dyn Fn(BackendEvent) + Send + Sync>;

/// Thin backend extension seam (ADR-0004): v1 ships pure ACP stdio only,
/// while keeping this trait clean for deterministic tests and future backends.
pub trait TutorBackendSession: Send + Sync {
    fn agent_name(&self) -> Option<String>;
    fn send_turn(&self, input: TutorTurnInput) -> Result<(), DomainError>;
    fn cancel_turn(&self, turn_id: &str) -> Result<(), DomainError>;
    fn respond_permission(
        &self,
        request_id: &str,
        allowed: bool,
        fs_read_result: Option<Result<String, String>>,
        fs_write_result: Option<Result<(), String>>,
    ) -> Result<(), DomainError>;
    fn shutdown(&self);
}

pub trait TutorBackend: Send + Sync {
    fn handshake(&self, spec: &TutorLaunchSpec) -> Result<TutorHandshakeResult, DomainError>;
    fn start_session(
        &self,
        spec: &TutorLaunchSpec,
        sink: BackendEventSink,
    ) -> Result<Box<dyn TutorBackendSession>, DomainError>;
}

pub(crate) fn teaching_style_directive(style: TeachingStyle) -> &'static str {
    match style {
        TeachingStyle::Socratic => {
            "[Teaching Style: socratic] Guide the learner step-by-step with clarifying hints and targeted questions first. Avoid giving away the full final solution immediately unless explicitly asked."
        }
        TeachingStyle::Direct => {
            "[Teaching Style: direct] Provide a direct, concise technical explanation and concrete solution guidance."
        }
    }
}

pub(crate) fn compose_turn_prompt(input: &TutorTurnInput) -> String {
    compose_prompt_parts(
        input.teaching_style,
        input.context_preamble.as_deref(),
        &input.prompt,
    )
}

pub(crate) fn compose_prompt_parts(
    teaching_style: TeachingStyle,
    context_preamble: Option<&str>,
    prompt: &str,
) -> String {
    let mut parts = vec![teaching_style_directive(teaching_style).to_owned()];
    if let Some(context) = context_preamble
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        parts.push(context.to_owned());
    }
    let trimmed_prompt = prompt.trim();
    if !trimmed_prompt.is_empty() {
        parts.push(trimmed_prompt.to_owned());
    }
    parts.join("\n\n")
}

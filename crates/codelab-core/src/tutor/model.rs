use serde::{Deserialize, Serialize};

use crate::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AcpAgentPreset {
    #[default]
    ClaudeCode,
    GeminiCli,
    CodexCli,
    OpenCode,
    Custom,
}

impl AcpAgentPreset {
    pub fn default_command(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude --acp",
            Self::GeminiCli => "gemini --acp",
            Self::CodexCli => "codex --acp",
            Self::OpenCode => "opencode acp",
            Self::Custom => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionPolicy {
    #[default]
    GuidedReadonly,
    StrictAskAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TeachingStyle {
    #[default]
    Socratic,
    Direct,
}

fn default_tutor_command() -> String {
    AcpAgentPreset::default().default_command().to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorSettings {
    #[serde(default)]
    pub preset: AcpAgentPreset,
    #[serde(default = "default_tutor_command")]
    pub command: String,
    #[serde(default)]
    pub permission_policy: PermissionPolicy,
    #[serde(default)]
    pub teaching_style: TeachingStyle,
}

impl Default for TutorSettings {
    fn default() -> Self {
        Self {
            preset: AcpAgentPreset::default(),
            command: default_tutor_command(),
            permission_policy: PermissionPolicy::default(),
            teaching_style: TeachingStyle::default(),
        }
    }
}

impl TutorSettings {
    pub fn is_valid(&self) -> bool {
        self.command.len() <= 2048
    }

    pub fn resolved_command(&self) -> String {
        let trimmed = self.command.trim();
        if trimmed.is_empty() && self.preset != AcpAgentPreset::Custom {
            self.preset.default_command().to_owned()
        } else {
            trimmed.to_owned()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorSettingsPatch {
    pub preset: Option<AcpAgentPreset>,
    pub command: Option<String>,
    pub permission_policy: Option<PermissionPolicy>,
    pub teaching_style: Option<TeachingStyle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TutorRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TutorMessageStatus {
    Streaming,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TutorPermissionKind {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TutorPermissionStatus {
    Pending,
    AutoAllowed,
    Approved,
    Denied,
    BlockedSensitive,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorPermissionRequest {
    pub id: String,
    pub turn_id: String,
    pub kind: TutorPermissionKind,
    pub title: String,
    pub target_path: Option<String>,
    pub command: Option<String>,
    pub outside_workspace: bool,
    pub sensitive: bool,
    pub high_risk: bool,
    pub status: TutorPermissionStatus,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorFileChange {
    pub path: String,
    pub before_text: Option<String>,
    pub after_text: String,
    pub created_at: String,
}

pub const MAX_CONTEXT_INPUT_TOKENS: u32 = 6_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextDimension {
    Chapter,
    ActiveFile,
    TerminalErrors,
    Verification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextQuoteSource {
    Manual,
    Editor,
    Terminal,
    CheckFailure,
    VerificationPanel,
    Reflection,
    PortConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextQuote {
    pub source: ContextQuoteSource,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub start_line: Option<u32>,
    #[serde(default)]
    pub end_line: Option<u32>,
    #[serde(default)]
    pub label: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshotInput {
    pub workspace_id: u64,
    #[serde(default)]
    pub chapter_id: Option<String>,
    #[serde(default)]
    pub active_file_path: Option<String>,
    #[serde(default)]
    pub active_file_text: Option<String>,
    #[serde(default)]
    pub active_file_start_line: Option<u32>,
    #[serde(default)]
    pub active_file_end_line: Option<u32>,
    #[serde(default)]
    pub quote: Option<ContextQuote>,
    #[serde(default)]
    pub excluded_dimensions: Vec<ContextDimension>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub teaching_style: Option<TeachingStyle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextChip {
    pub dimension: ContextDimension,
    pub label: String,
    #[serde(default)]
    pub detail: Option<String>,
    pub preview: String,
    pub estimated_tokens: u32,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshot {
    pub workspace_id: u64,
    pub chapter_id: Option<String>,
    pub quote: Option<ContextQuote>,
    pub chips: Vec<ContextChip>,
    pub excluded_dimensions: Vec<ContextDimension>,
    pub estimated_tokens: u32,
    pub max_tokens: u32,
    pub truncated: bool,
    pub sensitive_filtered: bool,
    pub rendered_preamble: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorMessage {
    pub id: String,
    pub turn_id: String,
    pub role: TutorRole,
    pub content: String,
    pub teaching_style: TeachingStyle,
    pub status: TutorMessageStatus,
    pub error: Option<DomainError>,
    #[serde(default)]
    pub quote: Option<ContextQuote>,
    #[serde(default)]
    pub context_dimensions: Vec<ContextDimension>,
    #[serde(default)]
    pub permissions: Vec<TutorPermissionRequest>,
    #[serde(default)]
    pub file_changes: Vec<TutorFileChange>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TutorSessionStatus {
    #[default]
    Idle,
    Starting,
    Ready,
    Streaming,
    AwaitingPermission,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorSessionSnapshot {
    pub workspace_id: u64,
    pub status: TutorSessionStatus,
    pub process_running: bool,
    pub session_reused: bool,
    pub preset: AcpAgentPreset,
    pub command: String,
    pub agent_name: Option<String>,
    pub permission_policy: PermissionPolicy,
    pub teaching_style: TeachingStyle,
    pub active_turn_id: Option<String>,
    pub last_error: Option<DomainError>,
    pub can_retry: bool,
    pub messages: Vec<TutorMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorHandshakeRequest {
    pub preset: AcpAgentPreset,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorHandshakeResult {
    pub preset: AcpAgentPreset,
    pub command: String,
    pub protocol_version: Option<String>,
    pub agent_name: Option<String>,
    pub agent_version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTutorPrompt {
    pub workspace_id: u64,
    pub prompt: String,
    #[serde(default)]
    pub teaching_style: Option<TeachingStyle>,
    #[serde(default)]
    pub context_preamble: Option<String>,
    #[serde(default)]
    pub context: Option<ContextSnapshotInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespondTutorPermission {
    pub workspace_id: u64,
    pub request_id: String,
    pub allow: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTutorRequest {
    pub workspace_id: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorHistory {
    #[serde(default)]
    pub messages: Vec<TutorMessage>,
}

impl TutorHistory {
    pub fn is_valid(&self) -> bool {
        self.messages
            .iter()
            .all(|msg| !msg.id.trim().is_empty() && !msg.turn_id.trim().is_empty())
    }

    pub fn find_assistant_mut(&mut self, turn_id: &str) -> Option<&mut TutorMessage> {
        self.messages
            .iter_mut()
            .rev()
            .find(|msg| msg.turn_id == turn_id && msg.role == TutorRole::Assistant)
    }

    pub fn recover_interrupted(&mut self) -> bool {
        let mut changed = false;
        for message in &mut self.messages {
            if message.status == TutorMessageStatus::Streaming {
                message.status = TutorMessageStatus::Cancelled;
                changed = true;
            }
            for perm in &mut message.permissions {
                if perm.status == TutorPermissionStatus::Pending {
                    perm.status = TutorPermissionStatus::Cancelled;
                    changed = true;
                }
            }
        }
        changed
    }
}

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::error::AppError;
use crate::domain::ids::{ChapterId, RelWorkspacePath, TerminalId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum TeachingMode {
    Socratic,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum SelectionSource {
    Manual,
    Editor,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ContextChipKind {
    #[serde(rename_all = "camelCase")]
    ChapterSummary {
        chapter_id: ChapterId,
        title: String,
    },
    #[serde(rename_all = "camelCase")]
    Selection {
        source: SelectionSource,
        preview: String,
        line_range: Option<(u32, u32)>,
    },
    #[serde(rename_all = "camelCase")]
    ActiveFileSnippet {
        path: RelWorkspacePath,
        start_line: u32,
        end_line: u32,
    },
    #[serde(rename_all = "camelCase")]
    TerminalErrorTail {
        terminal_id: TerminalId,
        line_count: u32,
    },
    #[serde(rename_all = "camelCase")]
    VerificationDiff {
        chapter_id: ChapterId,
        failed_count: u32,
    },
    #[serde(rename_all = "camelCase")]
    ExcludedSensitive {
        path: RelWorkspacePath,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextChip {
    pub chip_id: String,
    pub kind: ContextChipKind,
    pub estimated_tokens: u32,
    pub removable: bool,
    pub redacted_secrets: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum ToolCallStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum AcpOperationKind {
    Read,
    Write,
    Exec,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct AcpPermissionOption {
    pub option_id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AcpDecision {
    #[serde(rename_all = "camelCase")]
    AllowOnce {
        option_id: String,
    },
    #[serde(rename_all = "camelCase")]
    AllowSession {
        option_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Deny {
        reason: Option<String>,
    },
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum FinishReason {
    Stop,
    Length,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AiStreamEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        turn_id: String,
        prompt_tokens: u32,
        truncated_chips: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    TextDelta { turn_id: String, delta: String },
    #[serde(rename_all = "camelCase")]
    ThinkingDelta { turn_id: String, delta: String },
    #[serde(rename_all = "camelCase")]
    ToolCallUpdated {
        turn_id: String,
        call_id: String,
        title: String,
        status: ToolCallStatus,
    },
    #[serde(rename_all = "camelCase")]
    PermissionRequired {
        turn_id: String,
        request_id: String,
        agent_name: String,
        operation: AcpOperationKind,
        target_summary: String,
        options: Vec<AcpPermissionOption>,
    },
    #[serde(rename_all = "camelCase")]
    PermissionResolved {
        turn_id: String,
        request_id: String,
        decision: AcpDecision,
    },
    #[serde(rename_all = "camelCase")]
    Completed {
        turn_id: String,
        finish_reason: FinishReason,
    },
    #[serde(rename_all = "camelCase")]
    Failed { turn_id: String, error: AppError },
}

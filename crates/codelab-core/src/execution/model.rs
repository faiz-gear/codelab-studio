use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{CommandMode, WorkspaceFile};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTerminalRequest {
    pub workspace_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCommandBlock {
    pub workspace_id: u64,
    pub command_id: String,
    pub restart_terminal_id: Option<u64>,
    pub confirmation: Option<u64>,
    #[serde(default)]
    pub drafts: Vec<CommandDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandDraft {
    pub path: String,
    pub text: String,
    pub expected_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandConfirmation {
    pub token: u64,
    pub cwd: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandStarted {
    pub terminal: TerminalSummary,
    pub saved: Vec<WorkspaceFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSaveConflict {
    pub file: WorkspaceFile,
    pub saved: Vec<WorkspaceFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandCwdOverride {
    pub workspace_id: u64,
    pub command_id: String,
    pub cwd: Option<String>,
}

pub type CommandCwdOverrides = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalTarget {
    pub workspace_id: u64,
    pub terminal_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInput {
    #[serde(flatten)]
    pub target: TerminalTarget,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSize {
    #[serde(flatten)]
    pub target: TerminalTarget,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRead {
    pub workspace_id: u64,
    pub terminal_id: Option<u64>,
    #[serde(default)]
    pub cursor: u64,
    pub include_recent_errors: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSummary {
    pub id: u64,
    pub command_id: Option<String>,
    pub label: Option<String>,
    pub mode: Option<CommandMode>,
    pub cwd: String,
    pub started_at: String,
    pub port: Option<u16>,
    pub running: bool,
    pub exit_code: Option<u32>,
    pub port_conflict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutput {
    pub cursor: u64,
    pub reset: bool,
    pub data: String,
    pub more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalUpdates {
    pub terminals: Vec<TerminalSummary>,
    pub output: Option<TerminalOutput>,
    pub recent_errors: Vec<String>,
}

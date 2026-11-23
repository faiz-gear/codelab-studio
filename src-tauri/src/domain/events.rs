use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::ids::{RelWorkspacePath, SessionEpoch, TerminalId};
use crate::domain::lsp::{DiagnosticItem, LspLifecycleState};
use crate::domain::pty::{OneshotIdleState, PtyRole};
use crate::domain::runtime::Language;

pub const EVENT_FS_DELTA: &str = "workspace://fs-delta";
pub const EVENT_STORAGE_CORRUPTED: &str = "workspace://storage-corrupted";
pub const EVENT_PTY_STATE: &str = "pty://state";
pub const EVENT_PORT_CONFLICT: &str = "pty://port-conflict";
pub const EVENT_LSP_STATUS: &str = "lsp://status";
pub const EVENT_LSP_DIAGNOSTICS: &str = "lsp://diagnostics";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionEventEnvelope<T> {
    pub epoch: SessionEpoch,
    pub payload: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FsDelta {
    #[serde(rename_all = "camelCase")]
    Created { path: RelWorkspacePath },
    #[serde(rename_all = "camelCase")]
    ContentChanged {
        path: RelWorkspacePath,
        new_hash: String,
    },
    #[serde(rename_all = "camelCase")]
    Removed { path: RelWorkspacePath },
    #[serde(rename_all = "camelCase")]
    ManualChanged { path: RelWorkspacePath },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsDeltaEvent {
    pub deltas: Vec<FsDelta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StorageCorruptedEvent {
    pub file_name: String,
    pub backup_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PtyStateEvent {
    pub terminal_id: TerminalId,
    pub role: PtyRole,
    pub alive: bool,
    pub idle_state: OneshotIdleState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PortConflictEvent {
    pub terminal_id: TerminalId,
    pub port: Option<u16>,
    pub matched_line: String,
    pub candidate_daemon_id: Option<TerminalId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LspStatusEvent {
    pub language: Language,
    pub module_root: RelWorkspacePath,
    pub state: LspLifecycleState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LspDiagnosticsEvent {
    pub path: RelWorkspacePath,
    pub version: i32,
    pub items: Vec<DiagnosticItem>,
}

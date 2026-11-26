use serde::{Deserialize, Serialize};

use crate::StorageRecoveryNotice;

pub const MAX_WORKSPACE_TEXT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub id: u64,
    pub root: String,
    pub recovery_notices: Vec<StorageRecoveryNotice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePath {
    pub workspace_id: u64,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFileWrite {
    #[serde(flatten)]
    pub target: WorkspacePath,
    pub text: String,
    pub expected_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFileRename {
    #[serde(flatten)]
    pub target: WorkspacePath,
    pub new_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalIde {
    #[default]
    VsCode,
    Cursor,
    GoLand,
    IntelliJIdea,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceIdeRequest {
    #[serde(flatten)]
    pub target: WorkspacePath,
    pub ide: ExternalIde,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceEntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEntry {
    pub path: String,
    pub name: String,
    pub kind: WorkspaceEntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFile {
    pub path: String,
    pub size_bytes: u64,
    pub revision: Option<String>,
    #[serde(flatten)]
    pub contents: WorkspaceFileContents,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WorkspaceFileContents {
    Text { text: String },
    Binary,
    TooLarge,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFilesChanged {
    pub workspace_id: u64,
    pub paths: Vec<String>,
}

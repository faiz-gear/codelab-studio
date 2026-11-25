use crate::{CommandDraft, DomainError, WorkspaceFile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseTarget {
    pub workspace_id: u64,
    pub chapter_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunChecks {
    #[serde(flatten)]
    pub target: ExerciseTarget,
    #[serde(default)]
    pub drafts: Vec<CommandDraft>,
    pub confirmation: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSelfCheck {
    #[serde(flatten)]
    pub target: ExerciseTarget,
    pub check_id: String,
    pub checked: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionState {
    #[default]
    NotStarted,
    InProgress,
    Completed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    Running,
    Passed,
    Failed,
    TimedOut,
    Skipped,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDiff {
    pub label: String,
    pub expected: String,
    pub actual: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseResult {
    pub name: String,
    pub status: CheckStatus,
    pub duration_ms: u64,
    pub output: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub check_id: String,
    pub status: CheckStatus,
    pub duration_ms: u64,
    pub diffs: Vec<CheckDiff>,
    pub raw_output: String,
    pub test_cases: Vec<TestCaseResult>,
    pub error: Option<DomainError>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterProgress {
    pub completion_state: CompletionState,
    #[serde(default)]
    pub last_check_results: Vec<CheckResult>,
    #[serde(default)]
    pub self_checks: BTreeMap<String, bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningProgress {
    pub workspace_id: u64,
    pub completed_exercises: u32,
    pub total_exercises: u32,
    pub running_chapter_id: Option<String>,
    pub chapters: BTreeMap<String, ChapterProgress>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationStarted {
    pub progress: LearningProgress,
    pub saved: Vec<WorkspaceFile>,
}

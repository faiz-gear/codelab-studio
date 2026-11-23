use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::ids::{ChapterId, RunId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedActualDiff {
    pub label: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub check_id: String,
    pub title: String,
    pub passed: bool,
    #[ts(type = "number")]
    pub latency_ms: u64,
    pub summary: Option<String>,
    pub diff: Option<ExpectedActualDiff>,
    pub stdout_tail: Option<String>,
    pub stderr_tail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct GradeRunResult {
    pub run_id: RunId,
    pub chapter_id: ChapterId,
    pub passed: bool,
    pub passed_count: u32,
    pub total_count: u32,
    #[ts(type = "number")]
    pub total_latency_ms: u64,
    pub completed_at: String,
    pub results: Vec<CheckResult>,
}

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::ids::{RelWorkspacePath, RiskConfirmationToken};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PtyRole {
    #[serde(rename_all = "camelCase")]
    Daemon {
        service_key: String,
        label: String,
        port_hint: Option<u16>,
    },
    #[serde(rename_all = "camelCase")]
    Oneshot { slot_index: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OneshotIdleState {
    #[serde(rename_all = "camelCase")]
    Idle {
        last_exit_code: Option<i32>,
    },
    #[serde(rename_all = "camelCase")]
    Busy {
        active_command: Option<String>,
        #[ts(type = "number")]
        since_ms: u64,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct RiskRuleMatch {
    pub rule_id: String,
    pub reason_key: String,
    pub matched_segment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "level", rename_all = "camelCase")]
pub enum RiskAssessment {
    Safe,
    #[serde(rename_all = "camelCase")]
    HighRisk {
        matched_rules: Vec<RiskRuleMatch>,
        normalized_command: String,
        target_cwd: RelWorkspacePath,
        confirmation_token: RiskConfirmationToken,
    },
}

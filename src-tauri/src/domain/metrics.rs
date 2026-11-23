use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::ai::{AcpOperationKind, SelectionSource, TeachingMode};
use crate::domain::runtime::{Language, ManagedRuntimeTarget};
use crate::domain::workspace::{Locale, StorageSchemaVersion};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum MetricEventName {
    AppLaunched,
    CourseOpened,
    PreflightExecuted,
    RuntimeInstalled,
    CmdBlockClicked,
    CmdRiskModalAction,
    CmdFinished,
    FileTreeOpen,
    SelectionAiClicked,
    AiModeSwitched,
    AcpPermissionDecision,
    VerifyChapterClicked,
    ChapterMarkedDone,
}

impl MetricEventName {
    pub const ALL: [Self; 13] = [
        Self::AppLaunched,
        Self::CourseOpened,
        Self::PreflightExecuted,
        Self::RuntimeInstalled,
        Self::CmdBlockClicked,
        Self::CmdRiskModalAction,
        Self::CmdFinished,
        Self::FileTreeOpen,
        Self::SelectionAiClicked,
        Self::AiModeSwitched,
        Self::AcpPermissionDecision,
        Self::VerifyChapterClicked,
        Self::ChapterMarkedDone,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "name", content = "payload", rename_all = "snake_case")]
pub enum MetricEvent {
    #[serde(rename_all = "camelCase")]
    AppLaunched { locale: Locale },
    #[serde(rename_all = "camelCase")]
    CourseOpened {
        source_kind: String,
        language: Option<Language>,
    },
    #[serde(rename_all = "camelCase")]
    PreflightExecuted {
        ok_count: u32,
        warn_count: u32,
        missing_count: u32,
    },
    #[serde(rename_all = "camelCase")]
    RuntimeInstalled {
        target: ManagedRuntimeTarget,
        success: bool,
    },
    #[serde(rename_all = "camelCase")]
    CmdBlockClicked { role_kind: String },
    #[serde(rename_all = "camelCase")]
    CmdRiskModalAction { confirmed: bool },
    #[serde(rename_all = "camelCase")]
    CmdFinished {
        exit_code: Option<i32>,
        #[ts(type = "number")]
        duration_ms: u64,
    },
    #[serde(rename_all = "camelCase")]
    FileTreeOpen { opened: bool },
    #[serde(rename_all = "camelCase")]
    SelectionAiClicked { source: SelectionSource },
    #[serde(rename_all = "camelCase")]
    AiModeSwitched { mode: TeachingMode },
    #[serde(rename_all = "camelCase")]
    AcpPermissionDecision {
        operation: AcpOperationKind,
        allowed: bool,
    },
    #[serde(rename_all = "camelCase")]
    VerifyChapterClicked { chapter_index: u32 },
    #[serde(rename_all = "camelCase")]
    ChapterMarkedDone {
        chapter_index: u32,
        auto_passed: bool,
    },
}

impl MetricEvent {
    pub fn event_name(&self) -> MetricEventName {
        match self {
            Self::AppLaunched { .. } => MetricEventName::AppLaunched,
            Self::CourseOpened { .. } => MetricEventName::CourseOpened,
            Self::PreflightExecuted { .. } => MetricEventName::PreflightExecuted,
            Self::RuntimeInstalled { .. } => MetricEventName::RuntimeInstalled,
            Self::CmdBlockClicked { .. } => MetricEventName::CmdBlockClicked,
            Self::CmdRiskModalAction { .. } => MetricEventName::CmdRiskModalAction,
            Self::CmdFinished { .. } => MetricEventName::CmdFinished,
            Self::FileTreeOpen { .. } => MetricEventName::FileTreeOpen,
            Self::SelectionAiClicked { .. } => MetricEventName::SelectionAiClicked,
            Self::AiModeSwitched { .. } => MetricEventName::AiModeSwitched,
            Self::AcpPermissionDecision { .. } => MetricEventName::AcpPermissionDecision,
            Self::VerifyChapterClicked { .. } => MetricEventName::VerifyChapterClicked,
            Self::ChapterMarkedDone { .. } => MetricEventName::ChapterMarkedDone,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(untagged)]
pub enum MetricValue {
    Bool(bool),
    Int(i32),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct MetricRecord {
    pub id: String,
    pub name: MetricEventName,
    pub ts: String,
    #[serde(default)]
    pub payload: BTreeMap<String, MetricValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct MetricsLog {
    pub schema_version: StorageSchemaVersion,
    #[serde(default)]
    pub events: Vec<MetricRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_event_has_exact_13_prd_variants() {
        let expected_names = [
            "app_launched",
            "course_opened",
            "preflight_executed",
            "runtime_installed",
            "cmd_block_clicked",
            "cmd_risk_modal_action",
            "cmd_finished",
            "file_tree_open",
            "selection_ai_clicked",
            "ai_mode_switched",
            "acp_permission_decision",
            "verify_chapter_clicked",
            "chapter_marked_done",
        ];

        assert_eq!(MetricEventName::ALL.len(), 13);
        assert_eq!(expected_names.len(), 13);

        for (variant, expected) in MetricEventName::ALL.iter().zip(expected_names.iter()) {
            assert_eq!(serde_json::to_value(variant).unwrap(), *expected);
        }

        let sample_events = [
            MetricEvent::AppLaunched {
                locale: Locale::ZhCn,
            },
            MetricEvent::CourseOpened {
                source_kind: "builtin".to_owned(),
                language: Some(Language::Go),
            },
            MetricEvent::PreflightExecuted {
                ok_count: 4,
                warn_count: 1,
                missing_count: 0,
            },
            MetricEvent::RuntimeInstalled {
                target: ManagedRuntimeTarget::Go,
                success: true,
            },
            MetricEvent::CmdBlockClicked {
                role_kind: "oneshot".to_owned(),
            },
            MetricEvent::CmdRiskModalAction { confirmed: false },
            MetricEvent::CmdFinished {
                exit_code: Some(0),
                duration_ms: 420,
            },
            MetricEvent::FileTreeOpen { opened: true },
            MetricEvent::SelectionAiClicked {
                source: SelectionSource::Editor,
            },
            MetricEvent::AiModeSwitched {
                mode: TeachingMode::Direct,
            },
            MetricEvent::AcpPermissionDecision {
                operation: AcpOperationKind::Write,
                allowed: true,
            },
            MetricEvent::VerifyChapterClicked { chapter_index: 1 },
            MetricEvent::ChapterMarkedDone {
                chapter_index: 1,
                auto_passed: true,
            },
        ];

        assert_eq!(sample_events.len(), 13);
        for (event, expected) in sample_events.iter().zip(expected_names.iter()) {
            let value = serde_json::to_value(event).unwrap();
            assert_eq!(value["name"], *expected);
            assert_eq!(serde_json::to_value(event.event_name()).unwrap(), *expected);
        }

        assert!(serde_json::from_str::<MetricEventName>("\"unknown_event\"").is_err());
    }
}

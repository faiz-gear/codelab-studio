use std::collections::BTreeMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::course::CourseSource;
use crate::error::ErrorCode;
use crate::tutor::{PermissionPolicy, TeachingStyle, TutorPermissionKind};
use crate::{CommandMode, CourseLanguage};

pub const MAX_RETAINED_METRIC_EVENTS: usize = 500;
pub(crate) const MAX_PENDING_METRICS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricCourseKind {
    Builtin,
    LocalMarkdown,
    LocalPack,
    GitPack,
}

impl From<&CourseSource> for MetricCourseKind {
    fn from(source: &CourseSource) -> Self {
        match source {
            CourseSource::Builtin { .. } => Self::Builtin,
            CourseSource::LocalMarkdown { .. } => Self::LocalMarkdown,
            CourseSource::LocalPack { .. } => Self::LocalPack,
            CourseSource::GitPack { .. } => Self::GitPack,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricRiskSource {
    CommandBlock,
    Verification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricRiskDecision {
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricSelectionZone {
    Manual,
    Editor,
    Terminal,
    CheckFailure,
    VerificationPanel,
    Reflection,
    PortConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricSelectionAction {
    ExplainConcept,
    ExplainCode,
    CheckIssues,
    DiagnoseError,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricPermissionDecision {
    Approved,
    Denied,
    AutoAllowed,
    BlockedSensitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricCompletionTrigger {
    Verification,
    SelfCheck,
    Manual,
}

/// Strictly structured, zero-PII payloads for the 13 unified local metrics.
/// Contains only enums, counts, durations, and booleans—never source code,
/// raw commands, file paths, URLs, or chat transcripts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "detailKind", rename_all = "camelCase")]
pub enum MetricEventDetail {
    AppLaunched,
    CourseOpened {
        #[serde(rename = "courseKind")]
        course_kind: MetricCourseKind,
        language: Option<CourseLanguage>,
    },
    PreflightExecuted {
        #[serde(rename = "readyCount")]
        ready_count: u8,
        #[serde(rename = "totalLanguages")]
        total_languages: u8,
    },
    RuntimeInstalled {
        language: CourseLanguage,
        succeeded: bool,
        #[serde(rename = "errorCode", default, skip_serializing_if = "Option::is_none")]
        error_code: Option<ErrorCode>,
    },
    CmdBlockClicked {
        mode: CommandMode,
        #[serde(rename = "isRestart")]
        is_restart: bool,
    },
    CmdRiskModalAction {
        source: MetricRiskSource,
        decision: MetricRiskDecision,
    },
    CmdFinished {
        mode: Option<CommandMode>,
        #[serde(rename = "exitCode")]
        exit_code: u32,
        stopped: bool,
        #[serde(rename = "durationMs")]
        duration_ms: u64,
    },
    FileTreeOpen,
    SelectionAiClicked {
        zone: MetricSelectionZone,
        action: MetricSelectionAction,
    },
    AiModeSwitched {
        from: TeachingStyle,
        to: TeachingStyle,
    },
    AcpPermissionDecision {
        kind: TutorPermissionKind,
        decision: MetricPermissionDecision,
        policy: PermissionPolicy,
        #[serde(rename = "outsideWorkspace")]
        outside_workspace: bool,
        #[serde(rename = "highRisk")]
        high_risk: bool,
        sensitive: bool,
    },
    VerifyChapterClicked {
        #[serde(rename = "checkCount")]
        check_count: u16,
        #[serde(rename = "requiresDaemon")]
        requires_daemon: bool,
    },
    ChapterMarkedDone {
        trigger: MetricCompletionTrigger,
        #[serde(rename = "completedExercises")]
        completed_exercises: u32,
        #[serde(rename = "totalExercises")]
        total_exercises: u32,
    },
}

impl MetricEventDetail {
    pub fn event_name(&self) -> MetricEventName {
        match self {
            Self::AppLaunched => MetricEventName::AppLaunched,
            Self::CourseOpened { .. } => MetricEventName::CourseOpened,
            Self::PreflightExecuted { .. } => MetricEventName::PreflightExecuted,
            Self::RuntimeInstalled { .. } => MetricEventName::RuntimeInstalled,
            Self::CmdBlockClicked { .. } => MetricEventName::CmdBlockClicked,
            Self::CmdRiskModalAction { .. } => MetricEventName::CmdRiskModalAction,
            Self::CmdFinished { .. } => MetricEventName::CmdFinished,
            Self::FileTreeOpen => MetricEventName::FileTreeOpen,
            Self::SelectionAiClicked { .. } => MetricEventName::SelectionAiClicked,
            Self::AiModeSwitched { .. } => MetricEventName::AiModeSwitched,
            Self::AcpPermissionDecision { .. } => MetricEventName::AcpPermissionDecision,
            Self::VerifyChapterClicked { .. } => MetricEventName::VerifyChapterClicked,
            Self::ChapterMarkedDone { .. } => MetricEventName::ChapterMarkedDone,
        }
    }
}

/// Strictly typed UI-originated metric events forwarded through `AppCommand::RecordLocalMetric`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LocalMetricEvent {
    FileTreeOpen,
    SelectionAiClicked {
        zone: MetricSelectionZone,
        action: MetricSelectionAction,
    },
    CmdRiskModalCancelled {
        source: MetricRiskSource,
    },
}

impl LocalMetricEvent {
    pub(crate) fn into_detail(self) -> MetricEventDetail {
        match self {
            Self::FileTreeOpen => MetricEventDetail::FileTreeOpen,
            Self::SelectionAiClicked { zone, action } => {
                MetricEventDetail::SelectionAiClicked { zone, action }
            }
            Self::CmdRiskModalCancelled { source } => MetricEventDetail::CmdRiskModalAction {
                source,
                decision: MetricRiskDecision::Cancel,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordLocalMetric {
    pub workspace_id: u64,
    pub event: LocalMetricEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalMetricRecord {
    pub event: MetricEventName,
    pub recorded_at: String,
    #[serde(flatten)]
    pub detail: MetricEventDetail,
}

impl LocalMetricRecord {
    pub(crate) fn now(detail: MetricEventDetail) -> Self {
        Self {
            event: detail.event_name(),
            recorded_at: Utc::now().to_rfc3339(),
            detail,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMetrics {
    #[serde(default)]
    pub course_opened: u64,
    #[serde(default)]
    pub counts: BTreeMap<MetricEventName, u64>,
    #[serde(default)]
    pub events: Vec<LocalMetricRecord>,
}

impl WorkspaceMetrics {
    pub fn is_valid(&self) -> bool {
        self.events.len() <= MAX_RETAINED_METRIC_EVENTS
            && self.events.iter().all(|record| {
                !record.recorded_at.trim().is_empty() && record.event == record.detail.event_name()
            })
    }

    pub(crate) fn normalize_legacy(&mut self) {
        if self.course_opened > 0 {
            let count = self
                .counts
                .entry(MetricEventName::CourseOpened)
                .or_insert(0);
            *count = (*count).max(self.course_opened);
        } else if let Some(&count) = self.counts.get(&MetricEventName::CourseOpened) {
            self.course_opened = count;
        }
    }

    pub(crate) fn push(&mut self, record: LocalMetricRecord) {
        let name = record.event;
        let count = self.counts.entry(name).or_insert(0);
        *count = count.saturating_add(1);
        if name == MetricEventName::CourseOpened {
            self.course_opened = *count;
        }
        self.events.push(record);
        if self.events.len() > MAX_RETAINED_METRIC_EVENTS {
            let overflow = self.events.len() - MAX_RETAINED_METRIC_EVENTS;
            self.events.drain(..overflow);
        }
    }
}

use serde::{Deserialize, Serialize};

use crate::course::{CourseSource, CourseSummary};
use crate::error::ErrorCode;

pub const GLOBAL_DATA_SCHEMA_VERSION: u32 = 1;
pub const MIN_CODE_FONT_SIZE: f32 = 11.0;
pub const MAX_CODE_FONT_SIZE: f32 = 20.0;
pub const DEFAULT_CODE_FONT_SIZE: f32 = 13.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccentPreset {
    #[default]
    Neutral,
    Violet,
    Blue,
    Green,
    Amber,
    Orange,
    Coral,
    Rose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Locale {
    #[default]
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CourseLanguage {
    #[serde(rename = "Go")]
    Go,
    #[serde(rename = "JS/TS")]
    JsTs,
    #[serde(rename = "Python")]
    Python,
    #[serde(rename = "Java")]
    Java,
}

impl CourseLanguage {
    pub(crate) fn from_token(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "go" | "golang" => Some(Self::Go),
            "js/ts" | "js" | "jsx" | "ts" | "tsx" | "javascript" | "typescript" => Some(Self::JsTs),
            "python" | "py" => Some(Self::Python),
            "java" => Some(Self::Java),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    pub theme: ThemeMode,
    pub accent: AccentPreset,
    pub code_font_size: f32,
    pub locale: Locale,
    #[serde(default)]
    pub tutor: crate::tutor::TutorSettings,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Light,
            accent: AccentPreset::Neutral,
            code_font_size: DEFAULT_CODE_FONT_SIZE,
            locale: Locale::ZhCn,
            tutor: crate::tutor::TutorSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsPatch {
    pub theme: Option<ThemeMode>,
    pub accent: Option<AccentPreset>,
    pub code_font_size: Option<f32>,
    pub locale: Option<Locale>,
    #[serde(default)]
    pub tutor: Option<crate::tutor::TutorSettingsPatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentWorkspace {
    pub workspace_path: String,
    pub course_id: String,
    pub course_title: String,
    pub language: Option<CourseLanguage>,
    #[serde(default)]
    pub course_source: Option<CourseSource>,
    pub completed_exercises: u32,
    pub total_exercises: u32,
    pub last_opened_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordRecentWorkspaceInput {
    pub workspace_path: String,
    pub course_id: String,
    pub course_title: String,
    pub language: CourseLanguage,
    pub completed_exercises: u32,
    pub total_exercises: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrustSourceKind {
    LocalPath,
    GitRepo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustSource {
    pub source_kind: TrustSourceKind,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustEntry {
    pub source_kind: TrustSourceKind,
    pub normalized_source: String,
    pub trusted_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustState {
    pub source_kind: TrustSourceKind,
    pub normalized_source: String,
    pub trusted: bool,
    pub trusted_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreKind {
    Settings,
    RuntimePreferences,
    Recents,
    Trust,
    Workspace,
    Progress,
    Metrics,
    TutorHistory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRecoveryNotice {
    pub store: StoreKind,
    pub error_code: ErrorCode,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownReport {
    pub already_shut_down: bool,
    pub hooks_executed: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
    pub settings: GlobalSettings,
    pub recents: Vec<RecentWorkspace>,
    pub trust_entries: Vec<TrustEntry>,
    pub recovery_notices: Vec<StorageRecoveryNotice>,
    pub builtin_courses: Vec<CourseSummary>,
}

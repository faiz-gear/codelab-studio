use std::collections::BTreeMap;
use std::fmt;

use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use schemars::JsonSchema;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

use crate::domain::ai::TeachingMode;
use crate::domain::ids::{ChapterId, RelWorkspacePath, RunId, SessionEpoch};
use crate::domain::runtime::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TS)]
pub struct StorageSchemaVersion(#[ts(type = "1")] u8);

impl StorageSchemaVersion {
    pub const V1: Self = Self(1);

    pub fn as_u8(self) -> u8 {
        self.0
    }
}

impl Default for StorageSchemaVersion {
    fn default() -> Self {
        Self::V1
    }
}

impl Serialize for StorageSchemaVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.0)
    }
}

impl<'de> Deserialize<'de> for StorageSchemaVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V1Visitor;

        impl Visitor<'_> for V1Visitor {
            type Value = StorageSchemaVersion;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("integer schemaVersion 1")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                if value == 1 {
                    Ok(StorageSchemaVersion::V1)
                } else {
                    Err(E::invalid_value(de::Unexpected::Unsigned(value), &self))
                }
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                if value == 1 {
                    Ok(StorageSchemaVersion::V1)
                } else {
                    Err(E::invalid_value(de::Unexpected::Signed(value), &self))
                }
            }
        }

        deserializer.deserialize_any(V1Visitor)
    }
}

impl JsonSchema for StorageSchemaVersion {
    fn schema_name() -> String {
        "StorageSchemaVersion".to_owned()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        SchemaObject {
            instance_type: Some(InstanceType::Integer.into()),
            const_value: Some(serde_json::Value::from(1)),
            ..Default::default()
        }
        .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
pub enum Locale {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum AccentPreset {
    Neutral,
    Violet,
    Blue,
    Green,
    Amber,
    Orange,
    Coral,
    Rose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum ExternalIdeKind {
    Vscode,
    Cursor,
    Goland,
    Idea,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CourseSourceRef {
    #[serde(rename_all = "camelCase")]
    Builtin {
        course_id: String,
        entry_file: RelWorkspacePath,
    },
    #[serde(rename_all = "camelCase")]
    LocalMarkdown { file_path: String },
    #[serde(rename_all = "camelCase")]
    LocalPackDir {
        dir_path: String,
        entry_file: RelWorkspacePath,
    },
    #[serde(rename_all = "camelCase")]
    GitRepo {
        url: String,
        git_ref: Option<String>,
        entry_file: RelWorkspacePath,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct LayoutSnapshot {
    pub manual_width_pct: u16,
    pub bottom_height_px: u16,
    pub focus_mode: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMeta {
    pub schema_version: StorageSchemaVersion,
    pub workspace_id: String,
    pub workspace_root: String,
    pub course_source: CourseSourceRef,
    pub pack_fingerprint: String,
    #[serde(default)]
    pub hydrated_files: Vec<RelWorkspacePath>,
    pub active_chapter_id: Option<ChapterId>,
    #[serde(default)]
    pub open_tabs: Vec<RelWorkspacePath>,
    pub active_tab: Option<RelWorkspacePath>,
    pub layout: LayoutSnapshot,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ChapterStatus {
    NotStarted,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct AutoCheckRecord {
    pub passed: bool,
    #[ts(type = "number")]
    pub latency_ms: u64,
    pub checked_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChapterProgress {
    pub status: ChapterStatus,
    pub completed_at: Option<String>,
    pub last_run_id: Option<RunId>,
    #[serde(default)]
    pub auto_checks: BTreeMap<String, AutoCheckRecord>,
    #[serde(default)]
    pub self_checks: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProgressState {
    pub schema_version: StorageSchemaVersion,
    pub course_id: String,
    #[serde(default)]
    pub chapters: BTreeMap<ChapterId, ChapterProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub name: String,
    pub path: RelWorkspacePath,
    pub is_dir: bool,
    #[ts(type = "number | null")]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub children: Vec<FileNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub role: ChatRole,
    pub content: String,
    pub teaching_mode: Option<TeachingMode>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChapterChatLog {
    pub chapter_id: ChapterId,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChatHistory {
    pub schema_version: StorageSchemaVersion,
    #[serde(default)]
    pub chapters: BTreeMap<ChapterId, ChapterChatLog>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct GlobalConfigView {
    pub schema_version: StorageSchemaVersion,
    pub locale: Locale,
    pub theme: ThemeMode,
    pub accent: AccentPreset,
    pub code_font_size_px: u16,
    pub external_ide: ExternalIdeKind,
    pub lsp_enabled: bool,
    pub default_teaching_mode: TeachingMode,
    pub opt_in_anonymous_telemetry: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct GlobalConfigPatch {
    pub locale: Option<Locale>,
    pub theme: Option<ThemeMode>,
    pub accent: Option<AccentPreset>,
    pub code_font_size_px: Option<u16>,
    pub external_ide: Option<ExternalIdeKind>,
    pub lsp_enabled: Option<bool>,
    pub default_teaching_mode: Option<TeachingMode>,
    pub opt_in_anonymous_telemetry: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct RecentWorkspaceItem {
    pub workspace_id: String,
    pub workspace_root: String,
    pub course_id: String,
    pub course_title: String,
    pub language: Language,
    pub completed_chapters: u32,
    pub total_chapters: u32,
    pub last_opened_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinCourseSummary {
    pub course_id: String,
    pub title: String,
    pub description: String,
    pub language: Language,
    pub chapter_count: u32,
    pub entry_file: RelWorkspacePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapState {
    pub epoch: SessionEpoch,
    pub config: GlobalConfigView,
    pub recent_workspaces: Vec<RecentWorkspaceItem>,
    pub builtin_courses: Vec<BuiltinCourseSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_schema_version_accepts_only_integer_one() {
        let parsed: StorageSchemaVersion = serde_json::from_str("1").unwrap();
        assert_eq!(parsed, StorageSchemaVersion::V1);
        assert_eq!(serde_json::to_string(&parsed).unwrap(), "1");

        for invalid in ["0", "2", "-1", "\"1\"", "\"1.0\"", "1.0", "true", "null"] {
            assert!(
                serde_json::from_str::<StorageSchemaVersion>(invalid).is_err(),
                "expected {invalid} to fail"
            );
        }
    }

    #[test]
    fn workspace_meta_round_trips_architecture_example() {
        let json = r#"{
            "schemaVersion": 1,
            "workspaceId": "ws_01J00000000000000000000000",
            "workspaceRoot": "/Users/dev/go-practice",
            "courseSource": {
                "kind": "builtin",
                "courseId": "go-goframe-practice",
                "entryFile": "README.md"
            },
            "packFingerprint": "sha256:abcdef",
            "hydratedFiles": ["task-api/go.mod", "task-api/main.go"],
            "activeChapterId": "ex-1-healthz",
            "openTabs": ["task-api/main.go", "task-api/main_test.go"],
            "activeTab": "task-api/main.go",
            "layout": { "manualWidthPct": 35, "bottomHeightPx": 256, "focusMode": false },
            "updatedAt": "2026-11-22T12:00:00Z"
        }"#;

        let parsed: WorkspaceMeta = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.schema_version, StorageSchemaVersion::V1);
        assert_eq!(parsed.hydrated_files.len(), 2);
        let round_tripped: WorkspaceMeta =
            serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
        assert_eq!(parsed, round_tripped);
    }

    #[test]
    fn progress_state_round_trips_architecture_example() {
        let json = r#"{
            "schemaVersion": 1,
            "courseId": "go-goframe-practice",
            "chapters": {
                "ex-1-healthz": {
                    "status": "completed",
                    "completedAt": "2026-11-22T12:10:00Z",
                    "lastRunId": "run_01J00000000000000000000000",
                    "autoChecks": {
                        "v1": { "passed": true, "latencyMs": 4, "checkedAt": "2026-11-22T12:10:00Z" },
                        "v2": { "passed": true, "latencyMs": 820, "checkedAt": "2026-11-22T12:10:00Z" }
                    },
                    "selfChecks": { "s1": true }
                }
            }
        }"#;

        let parsed: ProgressState = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.schema_version, StorageSchemaVersion::V1);
        assert_eq!(parsed.chapters.len(), 1);
        let round_tripped: ProgressState =
            serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
        assert_eq!(parsed, round_tripped);
    }
}

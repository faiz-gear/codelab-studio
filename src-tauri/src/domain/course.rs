use std::collections::BTreeMap;
use std::fmt;

use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use schemars::JsonSchema;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

use crate::domain::ids::{ChapterId, CommandBlockId, RelWorkspacePath, SessionEpoch};
use crate::domain::pty::PtyRole;
use crate::domain::runtime::Language;
use crate::domain::workspace::{CourseSourceRef, ProgressState, WorkspaceMeta};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TS)]
pub struct PackSchemaVersion(#[ts(type = "\"1.0\"")] &'static str);

impl PackSchemaVersion {
    pub const V1_0: Self = Self("1.0");

    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl Default for PackSchemaVersion {
    fn default() -> Self {
        Self::V1_0
    }
}

impl Serialize for PackSchemaVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

impl<'de> Deserialize<'de> for PackSchemaVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V10Visitor;

        impl Visitor<'_> for V10Visitor {
            type Value = PackSchemaVersion;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string schemaVersion \"1.0\"")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                if value == "1.0" {
                    Ok(PackSchemaVersion::V1_0)
                } else {
                    Err(E::invalid_value(de::Unexpected::Str(value), &self))
                }
            }
        }

        deserializer.deserialize_str(V10Visitor)
    }
}

impl JsonSchema for PackSchemaVersion {
    fn schema_name() -> String {
        "PackSchemaVersion".to_owned()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            const_value: Some(serde_json::Value::String("1.0".to_owned())),
            ..Default::default()
        }
        .into()
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "snake_case")]
pub enum ChapterKind {
    #[default]
    Exercise,
    Reading,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChapterMatchSpec {
    #[serde(default)]
    pub anchor: Option<String>,
    #[serde(default)]
    pub heading_exact: Option<String>,
    #[serde(default)]
    pub heading_prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct CourseChapterRef {
    pub chapter_id: ChapterId,
    pub title: String,
    #[serde(default)]
    pub kind: Option<ChapterKind>,
    #[serde(default, rename = "match")]
    pub heading_match: Option<ChapterMatchSpec>,
    #[serde(default)]
    pub default_cwd: Option<RelWorkspacePath>,
    #[serde(default)]
    pub entry_file: Option<RelWorkspacePath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct CourseManifest {
    pub schema_version: PackSchemaVersion,
    pub course_id: String,
    pub title: String,
    pub description: String,
    pub language: Language,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    pub entry_file: RelWorkspacePath,
    #[serde(default)]
    pub starter_dir: Option<RelWorkspacePath>,
    #[serde(default)]
    pub checks_file: Option<RelWorkspacePath>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub chapters: Vec<CourseChapterRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct DaemonRequirement {
    pub label: String,
    pub port: u16,
    #[serde(default)]
    pub service_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestSpec {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct HttpExpectSpec {
    pub status: u16,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body_contains: Option<String>,
    #[serde(default)]
    pub body_equals: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckSpec {
    #[serde(rename_all = "camelCase")]
    HttpAssert {
        id: String,
        title: String,
        request: HttpRequestSpec,
        expect: HttpExpectSpec,
        #[serde(default)]
        timeout_sec: Option<u32>,
    },
    #[serde(rename_all = "camelCase")]
    CommandTest {
        id: String,
        title: String,
        #[serde(default)]
        cwd: Option<RelWorkspacePath>,
        command: String,
        #[serde(default)]
        timeout_sec: Option<u32>,
        #[serde(default)]
        expect_exit_code: Option<i32>,
        #[serde(default)]
        forbid_output: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    FunctionTest {
        id: String,
        title: String,
        language: Language,
        entry_file: RelWorkspacePath,
        #[serde(default)]
        test_file: Option<RelWorkspacePath>,
        #[serde(default)]
        cwd: Option<RelWorkspacePath>,
        #[serde(default)]
        timeout_sec: Option<u32>,
    },
    #[serde(rename_all = "camelCase")]
    SelfCheck {
        id: String,
        title: String,
        #[serde(default)]
        hint: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChapterChecksSpec {
    pub chapter_id: ChapterId,
    #[serde(default)]
    pub requires_daemon: Option<DaemonRequirement>,
    pub checks: Vec<CheckSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MultiChapterChecksSpec {
    pub schema_version: PackSchemaVersion,
    pub course_id: String,
    pub language: Language,
    pub chapters: Vec<ChapterChecksSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SingleChapterChecksSpec {
    pub schema_version: PackSchemaVersion,
    pub chapter_id: ChapterId,
    pub language: Language,
    #[serde(default)]
    pub requires_daemon: Option<DaemonRequirement>,
    pub checks: Vec<CheckSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(untagged)]
pub enum ChecksSpec {
    MultiChapter(MultiChapterChecksSpec),
    SingleChapter(SingleChapterChecksSpec),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MarkdownNode {
    #[serde(rename_all = "camelCase")]
    Paragraph {
        children: Vec<MarkdownNode>,
    },
    #[serde(rename_all = "camelCase")]
    Heading {
        level: u8,
        id: Option<String>,
        children: Vec<MarkdownNode>,
    },
    #[serde(rename_all = "camelCase")]
    Text {
        value: String,
    },
    #[serde(rename_all = "camelCase")]
    Strong {
        children: Vec<MarkdownNode>,
    },
    #[serde(rename_all = "camelCase")]
    Emphasis {
        children: Vec<MarkdownNode>,
    },
    #[serde(rename_all = "camelCase")]
    InlineCode {
        value: String,
    },
    #[serde(rename_all = "camelCase")]
    CodeBlock {
        language: Option<String>,
        code: String,
        target_path: Option<RelWorkspacePath>,
        command_block_id: Option<CommandBlockId>,
    },
    #[serde(rename_all = "camelCase")]
    List {
        ordered: bool,
        start: Option<u32>,
        items: Vec<Vec<MarkdownNode>>,
    },
    #[serde(rename_all = "camelCase")]
    BlockQuote {
        children: Vec<MarkdownNode>,
    },
    #[serde(rename_all = "camelCase")]
    Table {
        headers: Vec<Vec<MarkdownNode>>,
        rows: Vec<Vec<Vec<MarkdownNode>>>,
    },
    #[serde(rename_all = "camelCase")]
    Link {
        href: String,
        title: Option<String>,
        children: Vec<MarkdownNode>,
    },
    #[serde(rename_all = "camelCase")]
    Image {
        src: String,
        alt: String,
        title: Option<String>,
    },
    LineBreak,
    ThematicBreak,
    #[serde(rename_all = "camelCase")]
    Kbd {
        label: String,
    },
    #[serde(rename_all = "camelCase")]
    Superscript {
        children: Vec<MarkdownNode>,
    },
    #[serde(rename_all = "camelCase")]
    Subscript {
        children: Vec<MarkdownNode>,
    },
    #[serde(rename_all = "camelCase")]
    Details {
        summary: String,
        children: Vec<MarkdownNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct CommandBlock {
    pub id: CommandBlockId,
    pub chapter_id: ChapterId,
    pub command: String,
    pub cwd: Option<RelWorkspacePath>,
    pub role: PtyRole,
    pub is_secondary: bool,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceCodeBlock {
    pub code_index: u32,
    pub language: Option<String>,
    pub target_path: Option<RelWorkspacePath>,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct SelfCheckItem {
    pub id: String,
    pub title: String,
    pub checked_by_default: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChapterThreeStepFlow {
    pub step1_primary_commands: Vec<CommandBlockId>,
    pub step2_reference_blocks: Vec<ReferenceCodeBlock>,
    pub step3_self_checks: Vec<SelfCheckItem>,
    pub secondary_commands: Vec<CommandBlockId>,
    pub reflection_nodes: Vec<MarkdownNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ParsedChapter {
    pub chapter_id: ChapterId,
    pub index: u32,
    #[serde(default)]
    pub kind: ChapterKind,
    pub title: String,
    pub goal: Option<String>,
    #[serde(default)]
    pub default_cwd: Option<RelWorkspacePath>,
    #[serde(default)]
    pub entry_file: Option<RelWorkspacePath>,
    pub nodes: Vec<MarkdownNode>,
    pub command_blocks: Vec<CommandBlock>,
    pub self_check_items: Vec<String>,
    pub three_step: ChapterThreeStepFlow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ParsedCourse {
    pub course_id: String,
    pub title: String,
    pub language: Option<Language>,
    #[serde(default)]
    pub intro_nodes: Vec<MarkdownNode>,
    pub chapters: Vec<ParsedChapter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CourseSourceInput {
    #[serde(rename_all = "camelCase")]
    Builtin { course_id: String },
    #[serde(rename_all = "camelCase")]
    LocalMarkdown { file_path: String },
    #[serde(rename_all = "camelCase")]
    LocalPackDir { dir_path: String },
    #[serde(rename_all = "camelCase")]
    GitRepo {
        url: String,
        #[serde(default)]
        git_ref: Option<String>,
        #[serde(default)]
        refresh: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct CourseChapterPreview {
    pub chapter_id: ChapterId,
    pub index: u32,
    pub kind: ChapterKind,
    pub title: String,
    pub command_count: u32,
    pub auto_check_count: u32,
    pub self_check_count: u32,
    pub default_cwd: Option<RelWorkspacePath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct CommandSummary {
    pub id: CommandBlockId,
    pub chapter_id: ChapterId,
    pub command: String,
    pub cwd: Option<RelWorkspacePath>,
    pub role: PtyRole,
    pub is_secondary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct PackValidationIssue {
    pub file: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct CourseInspectResult {
    pub source: CourseSourceRef,
    pub source_key: String,
    pub pack_fingerprint: String,
    pub requires_trust: bool,
    pub trusted: bool,
    pub manifest: Option<CourseManifest>,
    pub checks: Option<ChecksSpec>,
    pub course: ParsedCourse,
    pub starter_files: Vec<RelWorkspacePath>,
    pub chapters_preview: Vec<CourseChapterPreview>,
    pub command_summaries: Vec<CommandSummary>,
    pub validation_issues: Vec<PackValidationIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OpenWorkspaceReq {
    pub source: CourseSourceInput,
    pub workspace_root: String,
    #[serde(default)]
    pub create_if_missing: bool,
    #[serde(default)]
    pub accept_trust: bool,
    #[serde(default)]
    pub expected_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct OpenedWorkspaceView {
    pub epoch: SessionEpoch,
    pub workspace_meta: WorkspaceMeta,
    pub progress: ProgressState,
    pub course: ParsedCourse,
    pub manifest: Option<CourseManifest>,
    pub checks: Option<ChecksSpec>,
    pub newly_hydrated_files: Vec<RelWorkspacePath>,
    pub skipped_existing_files: Vec<RelWorkspacePath>,
    pub recovered_from_backup: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct AuthorPreviewResult {
    pub course: ParsedCourse,
    pub checks: Option<ChecksSpec>,
    pub validation_issues: Vec<PackValidationIssue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_schema_version_accepts_only_string_1_0() {
        let parsed: PackSchemaVersion = serde_json::from_str("\"1.0\"").unwrap();
        assert_eq!(parsed, PackSchemaVersion::V1_0);
        assert_eq!(serde_json::to_string(&parsed).unwrap(), "\"1.0\"");

        for invalid in ["\"2.0\"", "\"1\"", "\"\"", "1", "1.0", "2", "null"] {
            assert!(
                serde_json::from_str::<PackSchemaVersion>(invalid).is_err(),
                "expected {invalid} to fail"
            );
        }
    }

    #[test]
    fn checks_spec_round_trips_prd_multi_chapter_example() {
        let json = r#"{
            "schemaVersion": "1.0",
            "courseId": "go-goframe-practice",
            "language": "go",
            "chapters": [
                {
                    "chapterId": "ex-1",
                    "requiresDaemon": { "label": "go run .", "port": 8080 },
                    "checks": [
                        {
                            "id": "v1",
                            "title": "GET /healthz 返回 200 {\"status\":\"ok\"}",
                            "type": "http_assert",
                            "request": { "method": "GET", "url": "http://127.0.0.1:8080/healthz" },
                            "expect": { "status": 200, "bodyContains": "\"status\":\"ok\"" }
                        },
                        {
                            "id": "v2",
                            "title": "go test -race ./... 零竞态通过",
                            "type": "command_test",
                            "cwd": "task-api",
                            "command": "go test -json -race ./...",
                            "timeoutSec": 10,
                            "expectExitCode": 0,
                            "forbidOutput": ["WARNING: DATA RACE"]
                        }
                    ]
                }
            ]
        }"#;

        let parsed: ChecksSpec = serde_json::from_str(json).unwrap();
        assert!(matches!(parsed, ChecksSpec::MultiChapter(_)));
        let round_tripped: ChecksSpec =
            serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
        assert_eq!(parsed, round_tripped);
    }

    #[test]
    fn checks_spec_supports_single_chapter_and_rejects_unknown_check_type() {
        let single = r#"{
            "schemaVersion": "1.0",
            "chapterId": "ex-1",
            "language": "ts",
            "checks": [
                {
                    "id": "f1",
                    "title": "pipeline transform passes",
                    "type": "function_test",
                    "language": "ts",
                    "entryFile": "src/pipeline.ts",
                    "testFile": "src/pipeline.test.ts"
                },
                {
                    "id": "s1",
                    "title": "reviewed edge cases",
                    "type": "self_check"
                }
            ]
        }"#;

        let parsed: ChecksSpec = serde_json::from_str(single).unwrap();
        assert!(matches!(parsed, ChecksSpec::SingleChapter(_)));

        let invalid = r#"{
            "schemaVersion": "1.0",
            "chapterId": "ex-1",
            "language": "go",
            "checks": [
                {
                    "id": "bad",
                    "title": "unknown",
                    "type": "arbitrary_exec"
                }
            ]
        }"#;
        assert!(serde_json::from_str::<ChecksSpec>(invalid).is_err());
    }

    #[test]
    fn course_chapter_ref_round_trips_legacy_and_extended_json() {
        let legacy = r#"{"chapterId":"ex-1","title":"Ex 1"}"#;
        let parsed_legacy: CourseChapterRef = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed_legacy.chapter_id, ChapterId("ex-1".to_owned()));
        assert_eq!(parsed_legacy.title, "Ex 1");
        assert_eq!(parsed_legacy.kind, None);
        assert_eq!(parsed_legacy.heading_match, None);
        assert_eq!(parsed_legacy.default_cwd, None);
        assert_eq!(parsed_legacy.entry_file, None);
        let legacy_rt: CourseChapterRef =
            serde_json::from_str(&serde_json::to_string(&parsed_legacy).unwrap()).unwrap();
        assert_eq!(parsed_legacy, legacy_rt);

        let extended = r#"{
            "chapterId": "ex-2",
            "title": "练习 2：内存版任务 CRUD",
            "kind": "exercise",
            "match": {
                "anchor": "ex-2",
                "headingExact": "练习 2：内存版任务 CRUD",
                "headingPrefix": "练习 2："
            },
            "defaultCwd": "task-api",
            "entryFile": "task-api/main.go"
        }"#;
        let parsed_extended: CourseChapterRef = serde_json::from_str(extended).unwrap();
        assert_eq!(parsed_extended.chapter_id, ChapterId("ex-2".to_owned()));
        assert_eq!(parsed_extended.kind, Some(ChapterKind::Exercise));
        assert_eq!(
            parsed_extended.heading_match,
            Some(ChapterMatchSpec {
                anchor: Some("ex-2".to_owned()),
                heading_exact: Some("练习 2：内存版任务 CRUD".to_owned()),
                heading_prefix: Some("练习 2：".to_owned()),
            })
        );
        assert_eq!(
            parsed_extended
                .default_cwd
                .as_ref()
                .map(RelWorkspacePath::as_str),
            Some("task-api")
        );
        assert_eq!(
            parsed_extended
                .entry_file
                .as_ref()
                .map(RelWorkspacePath::as_str),
            Some("task-api/main.go")
        );
        let serialized_extended = serde_json::to_string(&parsed_extended).unwrap();
        assert!(serialized_extended.contains("\"match\""));
        let extended_rt: CourseChapterRef = serde_json::from_str(&serialized_extended).unwrap();
        assert_eq!(parsed_extended, extended_rt);
    }
}

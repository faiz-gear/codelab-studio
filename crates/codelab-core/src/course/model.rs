use serde::{Deserialize, Serialize};

use crate::model::CourseLanguage;

pub const MAX_MARKDOWN_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CourseSource {
    Builtin {
        #[serde(rename = "courseId")]
        course_id: String,
    },
    LocalMarkdown {
        path: String,
    },
    LocalPack {
        path: String,
    },
    GitPack {
        url: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OpenCourseRequest {
    Builtin {
        #[serde(rename = "courseId")]
        course_id: String,
    },
    LocalMarkdown {
        path: String,
    },
    LocalPack {
        path: String,
    },
    GitPack {
        url: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChapterKind {
    Guide,
    Exercise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChapterPresentation {
    ThreeStep,
    LinearFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandMode {
    Oneshot,
    Daemon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValueOrigin {
    Explicit,
    Inferred,
    ChapterDefault,
    WorkspaceRoot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandBlock {
    pub id: String,
    pub shell: String,
    pub command: String,
    pub cwd: Option<String>,
    pub cwd_origin: ValueOrigin,
    pub mode: CommandMode,
    pub mode_origin: ValueOrigin,
    pub label: Option<String>,
    pub port: Option<u16>,
    pub is_secondary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckKind {
    HttpAssert,
    CommandTest,
    TestSuite,
    SelfCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaemonRequirement {
    pub label: String,
    pub port: u16,
    pub service_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseCheckSpec {
    pub id: String,
    pub title: String,
    pub kind: CheckKind,
    pub required: bool,
    pub cwd: Option<String>,
    pub command: Option<String>,
    pub method: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ContentBlock {
    Prose {
        html: String,
    },
    Command {
        #[serde(rename = "commandId")]
        command_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeStepProjection {
    pub step1_intro_html: Option<String>,
    pub step1_command_ids: Vec<String>,
    pub step2_entry_file: Option<String>,
    pub step2_html: Option<String>,
    pub step3_intro_html: Option<String>,
    pub step3_checks: Vec<CourseCheckSpec>,
    pub requires_daemon: Option<DaemonRequirement>,
    pub secondary_command_ids: Vec<String>,
    pub reflections_html: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub chapter_id: String,
    pub title: String,
    pub kind: ChapterKind,
    pub presentation: ChapterPresentation,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_reference_hint: bool,
    pub default_cwd: Option<String>,
    pub entry_file: Option<String>,
    pub summary_html: Option<String>,
    pub blocks: Vec<ContentBlock>,
    pub commands: Vec<CommandBlock>,
    pub three_step: Option<ThreeStepProjection>,
    pub reflections_html: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSummary {
    pub course_id: String,
    pub title: String,
    pub description: String,
    pub language: Option<CourseLanguage>,
    pub source: CourseSource,
    pub total_chapters: u32,
    pub total_exercises: u32,
    pub is_builtin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedCourse {
    pub course_id: String,
    pub title: String,
    pub description: String,
    pub language: Option<CourseLanguage>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub dependencies: Vec<String>,
    pub source: CourseSource,
    pub entry_markdown_path: Option<String>,
    pub is_builtin: bool,
    pub trusted: bool,
    pub intro_html: Option<String>,
    pub total_exercises: u32,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSourceChangedEvent {
    pub course_id: String,
    pub changed_paths: Vec<String>,
    pub detected_at: String,
}

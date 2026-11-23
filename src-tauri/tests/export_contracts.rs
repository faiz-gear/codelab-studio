use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use codelab_studio_lib::domain::ai::{
    AcpDecision, AcpOperationKind, AcpPermissionOption, AiStreamEvent, ContextChip,
    ContextChipKind, FinishReason, SelectionSource, TeachingMode, ToolCallStatus,
};
use codelab_studio_lib::domain::course::{
    ChapterChecksSpec, CheckSpec, ChecksSpec, CommandBlock, CourseChapterRef, CourseManifest,
    DaemonRequirement, HttpExpectSpec, HttpRequestSpec, MarkdownNode, MultiChapterChecksSpec,
    PackSchemaVersion, ParsedChapter, ParsedCourse, SingleChapterChecksSpec,
};
use codelab_studio_lib::domain::error::{AppError, ErrorCode};
use codelab_studio_lib::domain::events::{
    FsDelta, FsDeltaEvent, LspDiagnosticsEvent, LspStatusEvent, PortConflictEvent, PtyStateEvent,
    SessionEventEnvelope, StorageCorruptedEvent,
};
use codelab_studio_lib::domain::grading::{CheckResult, ExpectedActualDiff, GradeRunResult};
use codelab_studio_lib::domain::ids::{
    ChapterId, CommandBlockId, RelWorkspacePath, RiskConfirmationToken, RunId, SessionEpoch,
    TerminalId,
};
use codelab_studio_lib::domain::lsp::{DiagnosticItem, DiagnosticSeverity, LspLifecycleState};
use codelab_studio_lib::domain::metrics::{
    MetricEvent, MetricEventName, MetricRecord, MetricValue, MetricsLog,
};
use codelab_studio_lib::domain::pty::{OneshotIdleState, PtyRole, RiskAssessment, RiskRuleMatch};
use codelab_studio_lib::domain::runtime::{
    Language, ManagedRuntimeTarget, PreflightCheckItem, PreflightReport, PreflightStatus,
};
use codelab_studio_lib::domain::workspace::{
    AccentPreset, AutoCheckRecord, BootstrapState, BuiltinCourseSummary, ChapterChatLog,
    ChapterProgress, ChapterStatus, ChatHistory, ChatMessage, ChatRole, CourseSourceRef,
    ExternalIdeKind, FileNode, GlobalConfigPatch, GlobalConfigView, LayoutSnapshot, Locale,
    ProgressState, RecentWorkspaceItem, StorageSchemaVersion, ThemeMode, WorkspaceMeta,
};
use schemars::{schema_for, JsonSchema};
use serde_json::{Map, Value};
use ts_rs::{Dummy, TS};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri parent directory")
        .to_path_buf()
}

fn write_file_atomically(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create contract output parent directory");
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .expect("contract output file name");
    let tmp_path = path.with_file_name(format!(".{file_name}.tmp.{}", std::process::id()));
    fs::write(&tmp_path, contents).expect("write temporary contract file");
    fs::rename(&tmp_path, path).expect("atomically replace contract file");
}

fn format_ts_decl<T: TS>() -> String {
    let normalized = T::decl().replace("\r\n", "\n");
    let kept: Vec<&str> = normalized
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("import ") && !trimmed.starts_with("// This file was generated")
        })
        .collect();
    let mut body = kept.join("\n").trim().to_owned();
    if body.starts_with("type ") || body.starts_with("interface ") {
        body = format!("export {body}");
    }
    if !body.ends_with(';') && !body.ends_with('}') {
        body.push(';');
    }
    body
}

fn canonicalize_json(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map
                .into_iter()
                .map(|(k, v)| (k, canonicalize_json(v)))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut sorted = Map::new();
            for (k, v) in entries {
                sorted.insert(k, v);
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize_json).collect()),
        other => other,
    }
}

fn render_schema<T: JsonSchema>() -> String {
    let root = schema_for!(T);
    let value = serde_json::to_value(&root).expect("serialize JSON schema");
    let canonical = canonicalize_json(value);
    let mut pretty = serde_json::to_string_pretty(&canonical).expect("pretty-print JSON schema");
    pretty.push('\n');
    pretty
}

#[test]
fn export_bindings() {
    let declarations: Vec<String> = vec![
        // IDs & primitives
        format_ts_decl::<SessionEpoch>(),
        format_ts_decl::<ChapterId>(),
        format_ts_decl::<TerminalId>(),
        format_ts_decl::<CommandBlockId>(),
        format_ts_decl::<RunId>(),
        format_ts_decl::<RiskConfirmationToken>(),
        format_ts_decl::<RelWorkspacePath>(),
        // Errors
        format_ts_decl::<ErrorCode>(),
        format_ts_decl::<AppError>(),
        // Runtime
        format_ts_decl::<Language>(),
        format_ts_decl::<PreflightStatus>(),
        format_ts_decl::<PreflightCheckItem>(),
        format_ts_decl::<PreflightReport>(),
        format_ts_decl::<ManagedRuntimeTarget>(),
        // PTY
        format_ts_decl::<PtyRole>(),
        format_ts_decl::<OneshotIdleState>(),
        format_ts_decl::<RiskRuleMatch>(),
        format_ts_decl::<RiskAssessment>(),
        // Grading
        format_ts_decl::<ExpectedActualDiff>(),
        format_ts_decl::<CheckResult>(),
        format_ts_decl::<GradeRunResult>(),
        // LSP
        format_ts_decl::<LspLifecycleState>(),
        format_ts_decl::<DiagnosticSeverity>(),
        format_ts_decl::<DiagnosticItem>(),
        // AI & ACP
        format_ts_decl::<TeachingMode>(),
        format_ts_decl::<SelectionSource>(),
        format_ts_decl::<ContextChipKind>(),
        format_ts_decl::<ContextChip>(),
        format_ts_decl::<ToolCallStatus>(),
        format_ts_decl::<AcpOperationKind>(),
        format_ts_decl::<AcpPermissionOption>(),
        format_ts_decl::<AcpDecision>(),
        format_ts_decl::<FinishReason>(),
        format_ts_decl::<AiStreamEvent>(),
        // Broadcast events
        format_ts_decl::<SessionEventEnvelope<Dummy>>(),
        format_ts_decl::<FsDelta>(),
        format_ts_decl::<FsDeltaEvent>(),
        format_ts_decl::<StorageCorruptedEvent>(),
        format_ts_decl::<PtyStateEvent>(),
        format_ts_decl::<PortConflictEvent>(),
        format_ts_decl::<LspStatusEvent>(),
        format_ts_decl::<LspDiagnosticsEvent>(),
        // Workspace & global config
        format_ts_decl::<StorageSchemaVersion>(),
        format_ts_decl::<Locale>(),
        format_ts_decl::<ThemeMode>(),
        format_ts_decl::<AccentPreset>(),
        format_ts_decl::<ExternalIdeKind>(),
        format_ts_decl::<CourseSourceRef>(),
        format_ts_decl::<LayoutSnapshot>(),
        format_ts_decl::<WorkspaceMeta>(),
        format_ts_decl::<ChapterStatus>(),
        format_ts_decl::<AutoCheckRecord>(),
        format_ts_decl::<ChapterProgress>(),
        format_ts_decl::<ProgressState>(),
        format_ts_decl::<FileNode>(),
        format_ts_decl::<ChatRole>(),
        format_ts_decl::<ChatMessage>(),
        format_ts_decl::<ChapterChatLog>(),
        format_ts_decl::<ChatHistory>(),
        format_ts_decl::<GlobalConfigView>(),
        format_ts_decl::<GlobalConfigPatch>(),
        format_ts_decl::<RecentWorkspaceItem>(),
        format_ts_decl::<BuiltinCourseSummary>(),
        format_ts_decl::<BootstrapState>(),
        // Course & checks
        format_ts_decl::<PackSchemaVersion>(),
        format_ts_decl::<CourseChapterRef>(),
        format_ts_decl::<CourseManifest>(),
        format_ts_decl::<DaemonRequirement>(),
        format_ts_decl::<HttpRequestSpec>(),
        format_ts_decl::<HttpExpectSpec>(),
        format_ts_decl::<CheckSpec>(),
        format_ts_decl::<ChapterChecksSpec>(),
        format_ts_decl::<MultiChapterChecksSpec>(),
        format_ts_decl::<SingleChapterChecksSpec>(),
        format_ts_decl::<ChecksSpec>(),
        format_ts_decl::<MarkdownNode>(),
        format_ts_decl::<CommandBlock>(),
        format_ts_decl::<ParsedChapter>(),
        format_ts_decl::<ParsedCourse>(),
        // Metrics
        format_ts_decl::<MetricEventName>(),
        format_ts_decl::<MetricEvent>(),
        format_ts_decl::<MetricValue>(),
        format_ts_decl::<MetricRecord>(),
        format_ts_decl::<MetricsLog>(),
    ];

    let mut seen_names = BTreeSet::new();
    for decl in &declarations {
        let header_line = decl.lines().next().expect("non-empty TS declaration");
        let rest = header_line
            .strip_prefix("export type ")
            .or_else(|| header_line.strip_prefix("export interface "))
            .expect("declaration must start with export type or export interface");
        let name = rest
            .split([' ', '<', '='])
            .next()
            .expect("declaration type name");
        assert!(
            seen_names.insert(name.to_owned()),
            "duplicate exported TypeScript type: {name}"
        );
    }

    let mut output =
        String::from("// AUTO-GENERATED BY cargo test export_bindings. DO NOT EDIT.\n\n");
    output.push_str(&declarations.join("\n\n"));
    output.push('\n');

    let target = repo_root().join("src/lib/ipc/generated.ts");
    write_file_atomically(&target, &output);
}

#[test]
fn export_schemas() {
    let schemas_dir = repo_root().join("schemas");
    let exports: [(&str, String); 6] = [
        (
            "codelab-pack.v1.schema.json",
            render_schema::<CourseManifest>(),
        ),
        (
            "codelab-checks.v1.schema.json",
            render_schema::<ChecksSpec>(),
        ),
        (
            "workspace-meta.v1.schema.json",
            render_schema::<WorkspaceMeta>(),
        ),
        (
            "workspace-progress.v1.schema.json",
            render_schema::<ProgressState>(),
        ),
        (
            "workspace-metrics.v1.schema.json",
            render_schema::<MetricsLog>(),
        ),
        (
            "workspace-chat.v1.schema.json",
            render_schema::<ChatHistory>(),
        ),
    ];

    for (file_name, contents) in exports {
        write_file_atomically(&schemas_dir.join(file_name), &contents);
    }
}

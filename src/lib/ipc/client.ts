import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AuthorPreviewResult,
  BootstrapState,
  CourseInspectResult,
  CourseSourceInput,
  GlobalConfigPatch,
  GlobalConfigView,
  MetricEvent,
  OpenWorkspaceReq,
  OpenedWorkspaceView,
  ParsedCourse,
  SessionEpoch,
} from "./generated";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function appBootstrap(): Promise<BootstrapState> {
  return invoke<BootstrapState>("app_bootstrap");
}

export async function pickMarkdownFile(title: string): Promise<string | null> {
  return open({
    multiple: false,
    directory: false,
    title,
    filters: [{ name: "Markdown", extensions: ["md", "markdown", "mdown", "mkd"] }],
  });
}

export async function courseInspectSource(
  source: CourseSourceInput,
): Promise<CourseInspectResult> {
  return invoke<CourseInspectResult>("course_inspect_source", { source });
}

export async function courseTrustAndOpen(
  req: OpenWorkspaceReq,
): Promise<OpenedWorkspaceView> {
  return invoke<OpenedWorkspaceView>("course_trust_and_open", { req });
}

export async function courseReloadManual(
  epoch: SessionEpoch,
): Promise<ParsedCourse> {
  return invoke<ParsedCourse>("course_reload_manual", { epoch });
}

export async function courseAuthorPreview(
  markdown: string,
  checksJson: string | null = null,
): Promise<AuthorPreviewResult> {
  return invoke<AuthorPreviewResult>("course_author_preview", {
    markdown,
    checksJson,
  });
}

export async function courseAuthorSave(
  packDir: string,
  markdown: string,
  checksJson: string,
): Promise<ParsedCourse> {
  return invoke<ParsedCourse>("course_author_save", {
    packDir,
    markdown,
    checksJson,
  });
}

export async function settingsUpdate(patch: GlobalConfigPatch): Promise<GlobalConfigView> {
  return invoke<GlobalConfigView>("settings_update", { patch });
}

export async function metricsRecord(
  epoch: SessionEpoch | null,
  event: MetricEvent,
): Promise<void> {
  return invoke<void>("metrics_record", { epoch, event });
}

export async function workspaceClose(epoch: SessionEpoch): Promise<void> {
  return invoke<void>("workspace_close", { epoch });
}

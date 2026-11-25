import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it } from "vitest";
import { App } from "../App";
import type { AppCommand, AppResponse, LoadedCourse } from "../core/contracts";
import { createBuiltinGoCourseFixture, FakeCoreClient } from "./fakeCoreClient";

const repo = "https://example.test/team/course";
const draftPath = "/author/pack";

function pack(source: LoadedCourse["source"], chapterTitle: string): LoadedCourse {
  const course = createBuiltinGoCourseFixture();
  return {
    ...course, source, title: "A Course Pack", isBuiltin: false, trusted: false,
    chapters: course.chapters.slice(2).map((chapter) => ({ ...chapter, title: chapterTitle })),
    totalExercises: 1,
  };
}

class DraftClient extends FakeCoreClient {
  private course = pack({ kind: "localPack", path: draftPath }, "Original exercise");
  private existing = false;
  private trusted = false;

  override async dispatch(command: AppCommand): Promise<AppResponse> {
    switch (command.type) {
      case "createCourseDraft":
      case "openCourseDraft":
        this.existing = command.type === "openCourseDraft";
        this.commandLog.push(command);
        this.writeDiskFile(draftPath, "README.md", "Original exercise");
        this.writeDiskFile(draftPath, ".codelab-checks.json", "valid");
        return { type: "courseDraft", payload: {
          id: 41, root: draftPath, files: ["README.md", ".codelab-checks.json"],
          preview: { course: this.course, diagnostics: [] },
          requiresRegistryTrust: this.existing,
        } };
      case "readCourseDraftFile":
        return { type: "workspaceFile", payload: this.readDiskFile(draftPath, command.payload.path)! };
      case "previewCourseDraft": {
        const manual = command.payload.edits.find((edit) => edit.path === "README.md")?.text;
        const checks = command.payload.edits.find((edit) => edit.path === ".codelab-checks.json")?.text;
        return { type: "draftPreview", payload: {
          course: manual ? pack(this.course.source, manual) : this.course,
          diagnostics: checks === "invalid" ? [{
            path: ".codelab-checks.json",
            error: { code: "COURSE_CHECKS_INVALID", details: { field: "chapters[0].checks[0].type", reasonCode: "UNSUPPORTED_CHECK_TYPE" } },
          }] : [],
        } };
      }
      case "writeCourseDraftFile":
        this.commandLog.push(command);
        return { type: "workspaceFile", payload: this.writeDiskFile(draftPath, command.payload.path, command.payload.text) };
      case "startTrialWorkspace":
        this.commandLog.push(command);
        if (this.existing && !this.trusted) {
          return { type: "trialTrustRequired", payload: { sourceKind: "localPath", source: draftPath } };
        }
        return { type: "trialWorkspaceOpened", payload: {
          workspace: { id: 71, root: "/trial/isolated", recoveryNotices: [] },
          course: { ...pack(this.course.source, "Revised exercise"), trusted: true },
        } };
      case "grantTrust":
        this.trusted = command.payload.source === draftPath;
        return super.dispatch(command);
      default:
        return super.dispatch(command);
    }
  }
}

beforeEach(() => window.localStorage.clear());

it("opens Git in preview, grants trust by URL, and refreshes in the same Workspace", async () => {
  const source = { kind: "gitPack", url: repo } as const;
  const client = new FakeCoreClient({
    settings: { locale: "en-US" },
    localCourses: { [repo]: pack(source, "Original exercise") },
    pickedPaths: { workspaceDir: "/learner" },
  });
  const user = userEvent.setup();
  render(<App client={client} />);

  await user.click(await screen.findByRole("button", { name: /Import Local Markdown/ }));
  await user.click(screen.getByRole("button", { name: "Git repository" }));
  await user.type(screen.getByLabelText("Git repository URL"), repo);
  await user.click(screen.getByRole("button", { name: "Open Read-Only Manual" }));
  expect(await screen.findByRole("heading", { name: "Original exercise" })).toBeVisible();
  expect(screen.queryByRole("button", { name: "Choose Workspace" })).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Trust this Course" }));
  await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Trust this Course" }));
  expect(client.commandLog).toContainEqual({ type: "grantTrust", payload: { sourceKind: "gitRepo", source: repo } });
  await user.click(await screen.findByRole("button", { name: "Choose Workspace" }));
  expect(await screen.findByRole("button", { name: "Change Workspace" })).toBeVisible();

  client.registerLocalCourse(repo, pack(source, "Upstream exercise"));
  await user.click(screen.getByRole("button", { name: "Pull upstream updates" }));
  expect(await screen.findByRole("heading", { name: "Upstream exercise" })).toBeVisible();
  expect(screen.getByRole("button", { name: "Change Workspace" })).toBeVisible();
});

it("prompts before trying an existing Course Pack and retries only after trust is granted", async () => {
  const client = new DraftClient({ settings: { locale: "en-US" }, pickedPaths: { coursePackDir: draftPath } });
  const user = userEvent.setup();
  render(<App client={client} />);
  await user.click(await screen.findByRole("button", { name: /Import Local Markdown/ }));
  await user.click(screen.getByRole("button", { name: "Author Course Pack" }));
  await user.click(screen.getByRole("button", { name: "Browse folder" }));
  await user.click(screen.getByRole("button", { name: "Open existing draft" }));
  const start = await screen.findByRole("button", { name: "Start trial" });
  await waitFor(() => expect(start).toBeEnabled());
  await user.click(start);
  let dialog = await screen.findByRole("dialog", { name: "Trust this local Course?" });
  expect(within(dialog).getByText(draftPath)).toBeVisible();
  expect(client.commandLog.some(({ type }) => type === "grantTrust")).toBe(false);
  await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
  expect(screen.queryByRole("button", { name: "Return to draft" })).not.toBeInTheDocument();
  await user.click(start);
  dialog = await screen.findByRole("dialog", { name: "Trust this local Course?" });
  await user.click(within(dialog).getByRole("button", { name: "Trust this Course" }));
  expect(await screen.findByRole("button", { name: "Return to draft" })).toBeVisible();
  expect(client.commandLog.filter(({ type }) => type === "grantTrust" || type === "startTrialWorkspace").slice(-2)).toEqual([
    { type: "grantTrust", payload: { sourceKind: "localPath", source: draftPath } },
    { type: "startTrialWorkspace", payload: { draftId: 41 } },
  ]);
});

it("creates a draft, previews unsaved Markdown and Schema errors, and saves before trial", async () => {
  const client = new DraftClient({ settings: { locale: "en-US" }, pickedPaths: { coursePackDir: draftPath } });
  const user = userEvent.setup();
  render(<App client={client} />);
  await user.click(await screen.findByRole("button", { name: /Import Local Markdown/ }));
  await user.click(screen.getByRole("button", { name: "Author Course Pack" }));
  await user.type(screen.getByLabelText("Course name"), "Draft Course");
  await user.selectOptions(screen.getByLabelText("Language"), "Python");
  await user.click(screen.getByRole("button", { name: "Browse folder" }));
  await user.click(screen.getByRole("button", { name: "Create draft" }));
  const manual = await screen.findByRole("textbox", { name: "Source editor" });
  fireEvent.change(manual, { target: { value: "Revised exercise" } });
  expect(await screen.findByRole("heading", { name: "Revised exercise" })).toBeVisible();

  await user.selectOptions(screen.getByLabelText("Source file"), ".codelab-checks.json");
  const checks = await screen.findByRole("textbox", { name: "Source editor" });
  fireEvent.change(checks, { target: { value: "invalid" } });
  expect(await screen.findByText("chapters[0].checks[0].type")).toBeVisible();
  expect(screen.getByRole("button", { name: "Start trial" })).toBeDisabled();
  fireEvent.change(checks, { target: { value: "valid" } });

  const start = screen.getByRole("button", { name: "Start trial" });
  await waitFor(() => expect(start).toBeEnabled());
  await user.click(start);
  expect(await screen.findByRole("button", { name: "Return to draft" })).toBeVisible();
  expect(client.readDiskFile(draftPath, "README.md")).toMatchObject({ kind: "text", text: "Revised exercise" });
  expect(client.commandLog).toContainEqual({ type: "startTrialWorkspace", payload: { draftId: 41 } });
  await user.click(screen.getByRole("button", { name: "Return to draft" }));
  expect(await screen.findByRole("textbox", { name: "Source editor" })).toBeVisible();
});

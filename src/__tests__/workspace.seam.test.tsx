import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it } from "vitest";
import { App } from "../App";
import { DEFAULT_SETTINGS } from "../core/contracts";
import { createBuiltinGoCourseFixture, FakeCoreClient } from "./fakeCoreClient";

beforeEach(() => localStorage.clear());

const firstRoot = "/work/first";
const secondRoot = "/work/second";

async function openWorkspace() {
  const client = new FakeCoreClient({
    settings: { ...DEFAULT_SETTINGS, locale: "en-US" },
    pickedPaths: { workspaceDir: firstRoot },
  });
  client.writeDiskFile(firstRoot, "main.go", "package main\n");
  client.writeDiskFile(firstRoot, "other.go", "package other\n");
  const user = userEvent.setup();
  render(<App client={client} />);
  await user.click(await screen.findByRole("button", { name: "Start Built-in Go / GoFrame Lab" }));
  await user.click(await screen.findByRole("button", { name: "Choose Workspace" }));
  const files = await screen.findByRole("button", { name: "Files" });
  expect(files).toHaveAttribute("aria-expanded", "false");
  await user.click(files);
  await user.click(await screen.findByRole("button", { name: "main.go" }));
  const editor = await screen.findByRole("textbox", { name: "Edit main.go" });
  return { client, user, editor };
}

it("refreshes a clean file and requires a decision before replacing an edited draft", async () => {
  const { client, user, editor } = await openWorkspace();
  act(() => client.writeDiskFile(firstRoot, "main.go", "// external\n"));
  await waitFor(() => expect(editor).toHaveValue("// external\n"));
  expect(screen.queryByText("Changed on disk")).not.toBeInTheDocument();

  fireEvent.change(editor, { target: { value: "// my draft\n" } });
  act(() => client.writeDiskFile(firstRoot, "main.go", "// more external\n"));
  expect(await screen.findByText("Changed on disk")).toBeInTheDocument();
  expect(editor).toHaveValue("// my draft\n");
  await user.click(screen.getByRole("button", { name: "Keep draft" }));
  expect(screen.queryByText("Changed on disk")).not.toBeInTheDocument();
  expect(editor).toHaveValue("// my draft\n");
  fireEvent.keyDown(editor, { key: "s", ctrlKey: true });
  await screen.findByText("All changes saved");

  act(() => client.writeDiskFile(firstRoot, "main.go", "// later\n"));
  await waitFor(() => expect(editor).toHaveValue("// later\n"));
  fireEvent.change(editor, { target: { value: "// local again\n" } });
  act(() => client.writeDiskFile(firstRoot, "main.go", "// disk wins\n"));
  await user.click(await screen.findByRole("button", { name: "Load disk version" }));
  expect(editor).toHaveValue("// disk wins\n");
});

it("keeps drafts on cancel and saves to the original Workspace before switching", async () => {
  const { client, user, editor } = await openWorkspace();
  fireEvent.change(editor, { target: { value: "// save me\n" } });
  client.setPickedPath("workspaceDir", secondRoot);
  await user.click(screen.getByRole("button", { name: "Change Workspace" }));
  let dialog = await screen.findByRole("dialog");
  await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
  expect(editor).toHaveValue("// save me\n");

  await user.click(screen.getByRole("button", { name: "Change Workspace" }));
  dialog = await screen.findByRole("dialog");
  await user.click(within(dialog).getByRole("button", { name: "Save and switch" }));
  expect(await screen.findByText(secondRoot)).toBeInTheDocument();
  expect(client.commandLog.filter(({ type }) => type === "writeWorkspaceFile")).toEqual([
    { type: "writeWorkspaceFile", payload: { workspaceId: 1, path: "main.go", text: "// save me\n", expectedRevision: "1" } },
  ]);
});

it("keeps a local Course in preview until confirmation and locks it again on revocation", async () => {
  const source = "/courses/guide.md";
  const client = new FakeCoreClient({
    settings: { ...DEFAULT_SETTINGS, locale: "en-US" },
    pickedPaths: { workspaceDir: firstRoot },
    localCourses: {
      [source]: {
        ...createBuiltinGoCourseFixture(),
        courseId: "local-guide",
        title: "Local Guide",
        source: { kind: "localMarkdown", path: source },
        isBuiltin: false,
        trusted: false,
      },
    },
  });
  client.writeDiskFile(firstRoot, "hello.go", "package hello");
  const user = userEvent.setup();
  render(<App client={client} />);
  await user.click(await screen.findByRole("button", { name: "Import Local Markdown / Course Pack" }));
  await user.type(screen.getByLabelText("Local Markdown file path"), source);
  await user.click(screen.getByRole("button", { name: "Open Read-Only Manual" }));

  expect(await screen.findByText("Read-only preview")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Choose Workspace" })).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Trust this Course" }));
  const confirmation = await screen.findByRole("dialog", { name: "Trust this local Course?" });
  expect(client.commandLog.some(({ type }) => type === "grantTrust")).toBe(false);
  await user.click(within(confirmation).getByRole("button", { name: "Trust this Course" }));
  await user.click(await screen.findByRole("button", { name: "Choose Workspace" }));
  expect(await screen.findByRole("button", { name: "Open in IDE" })).toBeEnabled();

  await user.click(screen.getByRole("button", { name: "Revoke trust" }));
  expect(await screen.findByText("Read-only preview")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Open in IDE" })).not.toBeInTheDocument();
});

it("discards the draft only after a Workspace switch succeeds and reopens the original from recents", async () => {
  const { client, user, editor } = await openWorkspace();
  fireEvent.change(editor, { target: { value: "discard this" } });
  client.setPickedPath("workspaceDir", secondRoot);
  client.failNext("bindWorkspace", { code: "INVALID_WORKSPACE_PATH" });
  await user.click(screen.getByRole("button", { name: "Change Workspace" }));
  await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Discard changes" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  expect(editor).toHaveValue("discard this");

  await user.click(screen.getByRole("button", { name: "Change Workspace" }));
  await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Discard changes" }));
  expect(await screen.findByText(secondRoot)).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "CodeLab Studio" }));
  await user.click(await screen.findByRole("button", { name: /\/work\/first/ }));
  await user.click(await screen.findByRole("button", { name: "Files" }));
  await user.click(await screen.findByRole("button", { name: "main.go" }));
  expect(await screen.findByRole("textbox", { name: "Edit main.go" })).toHaveValue("package main\n");
});

it("shows a conflict for a background tab and keeps one tab when its path is created again", async () => {
  const { client, user, editor } = await openWorkspace();
  fireEvent.change(editor, { target: { value: "keep this draft" } });
  await user.click(screen.getByRole("button", { name: "other.go" }));
  act(() => client.deleteDiskFile(firstRoot, "main.go"));
  expect(await screen.findByText("Deleted on disk")).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "New file" }));
  await user.type(screen.getByRole("textbox", { name: "Path within Workspace" }), "main.go");
  await user.click(screen.getAllByRole("button", { name: "New file" })[1]);
  expect(await screen.findByRole("textbox", { name: "Edit main.go" })).toHaveValue("keep this draft");
  expect(screen.getAllByRole("tab", { name: /main.go/ })).toHaveLength(1);
  expect(await screen.findByText("Changed on disk")).toBeInTheDocument();
});

it("shows active language status, links to runtime settings, and keeps editing after degradation", async () => {
  const { client, user } = await openWorkspace();
  const syncs = () => client.commandLog.filter((command) => command.type === "syncLanguageDocument");
  expect(syncs()).toEqual([{
    type: "syncLanguageDocument",
    payload: { workspaceId: 1, version: 1, document: { path: "main.go", text: "package main\n" } },
  }]);
  const status = (path: string, version: number, state: "missing" | "ready" | "degraded") => {
    act(() => client.emitEvent({
      type: "languageStatusChanged",
      payload: { workspaceId: 1, path, version, language: "Go", status: state },
    }));
  };
  status("main.go", 1, "missing");
  await user.click(await screen.findByRole("button", { name: "Go LSP missing" }));
  const settings = await screen.findByRole("dialog");
  expect(within(settings).getByRole("tab", { name: "4-Language Runtimes" })).toHaveAttribute("aria-selected", "true");
  await user.click(within(settings).getByRole("button", { name: "Close" }));

  await user.click(screen.getByRole("button", { name: "other.go" }));
  expect(syncs().at(-1)?.payload.document?.path).toBe("other.go");
  status("main.go", 1, "ready");
  expect(screen.queryByRole("button", { name: "Go LSP ready" })).not.toBeInTheDocument();
  status("other.go", 2, "degraded");
  await screen.findByRole("button", { name: "Go LSP degraded" });
  const otherEditor = screen.getByRole("textbox", { name: "Edit other.go" });
  fireEvent.change(otherEditor, { target: { value: "package other\n// still editing" } });
  fireEvent.keyDown(otherEditor, { key: "s", ctrlKey: true });
  await screen.findByText("All changes saved");
  await user.click(screen.getByRole("button", { name: "Next Chapter" }));
  await user.click(screen.getAllByRole("button", { name: "Run command" })[0]);
  await waitFor(() => expect(client.commandLog.some(({ type }) => type === "runCommandBlock")).toBe(true));
  await user.click(screen.getByRole("button", { name: "Run checks" }));
  await waitFor(() => expect(client.commandLog.some(({ type }) => type === "runChecks")).toBe(true));
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

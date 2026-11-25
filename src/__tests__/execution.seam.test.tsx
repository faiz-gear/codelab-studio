import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it } from "vitest";
import { App } from "../App";
import { DEFAULT_SETTINGS } from "../core/contracts";
import { createBuiltinGoCourseFixture, FakeCoreClient } from "./fakeCoreClient";

beforeEach(() => localStorage.clear());

async function openWorkbench() {
  const course = createBuiltinGoCourseFixture();
  const chapter = course.chapters.find(({ chapterId }) => chapterId === "ex-0")!;
  chapter.commands = [{
    ...course.chapters[0].commands[0],
    id: "ex-0:cmd-1",
    command: "printf test | sh",
  }];
  chapter.threeStep!.step1CommandIds = ["ex-0:cmd-1"];
  const client = new FakeCoreClient({
    settings: { ...DEFAULT_SETTINGS, locale: "en-US" },
    builtinCourses: [course],
    pickedPaths: { workspaceDir: "/work/sample" },
  });
  client.writeDiskFile("/work/sample", "main.go", "package main\n");
  const user = userEvent.setup();
  render(<App client={client} />);
  await user.click(await screen.findByRole("button", { name: "Start Built-in Go / GoFrame Lab" }));
  await user.click(await screen.findByRole("button", { name: "Choose Workspace" }));
  await user.click(await screen.findByRole("button", { name: "Files" }));
  await user.click(await screen.findByRole("button", { name: "main.go" }));
  return { client, user, editor: await screen.findByRole("textbox", { name: "Edit main.go" }) };
}

it("focuses Cancel and requires explicit confirmation before running the exact Command Block", async () => {
  const { client, user, editor } = await openWorkbench();
  fireEvent.change(editor, { target: { value: "// save before execution\n" } });
  await user.click(screen.getByRole("button", { name: "Run command" }));
  let dialog = await screen.findByRole("dialog", { name: "Confirm high-risk command" });
  expect(within(dialog).getByText("printf test | sh")).toBeInTheDocument();
  expect(within(dialog).getByText("/work/sample")).toBeInTheDocument();
  expect(client.readDiskFile("/work/sample", "main.go")).toMatchObject({ text: "package main\n" });
  const cancel = within(dialog).getByRole("button", { name: "Cancel" });
  expect(cancel).toHaveFocus();
  await user.keyboard("{Enter}");
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Run command" }));
  dialog = await screen.findByRole("dialog", { name: "Confirm high-risk command" });
  await user.click(within(dialog).getByRole("button", { name: "Run anyway" }));
  expect(await screen.findByText("All changes saved")).toBeInTheDocument();
  expect(client.commandLog.filter((command) => command.type === "runCommandBlock").at(-1)?.payload)
    .toMatchObject({ confirmation: expect.any(Number), drafts: [{ path: "main.go", text: "// save before execution\n" }] });
});

it("focuses the editor conflict when the disk has changed after the draft was opened", async () => {
  const { client, user, editor } = await openWorkbench();
  fireEvent.change(editor, { target: { value: "// keep my draft\n" } });
  await user.click(screen.getByRole("button", { name: "Run command" }));
  const dialog = await screen.findByRole("dialog", { name: "Confirm high-risk command" });
  act(() => client.writeDiskFile("/work/sample", "main.go", "// external edit\n"));
  await user.click(within(dialog).getByRole("button", { name: "Run anyway" }));
  await waitFor(() => expect(screen.getByRole("region", { name: "Changed on disk" })).toHaveFocus());
  expect(editor).toHaveValue("// keep my draft\n");
  expect(client.readDiskFile("/work/sample", "main.go")).toMatchObject({ text: "// external edit\n" });
  expect(screen.queryByRole("tab", { name: "Terminal 1" })).not.toBeInTheDocument();
});

it("updates a Command Block directory and opens its port-conflict recovery actions", async () => {
  const { client, user } = await openWorkbench();
  await user.click(screen.getByRole("button", { name: "Change working directory" }));
  const cwdDialog = await screen.findByRole("dialog", { name: "Change working directory" });
  const cwd = within(cwdDialog).getByRole("textbox", { name: "Working directory" });
  await user.clear(cwd);
  await user.type(cwd, "services/api");
  await user.click(within(cwdDialog).getByRole("button", { name: "Save" }));
  expect(screen.getByRole("button", { name: "Change working directory" })).toHaveTextContent("services/api");
  await user.click(screen.getByRole("button", { name: "Run command" }));
  const confirmation = await screen.findByRole("dialog", { name: "Confirm high-risk command" });
  expect(within(confirmation).getByText("/work/sample/services/api")).toBeInTheDocument();
  const lastRun = client.commandLog.filter(({ type }) => type === "runCommandBlock").at(-1);
  if (lastRun?.type !== "runCommandBlock") throw new Error("Expected Command Block request");
  await user.click(within(confirmation).getByRole("button", { name: "Run anyway" }));
  await screen.findByRole("tab", { name: "Terminal 1" });
  const output = await client.dispatch({ type: "readTerminals", payload: { workspaceId: lastRun.payload.workspaceId } });
  if (output.type !== "terminals") throw new Error("Expected terminal list");
  act(() => client.markPortConflict(output.payload.terminals[0].id, "ERROR: address already in use"));
  const recovery = await screen.findByRole("region", { name: "A local port is already in use." });
  await user.click(within(recovery).getByRole("button", { name: "Ask AI" }));
  expect(within(await screen.findByRole("dialog", { name: "Ask AI" })).getByText("ERROR: address already in use")).toBeInTheDocument();
});

import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it } from "vitest";
import { App } from "../App";
import { FakeCoreClient } from "./fakeCoreClient";

beforeEach(() => localStorage.clear());

const root = "/work/solutions";

async function openWorkspace() {
  const client = new FakeCoreClient({
    settings: { locale: "en-US" },
    pickedPaths: { workspaceDir: root },
    chapterSolutions: { "ex-0": { "main.go": "// reference", "nested/extra.go": "// extra" } },
    starterFiles: { "main.go": "// starter", "main_test.go": "// visible tests" },
  });
  client.writeDiskFile(root, "main.go", "// learner");
  const user = userEvent.setup();
  render(<App client={client} />);
  await user.click(await screen.findByRole("button", { name: "Start Built-in Go / GoFrame Lab" }));
  await user.click(await screen.findByRole("button", { name: "Choose Workspace" }));
  await user.click(await screen.findByRole("button", { name: "Files" }));
  await user.click(await screen.findByRole("button", { name: "main.go" }));
  return { client, user, editor: await screen.findByRole("textbox", { name: "Edit main.go" }) };
}

it("reviews a draft and applies only the selected Reference Solution file after confirmation", async () => {
  const { client, user, editor } = await openWorkspace();
  fireEvent.change(editor, { target: { value: "// learner draft" } });
  const trigger = await screen.findByRole("button", { name: "View reference hints and diff" });
  await user.click(trigger);
  let dialog = within(await screen.findByRole("dialog", { name: "Reference Solution" }));
  expect(dialog.getByRole("button", { name: "Cancel" })).toHaveFocus();
  expect(await dialog.findByText("// learner draft")).toBeInTheDocument();
  expect(dialog.getByText("// reference")).toBeInTheDocument();
  expect(client.readDiskFile(root, "main.go")).toMatchObject({ text: "// learner" });
  await user.click(dialog.getByRole("button", { name: "Cancel" }));
  expect(editor).toHaveValue("// learner draft");
  expect(trigger).toHaveFocus();

  await user.click(trigger);
  dialog = within(await screen.findByRole("dialog", { name: "Reference Solution" }));
  const files = await dialog.findByRole("combobox", { name: "File to compare" });
  await user.click(files);
  await user.click(dialog.getByRole("option", { name: "nested/extra.go" }));
  expect(dialog.getByText("// extra")).toBeInTheDocument();
  await user.click(files);
  await user.click(dialog.getByRole("option", { name: "main.go" }));
  await user.click(dialog.getByRole("button", { name: "Apply Reference Solution" }));
  await waitFor(() => expect(editor).toHaveValue("// reference"));
  expect(client.readDiskFile(root, "main.go")).toMatchObject({ text: "// reference" });
  expect(client.readDiskFile(root, "nested/extra.go")).toBeUndefined();
});

it("restores a deleted test file from Starter and opens the restored text in the editor", async () => {
  const { client, user } = await openWorkspace();
  await user.click(screen.getByRole("button", { name: "Compare with Starter" }));
  const dialog = within(await screen.findByRole("dialog", { name: "Restore from Starter" }));
  const picker = await dialog.findByRole("combobox", { name: "File to compare" });
  await user.click(picker);
  await user.click(dialog.getByRole("option", { name: "main_test.go" }));
  expect(await dialog.findByText("File is missing")).toBeInTheDocument();
  expect(dialog.getByText("// visible tests")).toBeInTheDocument();
  expect(client.readDiskFile(root, "main_test.go")).toBeUndefined();
  await user.click(dialog.getByRole("button", { name: "Restore this file" }));
  expect(await screen.findByRole("textbox", { name: "Edit main_test.go" })).toHaveValue("// visible tests");
  expect(client.readDiskFile(root, "main.go")).toMatchObject({ text: "// learner" });
});

it("preserves a draft and routes an external write to the editor conflict controls", async () => {
  const { client, user, editor } = await openWorkspace();
  fireEvent.change(editor, { target: { value: "// my draft" } });
  await user.click(screen.getByRole("button", { name: "View reference hints and diff" }));
  const dialog = within(await screen.findByRole("dialog", { name: "Reference Solution" }));
  expect(await dialog.findByText("// my draft")).toBeInTheDocument();
  act(() => client.writeDiskFile(root, "main.go", "// external save"));
  await user.click(dialog.getByRole("button", { name: "Apply Reference Solution" }));

  expect(await screen.findByRole("button", { name: "Load disk version" })).toBeInTheDocument();
  expect(editor).toHaveValue("// my draft");
  expect(client.readDiskFile(root, "main.go")).toMatchObject({ text: "// external save" });
  await user.click(screen.getByRole("button", { name: "Keep draft" }));
  expect(editor).toHaveValue("// my draft");
});

import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { App } from "../App";
import { createBuiltinGoCourseFixture, FakeCoreClient } from "./fakeCoreClient";

beforeEach(() => {
  window.localStorage.clear();
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    writable: true,
    value: 1440,
  });
  Object.defineProperties(HTMLDialogElement.prototype, {
    showModal: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.setAttribute("open", "");
      },
    },
    close: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.removeAttribute("open");
      },
    },
  });
});

async function openBuiltinWithWorkspace(client: FakeCoreClient, user: ReturnType<typeof userEvent.setup>) {
  client.setPickedPath("workspaceDir", "/work/go-lab");
  client.writeDiskFile("/work/go-lab", "task-api/main.go", "package main\n// original\n");
  render(<App client={client} />);
  await screen.findByTestId("course-grid");
  await user.click(
    within(screen.getByTestId("course-grid")).getByRole("button", {
      name: /Go/,
    })
  );
  const chooseBtn = await screen.findByRole("button", {
    name: /Choose Workspace|选择 Workspace/,
  });
  await user.click(chooseBtn);
  await screen.findByText("/work/go-lab");
}

describe("Pure ACP Tutor Session frontend seam (CoreClient)", () => {
  it("configures pure ACP presets, Permission Policy, and handshake without BYOK or Keychain forms", async () => {
    const user = userEvent.setup();
    const client = new FakeCoreClient({ settings: { locale: "en-US" } });
    render(<App client={client} />);
    await screen.findByRole("heading", { level: 1 });

    await user.click(screen.getByRole("button", { name: "Settings" }));
    const dialog = within(screen.getByRole("dialog", { name: "Settings" }));
    await user.click(dialog.getByRole("tab", { name: "ACP Tutor" }));

    const panel = within(dialog.getByTestId("tutor-settings-panel"));
    // ADR-0004: strictly no BYOK or API Key / Keychain UI in v1.
    expect(panel.queryByText(/BYOK|API Key|Keychain/i)).toBeNull();
    expect(dialog.queryByRole("tab", { name: /BYOK/i })).toBeNull();

    const presetSelect = panel.getByLabelText("Local ACP Agent preset");
    expect(within(presetSelect).getAllByRole("option")).toHaveLength(5);

    await user.selectOptions(presetSelect, "geminiCli");
    expect(panel.getByLabelText("Launch command")).toHaveValue("gemini --acp");
    expect(client.commandLog).toContainEqual({
      type: "updateSettings",
      payload: { tutor: { preset: "geminiCli", command: "gemini --acp" } },
    });

    await user.selectOptions(presetSelect, "custom");
    const cmdInput = panel.getByLabelText("Launch command");
    await user.clear(cmdInput);
    await user.type(cmdInput, "my-acp-cli --acp");
    await user.click(panel.getByRole("button", { name: "Test ACP Handshake" }));

    expect(client.commandLog).toContainEqual({
      type: "testTutorHandshake",
      payload: { preset: "custom", command: "my-acp-cli --acp" },
    });
    expect(await panel.findByRole("status")).toHaveTextContent(
      /ACP handshake succeeded/
    );

    // Switch Permission Policy to strict_ask_all.
    const strictRadio = panel.getByRole("radio", { name: /Strict Ask-All/ });
    await user.click(strictRadio);
    expect(client.commandLog).toContainEqual({
      type: "updateSettings",
      payload: { tutor: { permissionPolicy: "strict_ask_all" } },
    });

    // Handshake failure renders localized alert.
    client.failNext("testTutorHandshake", { code: "ACP_HANDSHAKE_FAILED" });
    await user.click(panel.getByRole("button", { name: "Test ACP Handshake" }));
    expect(await panel.findByRole("alert")).toHaveTextContent(
      /ACP handshake failed/
    );
  });

  it("blocks Tutor startup on untrusted Courses, toggles docked/overlay layout, and returns focus on close", async () => {
    const user = userEvent.setup();
    const untrustedCourse = {
      ...createBuiltinGoCourseFixture(),
      courseId: "local-untrusted",
      title: "Untrusted Local Guide",
      source: { kind: "localMarkdown" as const, path: "/tmp/untrusted.md" },
      isBuiltin: false,
      trusted: false,
    };
    const client = new FakeCoreClient({
      settings: { locale: "en-US" },
      localCourses: { "/tmp/untrusted.md": untrustedCourse },
    });
    render(<App client={client} />);
    await screen.findByRole("heading", { level: 1 });

    // Open untrusted course via Import modal.
    await user.click(
      screen.getByRole("button", { name: "Import Local Markdown / Course Pack" })
    );
    const modal = within(screen.getByRole("dialog"));
    await user.type(modal.getByPlaceholderText("/path/to/handbook.md"), "/tmp/untrusted.md");
    await user.click(modal.getByRole("button", { name: "Open Read-Only Manual" }));
    await screen.findByTestId("course-workbench");

    // Drawer is closed by default.
    expect(screen.queryByTestId("tutor-drawer")).toBeNull();

    const tutorTrigger = screen.getByRole("button", { name: "AI Tutor" });
    await user.click(tutorTrigger);
    const drawer = screen.getByTestId("tutor-drawer");
    expect(drawer).toHaveAttribute("data-layout", "docked");
    expect(within(drawer).getByTestId("tutor-untrusted-notice")).toBeInTheDocument();
    expect(
      client.commandLog.some(
        (cmd) => cmd.type === "getTutorSession" || cmd.type === "sendTutorPrompt"
      )
    ).toBe(false);

    // Resize below 1280px -> frosted floating overlay layout.
    act(() => {
      window.innerWidth = 1120;
      window.dispatchEvent(new Event("resize"));
    });
    expect(screen.getByTestId("tutor-drawer")).toHaveAttribute("data-layout", "overlay");

    // Closing drawer returns focus to TopBar AI Tutor trigger button.
    await user.click(within(drawer).getByRole("button", { name: "Close AI Tutor drawer" }));
    await act(async () => {});
    expect(screen.queryByTestId("tutor-drawer")).toBeNull();
    expect(tutorTrigger).toHaveFocus();
  });

  it("handles streaming, stop generation, style switching, permission cards, crash retry, and clear", async () => {
    const user = userEvent.setup();
    const client = new FakeCoreClient({ settings: { locale: "en-US" } });
    await openBuiltinWithWorkspace(client, user);

    await user.click(screen.getByRole("button", { name: "AI Tutor" }));
    const drawer = within(screen.getByTestId("tutor-drawer"));

    // Switch teaching style from default Socratic to Direct.
    await user.click(drawer.getByRole("button", { name: "Direct Answer" }));
    expect(client.commandLog).toContainEqual({
      type: "updateSettings",
      payload: { tutor: { teachingStyle: "direct" } },
    });

    // Send prompt & stream chunks.
    const input = drawer.getByLabelText("Ask the local ACP Tutor");
    await user.type(input, "Check why my server fails");
    await user.click(drawer.getByRole("button", { name: "Send" }));

    act(() => {
      client.emitTutorChunk(1, "Let me inspect ");
      client.emitTutorChunk(1, "task-api/main.go.");
    });
    expect(drawer.getByText("Let me inspect task-api/main.go.")).toBeInTheDocument();

    // Auto-allowed workspace read, blocked sensitive file, and high-risk command approval card.
    act(() => {
      client.emitTutorPermission(1, {
        id: "perm-auto",
        kind: "read",
        title: "Read main.go",
        targetPath: "task-api/main.go",
        command: null,
        outsideWorkspace: false,
        sensitive: false,
        highRisk: false,
        status: "autoAllowed",
      });
      client.emitTutorPermission(1, {
        id: "perm-secret",
        kind: "read",
        title: "Read .env",
        targetPath: ".env",
        command: null,
        outsideWorkspace: false,
        sensitive: true,
        highRisk: false,
        status: "blockedSensitive",
      });
      client.emitTutorPermission(1, {
        id: "perm-risk",
        kind: "execute",
        title: "Run destructive command",
        targetPath: null,
        command: "rm -rf /tmp/demo",
        outsideWorkspace: false,
        sensitive: false,
        highRisk: true,
        status: "pending",
      });
    });

    expect(drawer.getByTestId("tutor-perm-perm-auto")).toHaveTextContent(
      /Auto-allowed/
    );
    expect(drawer.getByTestId("tutor-perm-perm-secret")).toHaveTextContent(
      /Matched sensitive file protection/
    );
    const riskCard = drawer.getByTestId("tutor-perm-perm-risk");
    expect(riskCard).toHaveAttribute("data-high-risk", "true");
    expect(within(riskCard).getByRole("alert")).toHaveTextContent(
      /High-risk command warning/
    );

    // Deny the high-risk command.
    await user.click(within(riskCard).getByRole("button", { name: "Deny" }));
    expect(client.commandLog).toContainEqual({
      type: "respondTutorPermission",
      payload: { workspaceId: 1, requestId: "perm-risk", allow: false },
    });

    // Stop generation mid-stream.
    await user.click(drawer.getByRole("button", { name: "Stop generating" }));
    expect(client.commandLog).toContainEqual({
      type: "stopTutorTurn",
      payload: { workspaceId: 1 },
    });
    expect(drawer.getByText("Generation stopped")).toBeInTheDocument();

    // Send another prompt and simulate ACP crash -> Retry recovers.
    await user.type(input, "Try again");
    await user.click(drawer.getByRole("button", { name: "Send" }));
    act(() => {
      client.emitTutorCrash(1);
    });
    expect(
      drawer
        .getAllByRole("alert")
        .some((el) => /exited unexpectedly/.test(el.textContent ?? ""))
    ).toBe(true);
    await user.click(drawer.getByRole("button", { name: "Retry" }));
    expect(client.commandLog).toContainEqual({
      type: "retryTutorTurn",
      payload: { workspaceId: 1 },
    });
    act(() => {
      client.emitTutorChunk(1, "Recovered response");
      client.emitTutorTurnCompleted(1);
    });
    expect(drawer.getByText("Recovered response")).toBeInTheDocument();

    // Clear conversation.
    await user.click(drawer.getByRole("button", { name: "Clear conversation" }));
    expect(client.commandLog).toContainEqual({
      type: "clearTutorSession",
      payload: { workspaceId: 1 },
    });
    expect(drawer.getByText("Local ACP Tutor ready")).toBeInTheDocument();
  });

  it("silently refreshes clean editor tabs with a Diff modal and protects unsaved drafts with the conflict banner on ACP writes", async () => {
    const user = userEvent.setup();
    const client = new FakeCoreClient({ settings: { locale: "en-US" } });
    await openBuiltinWithWorkspace(client, user);

    // Navigate from ex-0 to ex-1 and open task-api/main.go in the editor (clean, no draft).
    await user.click(screen.getByRole("button", { name: "Next Chapter" }));
    await user.click(screen.getByRole("button", { name: /Open task-api\/main\.go/ }));
    const editor = await screen.findByLabelText("Edit task-api/main.go");
    expect(editor).toHaveValue("package main\n// original\n");

    // Open Tutor drawer, prompt, and approve ACP file write.
    await user.click(screen.getByRole("button", { name: "AI Tutor" }));
    const drawer = within(screen.getByTestId("tutor-drawer"));
    await user.type(drawer.getByLabelText("Ask the local ACP Tutor"), "Fix handler");
    await user.click(drawer.getByRole("button", { name: "Send" }));

    act(() => {
      client.emitTutorPermission(1, {
        id: "perm-write",
        kind: "write",
        title: "Write task-api/main.go",
        targetPath: "task-api/main.go",
        command: null,
        outsideWorkspace: false,
        sensitive: false,
        highRisk: false,
        status: "pending",
      });
    });
    await user.click(
      within(drawer.getByTestId("tutor-perm-perm-write")).getByRole("button", {
        name: "Allow once",
      })
    );
    act(() => {
      client.emitTutorFileChange(1, "task-api/main.go", "package main\n// acp edit 1\n");
      client.emitTutorTurnCompleted(1);
    });

    // Clean editor silently refreshes without conflict banner.
    await act(async () => {});
    expect(screen.getByLabelText("Edit task-api/main.go")).toHaveValue(
      "package main\n// acp edit 1\n"
    );
    expect(screen.queryByRole("region", { name: "Changed on disk" })).toBeNull();

    // Clickable Diff entry opens side-by-side Diff dialog.
    await user.click(drawer.getByRole("button", { name: "View Diff" }));
    const diffDialog = within(
      screen.getByRole("dialog", { name: "ACP File Modification Diff" })
    );
    expect(diffDialog.getByText(/\/\/ original/)).toBeInTheDocument();
    expect(diffDialog.getByText(/\/\/ acp edit 1/)).toBeInTheDocument();
    fireEvent(
      screen.getByRole("dialog", { name: "ACP File Modification Diff" }),
      new Event("cancel", { cancelable: true })
    );

    // Now create an unsaved draft in the editor, then have ACP write the same file.
    const textarea = screen.getByLabelText("Edit task-api/main.go");
    fireEvent.change(textarea, { target: { value: "package main\n// my unsaved draft\n" } });
    expect(textarea).toHaveValue("package main\n// my unsaved draft\n");

    await user.type(drawer.getByLabelText("Ask the local ACP Tutor"), "Update again");
    await user.click(drawer.getByRole("button", { name: "Send" }));
    act(() => {
      client.emitTutorFileChange(1, "task-api/main.go", "package main\n// acp edit 2\n");
      client.emitTutorTurnCompleted(1);
    });

    // Unsaved draft MUST NOT be overwritten; conflict protection banner appears.
    const conflict = await screen.findByRole("region", { name: "Changed on disk" });
    expect(within(conflict).getByRole("button", { name: "Load disk version" })).toBeInTheDocument();
    expect(within(conflict).getByRole("button", { name: "Keep draft" })).toBeInTheDocument();
    expect(screen.getByLabelText("Edit task-api/main.go")).toHaveValue(
      "package main\n// my unsaved draft\n"
    );
  });
});

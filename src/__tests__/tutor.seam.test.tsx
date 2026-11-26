import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import type { LearningProgress } from "../core/contracts";
import { createBuiltinGoCourseFixture, FakeCoreClient } from "./fakeCoreClient";

beforeEach(() => {
  window.localStorage.clear();
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    writable: true,
    value: 1440,
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

  it("shows non-overlapping selection pill across Manual, Editor, and Terminal (>=2 chars, <150ms) and hides on Escape or scroll", async () => {
    const user = userEvent.setup();
    const client = new FakeCoreClient({ settings: { locale: "en-US" } });
    await openBuiltinWithWorkspace(client, user);

    // Navigate to ex-1 and open task-api/main.go + a terminal with error output.
    await user.click(screen.getByRole("button", { name: "Next Chapter" }));
    await user.click(screen.getByRole("button", { name: /Open task-api\/main\.go/ }));
    const editor = (await screen.findByLabelText("Edit task-api/main.go")) as HTMLTextAreaElement;

    const selectDomText = (node: Text, start: number, end: number) => {
      const range = document.createRange();
      range.setStart(node, start);
      range.setEnd(node, end);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    };

    // 1. Manual prose: < 2 chars must NOT trigger pill; >= 2 chars triggers within 150ms.
    const manualPane = screen.getByTestId("manual-pane");
    const step2 = within(manualPane).getByTestId("step-2-section");
    const manualProseText = within(step2).getByText("GET /healthz").firstChild as Text;
    const origGetRect = Range.prototype.getBoundingClientRect;
    Range.prototype.getBoundingClientRect = () =>
      ({
        top: 220,
        bottom: 244,
        left: 120,
        right: 260,
        width: 140,
        height: 24,
        x: 120,
        y: 220,
        toJSON: () => ({}),
      }) as DOMRect;
    try {
      // Use fake timers to verify the <150ms pill debounce budget deterministically across CI runners.
      vi.useFakeTimers();
      selectDomText(manualProseText, 0, 1);
      fireEvent.mouseUp(manualPane.querySelector(".manual-scroll")!);
      act(() => {
        vi.advanceTimersByTime(150);
      });
      expect(screen.queryByTestId("selection-ask-pill")).toBeNull();

      selectDomText(manualProseText, 0, manualProseText.length);
      fireEvent.mouseUp(manualPane.querySelector(".manual-scroll")!);
      act(() => {
        vi.advanceTimersByTime(140);
      });
      const manualPill = screen.getByTestId("selection-ask-pill");
      vi.useRealTimers();
      expect(manualPill).toHaveAttribute("data-zone", "manual");
      expect(manualPill).toHaveAttribute("data-placement", "above");
      expect(Number.parseInt(manualPill.style.top, 10) + 34).toBeLessThanOrEqual(220);
      expect(within(manualPill).getByRole("button", { name: "Explain concept" })).toBeInTheDocument();
      expect(within(manualPill).getByRole("button", { name: "Ask" })).toBeInTheDocument();

      // Near topbar selection flips strictly below without overlapping the selection rect.
      Range.prototype.getBoundingClientRect = () =>
        ({
          top: 40,
          bottom: 64,
          left: 120,
          right: 260,
          width: 140,
          height: 24,
          x: 120,
          y: 40,
          toJSON: () => ({}),
        }) as DOMRect;
      fireEvent.mouseUp(manualPane.querySelector(".manual-scroll")!);
      await waitFor(() => {
        expect(screen.getByTestId("selection-ask-pill")).toHaveAttribute("data-placement", "below");
      });
      expect(Number.parseInt(screen.getByTestId("selection-ask-pill").style.top, 10)).toBeGreaterThanOrEqual(64);
    } finally {
      vi.useRealTimers();
      Range.prototype.getBoundingClientRect = origGetRect;
    }

    // Pressing Escape hides the pill immediately.
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId("selection-ask-pill")).toBeNull();

    // Selecting again and scrolling hides the pill immediately.
    selectDomText(manualProseText, 0, manualProseText.length);
    fireEvent.mouseUp(manualPane.querySelector(".manual-scroll")!);
    await screen.findByTestId("selection-ask-pill");
    fireEvent.scroll(manualPane.querySelector(".manual-scroll")!);
    expect(screen.queryByTestId("selection-ask-pill")).toBeNull();

    // Clicking "Explain concept" opens AI drawer with pinned manual quote card.
    selectDomText(manualProseText, 0, manualProseText.length);
    fireEvent.mouseUp(manualPane.querySelector(".manual-scroll")!);
    await user.click(
      within(await screen.findByTestId("selection-ask-pill")).getByRole("button", {
        name: "Explain concept",
      })
    );
    const drawer = screen.getByTestId("tutor-drawer");
    const manualQuote = within(drawer).getByTestId("tutor-quote-card");
    expect(manualQuote).toHaveAttribute("data-source", "manual");
    expect(manualQuote).toHaveTextContent(/GET \/healthz/);
    await user.click(within(drawer).getByRole("button", { name: "Close AI Tutor drawer" }));

    // Reopening the drawer normally via TopBar must NOT resurrect the previously consumed quote.
    await user.click(screen.getByRole("button", { name: "AI Tutor" }));
    expect(within(screen.getByTestId("tutor-drawer")).queryByTestId("tutor-quote-card")).toBeNull();
    await user.click(
      within(screen.getByTestId("tutor-drawer")).getByRole("button", {
        name: "Close AI Tutor drawer",
      })
    );

    // 2. Editor selection (>= 2 chars) shows Explain code / Check issues / Ask.
    editor.focus();
    editor.setSelectionRange(0, 12);
    fireEvent.select(editor);
    const editorPill = await screen.findByTestId("selection-ask-pill");
    expect(editorPill).toHaveAttribute("data-zone", "editor");
    expect(within(editorPill).getByRole("button", { name: "Explain code" })).toBeInTheDocument();
    expect(within(editorPill).getByRole("button", { name: "Check issues" })).toBeInTheDocument();
    expect(within(editorPill).getByRole("button", { name: "Ask" })).toBeInTheDocument();

    await user.click(within(editorPill).getByRole("button", { name: "Check issues" }));
    const editorQuote = within(screen.getByTestId("tutor-drawer")).getByTestId("tutor-quote-card");
    expect(editorQuote).toHaveAttribute("data-source", "editor");
    expect(editorQuote).toHaveTextContent("task-api/main.go:L1");
    expect(editorQuote).toHaveTextContent("package main");
    await user.click(
      within(screen.getByTestId("tutor-drawer")).getByRole("button", {
        name: "Close AI Tutor drawer",
      })
    );

    // 3. Terminal selection (>= 2 chars) shows Diagnose error / Ask.
    await user.click(screen.getByRole("button", { name: /New terminal/ }));
    client.writeTerminalOutput("panic: listen tcp :8080: bind: address already in use\n");
    const termOutput = await screen.findByTestId("terminal-fallback-output");
    await waitFor(() => expect(termOutput).toHaveTextContent("address already in use"));
    const termTextNode = termOutput.firstChild as Text;
    selectDomText(termTextNode, 0, termTextNode.length);
    fireEvent.mouseUp(termOutput);
    const termPill = await screen.findByTestId("selection-ask-pill");
    expect(termPill).toHaveAttribute("data-zone", "terminal");
    expect(within(termPill).getByRole("button", { name: "Diagnose error" })).toBeInTheDocument();
    expect(within(termPill).getByRole("button", { name: "Ask" })).toBeInTheDocument();

    await user.click(within(termPill).getByRole("button", { name: "Diagnose error" }));
    const termQuote = within(screen.getByTestId("tutor-drawer")).getByTestId("tutor-quote-card");
    expect(termQuote).toHaveAttribute("data-source", "terminal");
    expect(termQuote).toHaveTextContent("address already in use");
  });

  it("wires all four one-click Ask AI entries and 4D removable Context Snapshot Chips with live token estimate", async () => {
    const user = userEvent.setup();
    const client = new FakeCoreClient({ settings: { locale: "en-US" } });
    await openBuiltinWithWorkspace(client, user);

    // Move to ex-1 (has checks, entry file, reflections, and daemon command).
    await user.click(screen.getByRole("button", { name: "Next Chapter" }));
    await user.click(screen.getByRole("button", { name: /Open task-api\/main\.go/ }));
    await screen.findByLabelText("Edit task-api/main.go");

    // Open terminal & trigger port conflict error lines.
    await user.click(screen.getByRole("button", { name: /New terminal/ }));
    const termTab = await screen.findByRole("tab", { name: "Terminal 1" });
    expect(termTab).toBeInTheDocument();
    const readRes = await client.dispatch({
      type: "readTerminals",
      payload: { workspaceId: 1 },
    });
    if (readRes.type !== "terminals") throw new Error("Expected terminals");
    act(() => {
      client.markPortConflict(
        readRes.payload.terminals[0].id,
        "ERROR: listen tcp :8080: address already in use"
      );
    });

    // Emit failing check results for ex-1.
    const progress: LearningProgress = {
      workspaceId: 1,
      completedExercises: 0,
      totalExercises: 10,
      runningChapterId: null,
      chapters: {
        "ex-1": {
          completionState: "in_progress",
          selfChecks: {},
          lastCheckResults: [
            {
              checkId: "v1",
              status: "failed",
              durationMs: 28,
              diffs: [{ label: "status", expected: "200", actual: "500" }],
              rawOutput: "HTTP 500 internal error",
              testCases: [],
              error: null,
            },
          ],
        },
      },
    };
    act(() => client.emitEvent({ type: "progressUpdated", payload: progress }));

    // Entry 1: Failed check item in Step 3 opens Tutor drawer with checkFailure quote card.
    await user.click(await screen.findByTestId("step3-check-ask-ai-v1"));
    const drawer = screen.getByTestId("tutor-drawer");
    expect(within(drawer).getByTestId("tutor-quote-card")).toHaveAttribute(
      "data-source",
      "checkFailure"
    );
    expect(within(drawer).getByTestId("tutor-quote-card")).toHaveTextContent(/200/);

    // Verify all 4 Context Snapshot Chips + monospace token estimate render above composer.
    const contextBar = within(drawer).getByTestId("tutor-context-bar");
    await waitFor(() => {
      expect(within(contextBar).getByTestId("context-chip-chapter")).toBeInTheDocument();
      expect(within(contextBar).getByTestId("context-chip-activeFile")).toBeInTheDocument();
      expect(within(contextBar).getByTestId("context-chip-terminalErrors")).toBeInTheDocument();
      expect(within(contextBar).getByTestId("context-chip-verification")).toBeInTheDocument();
    });
    const tokenEl = within(drawer).getByTestId("tutor-token-estimate");
    expect(tokenEl).toHaveClass("mono");
    const parseTokens = () => Number(/~(\d+)/.exec(tokenEl.textContent ?? "")?.[1] ?? "0");
    const initialTokens = parseTokens();
    expect(initialTokens).toBeGreaterThan(0);

    // Click ActiveFile chip to preview its context text.
    await user.click(
      within(within(contextBar).getByTestId("context-chip-activeFile")).getByRole("button", {
        name: /Preview/,
      })
    );
    expect(within(drawer).getByTestId("context-chip-preview")).toHaveTextContent(
      "task-api/main.go"
    );

    // Remove Chapter and TerminalErrors chips -> token estimate drops and sendTutorPrompt omits them!
    await user.click(
      within(within(contextBar).getByTestId("context-chip-chapter")).getByRole("button", {
        name: /Remove/,
      })
    );
    await user.click(
      within(within(contextBar).getByTestId("context-chip-terminalErrors")).getByRole("button", {
        name: /Remove/,
      })
    );
    await waitFor(() => {
      expect(within(contextBar).queryByTestId("context-chip-chapter")).toBeNull();
      expect(within(contextBar).queryByTestId("context-chip-terminalErrors")).toBeNull();
      expect(parseTokens()).toBeLessThan(initialTokens);
    });

    await user.click(within(drawer).getByRole("button", { name: "Send" }));
    const sentCmd = client.commandLog
      .filter((cmd) => cmd.type === "sendTutorPrompt")
      .at(-1);
    expect(sentCmd?.type).toBe("sendTutorPrompt");
    if (sentCmd?.type === "sendTutorPrompt") {
      expect(sentCmd.payload.context?.chapterId).toBe("ex-1");
      expect(sentCmd.payload.context?.excludedDimensions).toEqual([
        "chapter",
        "terminalErrors",
      ]);
      expect(sentCmd.payload.context?.quote?.source).toBe("checkFailure");
    }
    act(() => client.emitTutorTurnCompleted(1));

    // Entry 2: Bottom "Check results" panel Ask AI button.
    await user.click(screen.getByRole("tab", { name: /Check results/ }));
    await user.click(screen.getByTestId("verification-panel-ask-ai"));
    expect(within(drawer).getByTestId("tutor-quote-card")).toHaveAttribute(
      "data-source",
      "verificationPanel"
    );

    // Entry 3: Reflections disclosure Ask AI button.
    const reflections = screen.getByTestId("reflections-details");
    await user.click(within(reflections).getByText("Reflection prompts"));
    await user.click(screen.getByTestId("reflections-ask-ai"));
    expect(within(drawer).getByTestId("tutor-quote-card")).toHaveAttribute(
      "data-source",
      "reflection"
    );
    expect(within(drawer).getByTestId("tutor-quote-card")).toHaveTextContent("http.Server");

    // Entry 4: Port conflict banner Ask AI button.
    await user.click(screen.getByRole("tab", { name: "Terminal" }));
    const recovery = await screen.findByRole("region", {
      name: "A local port is already in use.",
    });
    await user.click(within(recovery).getByRole("button", { name: "Ask AI" }));
    await waitFor(() => {
      expect(within(drawer).getByTestId("tutor-quote-card")).toHaveAttribute(
        "data-source",
        "portConflict"
      );
    });
    expect(within(drawer).getByTestId("tutor-quote-card")).toHaveTextContent(
      "address already in use"
    );

    // Chapter switch + style switch updates Context Snapshot & reuses Workspace ACP session.
    await user.click(screen.getByRole("button", { name: "Previous Chapter" }));
    await user.click(within(drawer).getByRole("button", { name: "Direct Answer" }));
    await waitFor(() => {
      expect(within(drawer).getByTestId("context-chip-chapter")).toHaveTextContent(/练习 0/);
    });
    const composer = within(drawer).getByLabelText("Ask the local ACP Tutor");
    await user.clear(composer);
    await user.type(composer, "Explain the task rules");
    await user.click(within(drawer).getByRole("button", { name: "Send" }));
    const secondCmd = client.commandLog
      .filter((cmd) => cmd.type === "sendTutorPrompt")
      .at(-1);
    if (secondCmd?.type === "sendTutorPrompt") {
      expect(secondCmd.payload.teachingStyle).toBe("direct");
      expect(secondCmd.payload.context?.chapterId).toBe("ex-0");
    }
    expect(within(drawer).getByText("Session reused")).toBeInTheDocument();
  });
});

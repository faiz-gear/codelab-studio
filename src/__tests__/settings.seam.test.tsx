import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { App } from "../App";
import {
  DEFAULT_SETTINGS,
  ERROR_CODES,
  type LoadedCourse,
} from "../core/contracts";
import { formatDomainError, getMessages } from "../i18n";
import {
  createBuiltinGoCourseFixture,
  FakeCoreClient,
} from "./fakeCoreClient";

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.removeAttribute("data-accent");
  document.documentElement.lang = "";
});

function createExternalCourse(path = "/courses/external-go-lab"): LoadedCourse {
  const base = createBuiltinGoCourseFixture();
  const ex0 = base.chapters.find((ch) => ch.chapterId === "ex-0")!;
  ex0.commands = [
    {
      id: "ex-0:cmd-risk",
      shell: "bash",
      command: "printf test | sh",
      cwd: null,
      cwdOrigin: "workspaceRoot",
      mode: "oneshot",
      modeOrigin: "explicit",
      label: "Risky script",
      port: null,
      isSecondary: false,
    },
  ];
  ex0.threeStep!.step1CommandIds = ["ex-0:cmd-risk"];
  return {
    ...base,
    courseId: "external-go-lab",
    title: "External Go Engineering Pack",
    isBuiltin: false,
    trusted: false,
    source: { kind: "localPack", path },
  };
}

describe("Unified 4-Tab Settings, Local Metrics, Keyboard A11y & Bilingual Error Coverage", () => {
  it("exposes all 4 Settings tabs with keyboard navigation, zero BYOK UI, and focus restoration", async () => {
    const client = new FakeCoreClient({
      settings: { ...DEFAULT_SETTINGS, locale: "en-US" },
    });
    const user = userEvent.setup();
    render(<App client={client} />);

    const settingsBtn = await screen.findByRole("button", { name: "Settings" });
    await user.click(settingsBtn);

    const dialog = screen.getByRole("dialog", { name: "Settings" });
    const runtimeTab = within(dialog).getByRole("tab", { name: "4-Language Runtimes" });
    const tutorTab = within(dialog).getByRole("tab", { name: "ACP Tutor" });
    const editorTab = within(dialog).getByRole("tab", { name: "Editor & Desktop" });
    const privacyTab = within(dialog).getByRole("tab", { name: "Privacy & Local Data" });

    expect(runtimeTab).toHaveAttribute("aria-selected", "true");
    expect(runtimeTab).toHaveFocus();

    await user.keyboard("{ArrowRight}");
    expect(editorTab).toHaveAttribute("aria-selected", "true");
    expect(editorTab).toHaveFocus();
    expect(within(dialog).getByTestId("editor-settings-panel")).toBeVisible();

    await user.keyboard("{End}");
    expect(privacyTab).toHaveAttribute("aria-selected", "true");
    expect(within(dialog).getByTestId("privacy-settings-panel")).toBeVisible();

    await user.keyboard("{Home}");
    expect(tutorTab).toHaveAttribute("aria-selected", "true");
    expect(within(dialog).getByTestId("tutor-settings-panel")).toBeVisible();

    // ADR-0004: Zero BYOK / API Key / Keychain surfaces across the Settings dialog
    expect(dialog.textContent).not.toMatch(/BYOK|API Key|Keychain/i);

    await user.keyboard("{Escape}");
    await waitFor(() =>
      expect(screen.queryByRole("dialog", { name: "Settings" })).toBeNull()
    );
    await waitFor(() => expect(settingsBtn).toHaveFocus());
  });

  it("synchronizes theme, accent, code font size, external IDE, locale, and teaching style bidirectionally across Settings, AppearanceDrawer, WorkspaceEditor, and TutorDrawer", async () => {
    const client = new FakeCoreClient({
      settings: { ...DEFAULT_SETTINGS, locale: "zh-CN", preferredIde: "vsCode" },
      pickedPaths: { workspaceDir: "/work/sync-lab" },
    });
    client.writeDiskFile("/work/sync-lab", "main.go", "package main\n");
    const user = userEvent.setup();
    render(<App client={client} />);

    // Open built-in Go course and bind Workspace
    await user.click(await screen.findByRole("button", { name: "开始内置 Go / GoFrame 实战" }));
    await user.click(await screen.findByRole("button", { name: "选择 Workspace" }));

    // Open Settings -> Editor & Desktop
    await user.click(screen.getByRole("button", { name: "设置" }));
    const dialog = screen.getByRole("dialog", { name: "设置" });
    await user.click(within(dialog).getByRole("tab", { name: "编辑器与桌面" }));

    await user.click(within(dialog).getByRole("button", { name: "深色 Dark" }));
    await user.click(within(dialog).getByRole("button", { name: "紫罗兰 Violet" }));
    await user.click(within(dialog).getByRole("button", { name: "大字 15" }));
    await user.selectOptions(within(dialog).getByLabelText("默认外部 IDE"), "cursor");
    await user.click(within(dialog).getByRole("button", { name: "English" }));

    await waitFor(() => {
      expect(document.documentElement.dataset.theme).toBe("dark");
      expect(document.documentElement.dataset.accent).toBe("violet");
      expect(document.documentElement.lang).toBe("en-US");
      expect(document.documentElement.style.getPropertyValue("--code-size")).toBe("15px");
    });

    // Dialog title & tabs switch live to English
    expect(within(dialog).getByRole("tab", { name: "Editor & Desktop" })).toBeVisible();

    // Switch to ACP Tutor tab and change teaching style to Direct Answer
    await user.click(within(dialog).getByRole("tab", { name: "ACP Tutor" }));
    await user.click(within(dialog).getByRole("button", { name: "Direct Answer" }));

    // Close Settings and check WorkspaceEditor + AppearanceDrawer + TutorDrawer reflect changes
    await user.keyboard("{Escape}");
    const workspaceIdeSelect = screen.getByLabelText("External IDE") as HTMLSelectElement;
    expect(workspaceIdeSelect.value).toBe("cursor");

    // Change IDE in WorkspaceEditor -> must sync back to Settings
    await user.selectOptions(workspaceIdeSelect, "goLand");
    expect(workspaceIdeSelect.value).toBe("goLand");

    // Change accent via AppearanceDrawer -> must sync back to Settings
    await user.click(screen.getByRole("button", { name: "Appearance" }));
    const appearanceDialog = screen.getByRole("dialog", { name: "Appearance customization" });
    expect(within(appearanceDialog).getByRole("button", { name: "Dark" })).toHaveAttribute("aria-pressed", "true");
    await user.click(within(appearanceDialog).getByRole("button", { name: "Green" }));
    await user.keyboard("{Escape}");

    // Open TutorDrawer -> reflects Direct Answer, then switch back to Socratic Guidance
    await user.click(screen.getByRole("button", { name: "AI Tutor" }));
    const tutorDrawer = screen.getByTestId("tutor-drawer");
    expect(within(tutorDrawer).getByRole("button", { name: "Direct Answer" })).toHaveAttribute("aria-pressed", "true");
    await user.click(within(tutorDrawer).getByRole("button", { name: "Socratic Guidance" }));
    await user.click(within(tutorDrawer).getByRole("button", { name: "Close AI Tutor drawer" }));

    // Reopen Settings and confirm bidirectional sync from WorkspaceEditor, AppearanceDrawer, and TutorDrawer
    await user.click(screen.getByRole("button", { name: "Settings" }));
    const reopened = screen.getByRole("dialog", { name: "Settings" });
    await user.click(within(reopened).getByRole("tab", { name: "Editor & Desktop" }));
    expect((within(reopened).getByLabelText("Preferred external IDE") as HTMLSelectElement).value).toBe("goLand");
    expect(within(reopened).getByRole("button", { name: "Green" })).toHaveAttribute("aria-pressed", "true");

    await user.click(within(reopened).getByRole("tab", { name: "ACP Tutor" }));
    expect(within(reopened).getByRole("button", { name: "Socratic Guidance" })).toHaveAttribute("aria-pressed", "true");
  });

  it("manages Trust State revocation, Tutor history clearing, Exercise progress reset, and local metrics in Privacy & Local Data", async () => {
    const externalPath = "/courses/external-go-lab";
    const externalCourse = createExternalCourse(externalPath);
    const client = new FakeCoreClient({
      settings: { ...DEFAULT_SETTINGS, locale: "en-US" },
      localCourses: { [externalPath]: externalCourse },
      trustEntries: [
        {
          sourceKind: "localPath",
          normalizedSource: externalPath,
          trustedAt: new Date().toISOString(),
        },
      ],
      pickedPaths: { workspaceDir: "/work/privacy-lab" },
    });
    client.writeDiskFile("/work/privacy-lab", "main.go", "package main\n");
    const user = userEvent.setup();
    render(<App client={client} />);

    // Import & open the trusted external local pack, then bind Workspace
    await user.click(await screen.findByRole("button", { name: "Import Local Markdown / Course Pack" }));
    const importDialog = screen.getByRole("dialog");
    await user.click(within(importDialog).getByRole("button", { name: "Local Course Pack Directory" }));
    await user.type(within(importDialog).getByLabelText("Local Course Pack directory path"), externalPath);
    await user.click(within(importDialog).getByRole("button", { name: "Open Read-Only Manual" }));
    await user.click(await screen.findByRole("button", { name: "Choose Workspace" }));

    // Simulate Exercise progress (1 / 10)
    const wsId = 1;
    act(() => {
      client.emitEvent({
        type: "recentsUpdated",
        payload: [
          {
            workspacePath: "/work/privacy-lab",
            courseId: externalCourse.courseId,
            courseTitle: externalCourse.title,
            language: "Go",
            courseSource: externalCourse.source,
            completedExercises: 1,
            totalExercises: 10,
            lastOpenedAt: new Date().toISOString(),
          },
        ],
      });
    });
    expect(screen.getByTestId("topbar-exercise-progress")).toHaveTextContent("1 / 10");

    // Trigger UI-originated local metrics: file_tree_open, cmd_risk_modal_action (cancel), selection_ai_clicked
    await user.click(screen.getByRole("button", { name: "Files" }));

    // Trigger high-risk command modal & cancel
    await user.click(screen.getByRole("button", { name: "Run command" }));
    const confirmDialog = await screen.findByRole("dialog", { name: "Confirm high-risk command" });
    await user.click(within(confirmDialog).getByRole("button", { name: "Cancel" }));

    // Trigger Ask AI from reflections -> opens Tutor & records selection_ai_clicked
    const reflections = screen.getByTestId("reflections-details");
    await user.click(reflections.querySelector("summary")!);
    await user.click(screen.getByTestId("reflections-ask-ai"));
    const drawer = await screen.findByTestId("tutor-drawer");
    await user.click(within(drawer).getByRole("button", { name: "Send" }));
    act(() => {
      client.emitTutorChunk(wsId, "Here is a Socratic hint.");
      client.emitTutorTurnCompleted(wsId);
    });
    expect(within(drawer).getByText("Here is a Socratic hint.")).toBeVisible();

    // Open Settings -> Privacy & Local Data
    await user.click(screen.getByRole("button", { name: "Settings" }));
    const settingsDialog = screen.getByRole("dialog", { name: "Settings" });
    await user.click(within(settingsDialog).getByRole("tab", { name: "Privacy & Local Data" }));

    const privacyPanel = within(settingsDialog).getByTestId("privacy-settings-panel");
    expect(privacyPanel).toHaveTextContent("Zero telemetry");
    expect(privacyPanel).toHaveTextContent(".codelab/metrics.json");
    expect(privacyPanel).toHaveTextContent(externalPath);

    // Local metrics grid reflects recorded local events
    const metricsGrid = await within(privacyPanel).findByTestId("local-metrics-grid");
    const metricCountFor = (label: string) => {
      const dt = within(metricsGrid).getByText(label);
      return Number(dt.nextElementSibling?.textContent ?? "0");
    };
    await waitFor(() => {
      expect(metricCountFor("App launched")).toBeGreaterThanOrEqual(1);
      expect(metricCountFor("Course opened")).toBeGreaterThanOrEqual(1);
      expect(metricCountFor("File tree opened")).toBeGreaterThanOrEqual(1);
      expect(metricCountFor("Selection / Failure Ask AI")).toBeGreaterThanOrEqual(1);
      expect(metricCountFor("Risk modal confirm/cancel")).toBeGreaterThanOrEqual(1);
    });

    // 1. One-click Clear Tutor Session history
    await user.click(within(privacyPanel).getByRole("button", { name: "Clear Tutor history" }));
    await waitFor(() =>
      expect(client.commandLog.some((cmd) => cmd.type === "clearTutorSession")).toBe(true)
    );
    expect(within(drawer).queryByText("Here is a Socratic hint.")).toBeNull();

    // 2. One-click Reset Exercise progress (inline confirmation with Cancel default focus)
    await user.click(within(privacyPanel).getByRole("button", { name: "Reset Exercise progress" }));
    const cancelReset = within(privacyPanel).getByRole("button", { name: "Cancel" });
    expect(cancelReset).toHaveFocus();
    await user.click(within(privacyPanel).getByRole("button", { name: "Confirm reset" }));
    await waitFor(() =>
      expect(screen.getByTestId("topbar-exercise-progress")).toHaveTextContent("0 / 10")
    );

    // 3. Revoke Trust State -> removes entry and locks active external Course back to read-only preview
    await user.click(within(privacyPanel).getByRole("button", { name: "Revoke trust" }));
    await waitFor(() =>
      expect(within(privacyPanel).getByText("No external local Courses or Git Course Packs are currently trusted.")).toBeVisible()
    );

    await user.keyboard("{Escape}");
    await waitFor(() => {
      // Revoking trust locks both the Workspace stage and the open TutorDrawer immediately
      expect(screen.getAllByRole("button", { name: "Trust this Course" }).length).toBe(2);
      expect(screen.getByTestId("tutor-untrusted-notice")).toBeVisible();
    });
  });

  it("restores focus on Escape for Course menu, Import modal, and Chapter Outline, and mounts Toast live region persistently", async () => {
    const client = new FakeCoreClient({
      settings: { ...DEFAULT_SETTINGS, locale: "en-US" },
    });
    const user = userEvent.setup();
    render(<App client={client} />);

    // Persistent polite aria-live container even with zero active toasts
    const liveStatus = await screen.findByRole("status");
    expect(liveStatus).toHaveAttribute("aria-live", "polite");

    // Course menu keyboard nav + Escape focus restoration
    const courseMenuTrigger = screen.getByRole("button", { name: "Switch or import Course" });
    await user.click(courseMenuTrigger);
    const menu = screen.getByRole("menu", { name: "Switch or import Course" });
    const items = within(menu).getAllByRole("menuitem");
    await waitFor(() => expect(items[0]).toHaveFocus());
    await user.keyboard("{ArrowDown}");
    expect(items[1]).toHaveFocus();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(courseMenuTrigger).toHaveFocus());

    // Import modal Escape focus restoration
    const importBtn = screen.getByRole("button", { name: "Import Local Markdown / Course Pack" });
    await user.click(importBtn);
    expect(screen.getByRole("dialog", { name: "Import or Author a Course Pack" })).toBeVisible();
    await user.keyboard("{Escape}");
    await waitFor(() =>
      expect(screen.queryByRole("dialog", { name: "Import or Author a Course Pack" })).toBeNull()
    );
    await waitFor(() => expect(importBtn).toHaveFocus());

    // Import modal opened from TopBar Course menu restores focus to Course menu trigger on Escape
    await user.click(courseMenuTrigger);
    await user.click(
      within(screen.getByRole("menu", { name: "Switch or import Course" })).getByRole(
        "menuitem",
        { name: /Import Local Markdown/ }
      )
    );
    expect(screen.getByRole("dialog", { name: "Import or Author a Course Pack" })).toBeVisible();
    await user.keyboard("{Escape}");
    await waitFor(() =>
      expect(screen.queryByRole("dialog", { name: "Import or Author a Course Pack" })).toBeNull()
    );
    await waitFor(() => expect(courseMenuTrigger).toHaveFocus());

    // Chapter outline keyboard nav survives parent re-renders and restores focus on Escape
    await user.click(screen.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }));
    const outlineBtn = await screen.findByRole("button", { name: "Chapter outline" });
    await user.click(outlineBtn);
    const outline = screen.getByTestId("chapter-outline-drawer");
    expect(outline).toBeVisible();
    await user.keyboard("{ArrowDown}");
    const focusedItem = document.activeElement;
    // Trigger an unrelated parent re-render (preflight update) while outline is open
    act(() => {
      client.emitEvent({
        type: "preflightUpdated",
        payload: {
          revision: 99,
          checked: true,
          probing: false,
          installingCount: 0,
          languages: [],
          tools: [],
          goProxy: null,
          installDir: "/codelab/runtimes",
        },
      });
    });
    expect(document.activeElement).toBe(focusedItem);
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByTestId("chapter-outline-drawer")).toBeNull());
    await waitFor(() => expect(outlineBtn).toHaveFocus());
  });

  it("renders every ErrorCode cleanly in both zh-CN and en-US with zero raw codes or unresolved placeholders", () => {
    const sampleDetails: Record<string, string> = {
      path: "/tmp/demo.go",
      source: "demo-course",
      ide: "VS Code",
      min: "11",
      max: "20",
      completed: "3",
      total: "2",
      maxMb: "5",
      store: "settings",
      port: "8080",
      shell: "bash",
      url: "https://example.org/course.git",
      checkId: "v1",
      failures: "1",
    };

    const zhMessages = getMessages("zh-CN");
    const enMessages = getMessages("en-US");

    for (const code of ERROR_CODES) {
      expect(zhMessages.errors[code]).toBeTruthy();
      expect(enMessages.errors[code]).toBeTruthy();

      const zh = formatDomainError({ code, details: sampleDetails }, "zh-CN");
      const en = formatDomainError({ code, details: sampleDetails }, "en-US");

      expect(zh, `zh-CN formatting for ${code}`).not.toContain(code);
      expect(en, `en-US formatting for ${code}`).not.toContain(code);
      expect(zh, `unresolved placeholder in zh-CN ${code}`).not.toMatch(/\{[a-zA-Z0-9_]+\}/);
      expect(en, `unresolved placeholder in en-US ${code}`).not.toMatch(/\{[a-zA-Z0-9_]+\}/);
      expect(zh, `zh-CN ${code} should contain CJK characters`).toMatch(/[\u4e00-\u9fff]/);
      expect(en, `en-US ${code} should not contain CJK characters`).not.toMatch(/[\u4e00-\u9fff]/);
    }
  });
});

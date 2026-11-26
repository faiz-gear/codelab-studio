import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import { formatDomainError } from "../i18n";
import { APPEARANCE_CACHE_KEY } from "../theme/applyTheme";
import { FakeCoreClient } from "./fakeCoreClient";

beforeEach(() => {
  window.localStorage.clear();
  document.documentElement.dataset.theme = "light";
  document.documentElement.dataset.accent = "neutral";
  document.documentElement.lang = "zh-CN";
  document.documentElement.style.removeProperty("--code-size");
  document.documentElement.style.removeProperty("--code-lh");
});

describe("CodeLab Studio frontend seam (CoreClient)", () => {
  it("renders Welcome Home hero, language filter, 2x2 course placeholders, and recent Workspaces from CoreClient", async () => {
    const client = new FakeCoreClient({
      recents: [
        {
          workspacePath: "/Users/learner/codelab/go-practice",
          courseId: "go-goframe-practice",
          courseTitle: "Go 与 GoFrame 开发实践",
          language: "Go",
          completedExercises: 4,
          totalExercises: 10,
          lastOpenedAt: "2026-11-24T06:00:00.000Z",
        },
      ],
    });

    const user = userEvent.setup();
    render(<App client={client} />);

    // Cold-start applies authoritative settings from CoreClient
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
      "开始今天的本地实战。"
    );
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.dataset.accent).toBe("neutral");
    expect(document.documentElement.lang).toBe("zh-CN");
    expect(
      document.documentElement.style.getPropertyValue("--code-size")
    ).toBe("13.5px");

    // 2x2 Course placeholder grid shows all 4 languages initially
    const courseGrid = screen.getByTestId("course-grid");
    expect(within(courseGrid).getAllByRole("button")).toHaveLength(4);

    // Language filter narrows the grid
    const filterGroup = screen.getByRole("group", { name: "按语言筛选" });
    await user.click(within(filterGroup).getByRole("button", { name: "Go" }));
    expect(within(courseGrid).getAllByRole("button")).toHaveLength(1);
    expect(
      within(courseGrid).getByText("Go 与 GoFrame 开发实践")
    ).toBeInTheDocument();

    // Recent Workspace list shows persisted Workspace from CoreClient
    const recentList = screen.getByTestId("recent-workspace-list");
    expect(
      within(recentList).getByText("/Users/learner/codelab/go-practice")
    ).toBeInTheDocument();
    expect(within(recentList).getByText("4/10")).toBeInTheDocument();
  });

  it("switches theme, 8 accents, code font size, language, copies theme config, and persists via CoreClient", async () => {
    const client = new FakeCoreClient();
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText");
    const { unmount } = render(<App client={client} />);

    await screen.findByRole("heading", { level: 1 });

    // Open Appearance Drawer
    const trigger = screen.getByRole("button", { name: "外观" });
    await user.click(trigger);

    const dialog = screen.getByRole("dialog", { name: "外观定制" });

    // Verify all 8 Arc accent swatches exist and pick Rose
    const accentGroup = within(dialog).getByRole("group", {
      name: "选择强调色",
    });
    expect(within(accentGroup).getAllByRole("button")).toHaveLength(8);
    await user.click(
      within(accentGroup).getByRole("button", { name: "玫红 Rose" })
    );
    expect(document.documentElement.dataset.accent).toBe("rose");

    // Switch to Dark theme
    await user.click(within(dialog).getByRole("button", { name: "深色 Dark" }));
    expect(document.documentElement.dataset.theme).toBe("dark");

    // Switch code font size to Large (15px)
    await user.click(within(dialog).getByRole("button", { name: "大字 15" }));
    expect(
      document.documentElement.style.getPropertyValue("--code-size")
    ).toBe("15px");

    // Switch UI language to English (en-US)
    await user.click(within(dialog).getByRole("button", { name: "English" }));
    expect(document.documentElement.lang).toBe("en-US");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
      "Start today's local practice."
    );

    // Copy theme config in English
    await user.click(
      within(dialog).getByRole("button", { name: "Copy theme config" })
    );
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText.mock.calls[0][0]).toContain('data-theme="dark"');
    expect(writeText.mock.calls[0][0]).toContain('data-accent="rose"');
    expect(writeText.mock.calls[0][0]).toContain("--code-size: 15px;");
    expect(
      await screen.findByText("Copied theme config (dark · rose)")
    ).toBeInTheDocument();

    // Escape closes drawer and restores focus to Appearance button
    await user.keyboard("{Escape}");
    expect(
      screen.queryByRole("dialog", { name: "Appearance customization" })
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Appearance" })).toHaveFocus();

    // Simulate app restart with same CoreClient persistence (no zh-CN flash on initial render)
    unmount();
    render(<App client={client} />);

    // Synchronously hydrated from appearance cache before async bootstrap settles
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
      "Start today's local practice."
    );
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
      "Start today's local practice."
    );
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.dataset.accent).toBe("rose");
    expect(
      document.documentElement.style.getPropertyValue("--code-size")
    ).toBe("15px");
    expect(document.documentElement.lang).toBe("en-US");
  });

  it("renders structured DomainError codes in zh-CN and en-US and reacts to CoreClient events", async () => {
    const client = new FakeCoreClient({
      recoveryNotices: [
        {
          store: "recents",
          errorCode: "STORAGE_CORRUPTED",
          backupPath: "/tmp/recents.json.corrupt",
        },
      ],
    });
    const user = userEvent.setup();
    render(<App client={client} />);

    // Startup corruption recovery toast localized in zh-CN
    expect(
      await screen.findByText(
        "检测到 最近 Workspace（recents） 配置文件损坏，已自动备份并恢复默认配置。"
      )
    ).toBeInTheDocument();

    // Inject structured DomainError on next updateSettings in zh-CN
    client.failNext("updateSettings", {
      code: "STORAGE_WRITE_FAILED",
      details: { store: "settings" },
    });

    await user.click(screen.getByRole("button", { name: "外观" }));
    const dialog = screen.getByRole("dialog", { name: "外观定制" });
    await user.click(within(dialog).getByRole("button", { name: "深色 Dark" }));

    expect(
      await screen.findByText(
        "保存本地数据（全局偏好（settings））失败，请检查磁盘空间与写权限。"
      )
    ).toBeInTheDocument();
    expect(screen.queryByText(/STORAGE_WRITE_FAILED/)).not.toBeInTheDocument();

    // Switch to en-US, then inject another DomainError and verify English rendering
    await user.click(within(dialog).getByRole("button", { name: "English" }));
    client.failNext("updateSettings", {
      code: "STORAGE_WRITE_FAILED",
      details: { store: "settings" },
    });
    await user.click(within(dialog).getByRole("button", { name: "Dark" }));

    expect(
      await screen.findByText(
        "Failed to save local store (global settings). Check disk space and write permissions."
      )
    ).toBeInTheDocument();

    // Unknown or missing-detail error falls back safely without leaking raw placeholders
    expect(
      formatDomainError({ code: "STORAGE_READ_FAILED" }, "en-US")
    ).toBe("Unable to complete the operation. Please retry.");
    expect(
      formatDomainError({ code: "UNKNOWN_FUTURE_CODE" }, "zh-CN")
    ).toBe("操作未能完成，请重试。");

    // Reactive CoreClient event stream updates recent Workspaces live
    await user.keyboard("{Escape}");
    act(() => {
      client.emitEvent({
        type: "recentsUpdated",
        payload: [
          {
            workspacePath: "/Users/learner/codelab/python-clean",
            courseId: "python-data-clean",
            courseTitle: "Python Data Cleaning & Order Analytics",
            language: "Python",
            completedExercises: 3,
            totalExercises: 6,
            lastOpenedAt: new Date().toISOString(),
          },
        ],
      });
    });

    const recentList = screen.getByTestId("recent-workspace-list");
    expect(
      await within(recentList).findByText("/Users/learner/codelab/python-clean")
    ).toBeInTheDocument();
    expect(within(recentList).getByText("3/6")).toBeInTheDocument();
  });

  it("hydrates persisted en-US locale synchronously from cache without zh-CN first-frame flash", async () => {
    window.localStorage.setItem(
      APPEARANCE_CACHE_KEY,
      JSON.stringify({
        theme: "dark",
        accent: "blue",
        codeFontSize: 12.5,
        locale: "en-US",
      })
    );

    const client = new FakeCoreClient({
      settings: {
        theme: "dark",
        accent: "blue",
        codeFontSize: 12.5,
        locale: "en-US",
      },
    });

    render(<App client={client} />);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
      "Start today's local practice."
    );
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
      "Start today's local practice."
    );
  });

  it("lights up all 4 language builtin Course Packs in Welcome Home 2x2 grid, language filter, and TopBar dropdown", async () => {
    const client = new FakeCoreClient();
    const user = userEvent.setup();
    render(<App client={client} />);

    await screen.findByRole("heading", { level: 1 });

    for (const [lang, expectedTitle, courseId] of [
      ["JS/TS", "TypeScript 数据流水线实战", "ts-data-pipeline"],
      ["Python", "Python 数据清洗与订单统计", "python-data-clean"],
      ["Java", "Java 21 并发限流器实战", "java-rate-limiter"],
    ] as const) {
      const courseGrid = screen.getByTestId("course-grid");
      const filterGroup = screen.getByRole("group", { name: "按语言筛选" });
      await user.click(within(filterGroup).getByRole("button", { name: lang }));
      expect(within(courseGrid).getAllByRole("button")).toHaveLength(1);
      await user.click(
        within(courseGrid).getByRole("button", { name: new RegExp(expectedTitle) })
      );
      expect(
        await screen.findByRole("region", { name: "只读手册工作台" })
      ).toBeInTheDocument();
      expect(client.commandLog).toContainEqual({
        type: "openCourse",
        payload: { kind: "builtin", courseId },
      });

      // Return Home via TopBar dropdown and verify all 4 builtins are listed
      await user.click(screen.getByRole("button", { name: /切换或导入 Course/ }));
      const menu = screen.getByRole("menu", { name: "切换或导入 Course" });
      expect(within(menu).getByRole("menuitem", { name: /Go 与 GoFrame 开发实践/ })).toBeInTheDocument();
      expect(within(menu).getByRole("menuitem", { name: /TypeScript 数据流水线实战/ })).toBeInTheDocument();
      expect(within(menu).getByRole("menuitem", { name: /Python 数据清洗与订单统计/ })).toBeInTheDocument();
      expect(within(menu).getByRole("menuitem", { name: /Java 21 并发限流器实战/ })).toBeInTheDocument();
      await user.click(within(menu).getByRole("menuitem", { name: "返回课程主页" }));
      await screen.findByRole("heading", { level: 1 });
    }
  });
});

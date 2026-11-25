import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import type { LoadedCourse } from "../core/contracts";
import { FakeCoreClient } from "./fakeCoreClient";

function createPlainMarkdownFixture(): LoadedCourse {
  return {
    courseId: "local-md-shell-notes",
    title: "Shell 运维速查手册",
    description: "",
    language: null,
    version: null,
    author: null,
    dependencies: [],
    source: { kind: "localMarkdown", path: "/Users/learner/notes/shell.md" },
    entryMarkdownPath: "/Users/learner/notes/shell.md",
    isBuiltin: false,
    trusted: false,
    introHtml: "<p>纯外部标准 Markdown 示例。</p>",
    totalExercises: 0,
    chapters: [
      {
        chapterId: "checking-ports",
        title: "检查端口占用",
        kind: "guide",
        presentation: "linearFallback",
        defaultCwd: null,
        entryFile: null,
        summaryHtml: "<p>按原文顺序查看端口并启动服务。</p>",
        blocks: [
          { kind: "prose", html: "<p>先查看 8080 端口占用情况：</p>" },
          { kind: "command", commandId: "checking-ports:cmd-1" },
          { kind: "prose", html: "<p>再在 api 目录启动开发服务：</p>" },
          { kind: "command", commandId: "checking-ports:cmd-2" },
        ],
        commands: [
          {
            id: "checking-ports:cmd-1",
            shell: "bash",
            command: "lsof -i :8080",
            cwd: null,
            cwdOrigin: "workspaceRoot",
            mode: "oneshot",
            modeOrigin: "inferred",
            label: null,
            port: null,
            isSecondary: true,
          },
          {
            id: "checking-ports:cmd-2",
            shell: "sh",
            command: "cd api && npm run dev",
            cwd: "api",
            cwdOrigin: "inferred",
            mode: "daemon",
            modeOrigin: "inferred",
            label: null,
            port: null,
            isSecondary: false,
          },
        ],
        threeStep: null,
        reflectionsHtml: null,
      },
    ],
  };
}

beforeEach(() => {
  window.localStorage.clear();
  document.documentElement.dataset.theme = "light";
  document.documentElement.dataset.accent = "neutral";
  document.documentElement.lang = "zh-CN";
});

describe("Course Workbench & Navigation seam (CoreClient)", () => {
  it("opens built-in Go/GoFrame Course Pack from Welcome Home and TopBar dropdown, and imports local Markdown / Course Pack with localized errors", async () => {
    const localCourse = createPlainMarkdownFixture();
    const client = new FakeCoreClient({
      localCourses: {
        "/Users/learner/notes/shell.md": localCourse,
      },
      pickedPaths: {
        markdownFile: "/Users/learner/notes/shell.md",
      },
    });
    const user = userEvent.setup();
    render(<App client={client} />);

    // Wait for Welcome Home bootstrap
    await screen.findByRole("heading", { level: 1 });

    // 1. Open built-in Go / GoFrame directly from Welcome Home course card
    const courseGrid = screen.getByTestId("course-grid");
    await user.click(
      within(courseGrid).getByRole("button", { name: /Go 与 GoFrame 开发实践/ })
    );

    // Enters two-pane workbench and displays Go/GoFrame course
    expect(
      await screen.findByRole("region", { name: "只读手册工作台" })
    ).toBeInTheDocument();
    expect(client.commandLog).toContainEqual({
      type: "openCourse",
      payload: { kind: "builtin", courseId: "go-goframe-practice" },
    });

    // TopBar shows course dropdown & exercise-only progress counter (0 / 10)
    const courseMenuTrigger = screen.getByRole("button", {
      name: /切换或导入 Course/,
    });
    expect(courseMenuTrigger).toHaveTextContent("Go 与 GoFrame 开发实践");
    expect(screen.getByTestId("topbar-exercise-progress")).toHaveTextContent(
      "0 / 10"
    );

    // 2. Return to Welcome Home via TopBar course dropdown
    await user.click(courseMenuTrigger);
    const menu = screen.getByRole("menu", { name: "切换或导入 Course" });
    await user.click(within(menu).getByRole("menuitem", { name: "返回课程主页" }));

    expect(
      await screen.findByRole("heading", { level: 1 })
    ).toHaveTextContent("开始今天的本地实战。");
    expect(client.commandLog).toContainEqual({ type: "closeActiveCourse" });

    // 3. Re-open built-in Go/GoFrame directly from TopBar dropdown while on Home
    await user.click(
      screen.getByRole("button", { name: /切换或导入 Course/ })
    );
    const homeMenu = screen.getByRole("menu", { name: "切换或导入 Course" });
    await user.click(
      within(homeMenu).getByRole("menuitem", { name: /Go 与 GoFrame 开发实践/ })
    );
    expect(
      await screen.findByRole("region", { name: "只读手册工作台" })
    ).toBeInTheDocument();

    // 4. Open Import Local Course modal from TopBar dropdown and verify localized error first
    await user.click(
      screen.getByRole("button", { name: /切换或导入 Course/ })
    );
    await user.click(
      within(screen.getByRole("menu", { name: "切换或导入 Course" })).getByRole(
        "menuitem",
        { name: /导入本地 Markdown/ }
      )
    );

    const dialog = screen.getByRole("dialog", { name: "导入本地 Course" });
    await user.click(within(dialog).getByRole("button", { name: "浏览 .md 文件" }));
    expect(
      within(dialog).getByLabelText("本地 Markdown 文件路径")
    ).toHaveValue("/Users/learner/notes/shell.md");

    client.failNext("openCourse", {
      code: "COURSE_MARKDOWN_TOO_LARGE",
      details: { maxMb: "5" },
    });
    await user.click(within(dialog).getByRole("button", { name: "打开只读手册" }));
    expect(
      await within(dialog).findByText(
        "Markdown 文件超过 5 MB 上限，已拒绝加载。"
      )
    ).toBeInTheDocument();

    // Retry succeeds and loads the local Markdown course into the workbench
    await user.click(within(dialog).getByRole("button", { name: "打开只读手册" }));
    expect(
      screen.queryByRole("dialog", { name: "导入本地 Course" })
    ).not.toBeInTheDocument();
    expect(
      await screen.findByRole("heading", { level: 2, name: "检查端口占用" })
    ).toBeInTheDocument();
  });

  it("renders 3-step guided flow with collapsed <details>, distinguishes guide vs exercise in outline drawer, and navigates prev/next", async () => {
    const client = new FakeCoreClient();
    const user = userEvent.setup();
    render(<App client={client} />);

    await screen.findByRole("heading", { level: 1 });
    await user.click(
      within(screen.getByTestId("course-grid")).getByRole("button", {
        name: /Go 与 GoFrame 开发实践/,
      })
    );

    // Lands on first Exercise (ex-0); ex-0 has no primary commands so Step 1 is not fabricated empty
    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "练习 0：写清楚系统的规则",
      })
    ).toBeInTheDocument();
    expect(screen.getByTestId("manual-chapter-pos")).toHaveClass("mono");
    expect(screen.queryByTestId("step-1-section")).not.toBeInTheDocument();
    expect(screen.getByTestId("step-2-section")).toBeInTheDocument();
    expect(screen.getByTestId("step-3-section")).toBeInTheDocument();

    // Navigate to next chapter (ex-1) via top manual nav button
    await user.click(screen.getByRole("button", { name: "下一节" }));
    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "练习 1：让一个请求完成往返",
      })
    ).toBeInTheDocument();

    // Full 3-step projection rendered with daemon command, target entryFile, and HTTP checks
    const threeStep = screen.getByTestId("three-step-flow");
    expect(within(threeStep).getByTestId("step-1-section")).toBeInTheDocument();
    expect(within(threeStep).getByTestId("step-2-section")).toBeInTheDocument();
    expect(within(threeStep).getByTestId("step-3-section")).toBeInTheDocument();

    const daemonCard = screen.getByTestId("command-block-ex-1:cmd-1");
    expect(daemonCard).toHaveTextContent("go run .");
    expect(daemonCard).toHaveTextContent("task-api");
    expect(daemonCard).toHaveTextContent("常驻 daemon");

    // Secondary debug commands & reflections are collapsed in <details> by default
    const secDetails = screen.getByTestId(
      "secondary-commands-details"
    ) as HTMLDetailsElement;
    const reflDetails = screen.getByTestId(
      "reflections-details"
    ) as HTMLDetailsElement;
    expect(secDetails.open).toBe(false);
    expect(reflDetails.open).toBe(false);

    // Expanding <details> reveals folded curl command and reflection question
    await user.click(within(secDetails).getByText(/次要调试命令/));
    expect(secDetails.open).toBe(true);
    expect(within(secDetails).getByText(/curl -i/)).toBeInTheDocument();

    await user.click(within(reflDetails).getByText("课后思考题"));
    expect(reflDetails.open).toBe(true);
    expect(within(reflDetails).getByText(/http\.Server/)).toBeInTheDocument();

    // Next buttons (header + footer) are disabled at last chapter
    for (const btn of screen.getAllByRole("button", { name: "下一节" })) {
      expect(btn).toBeDisabled();
    }

    // Open Chapter Outline Drawer, verify guide vs exercise badges, and switch to guide chapter
    await user.click(screen.getByRole("button", { name: "章节大纲" }));
    const outline = screen.getByTestId("chapter-outline-drawer");
    expect(within(outline).getByText("概念导读")).toBeInTheDocument();
    expect(within(outline).getAllByText("实战 Exercise")).toHaveLength(2);

    await user.click(
      within(outline).getByRole("button", { name: /工作目录与使用方式/ })
    );
    expect(
      screen.queryByTestId("chapter-outline-drawer")
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 2, name: "工作目录与使用方式" })
    ).toBeInTheDocument();
    expect(screen.getByTestId("course-intro")).toHaveTextContent(
      "先用 Go 标准库实现，再迁移到 GoFrame。"
    );
    expect(screen.getByTestId("chapter-kind-badge")).toHaveTextContent(
      "概念导读"
    );
    // Opening paragraph of LinearFallback guide chapter renders once (not duplicated in hero + blocks)
    expect(
      screen.getAllByText("选择一个本地空目录作为学习 Workspace。")
    ).toHaveLength(1);
    expect(screen.getByTestId("linear-fallback-flow")).toBeInTheDocument();
    expect(screen.queryByTestId("three-step-flow")).not.toBeInTheDocument();
    for (const btn of screen.getAllByRole("button", { name: "上一节" })) {
      expect(btn).toBeDisabled();
    }
  });

  it("supports Ctrl/Cmd+B focus mode, splitter drag + double-click reset, plain Markdown fallback, and hot-reload banner", async () => {
    const plainCourse = createPlainMarkdownFixture();
    const localPackCourse: LoadedCourse = {
      ...plainCourse,
      courseId: "local-pack-demo",
      title: "本地课程包示例",
      source: { kind: "localPack", path: "/Users/learner/packs/demo" },
      entryMarkdownPath: "/Users/learner/packs/demo/README.md",
    };

    const client = new FakeCoreClient({
      localCourses: {
        "/Users/learner/notes/shell.md": plainCourse,
        "/Users/learner/packs/demo": localPackCourse,
      },
      pickedPaths: {
        coursePackDir: "/Users/learner/packs/demo",
      },
    });
    const user = userEvent.setup();
    render(<App client={client} />);

    await screen.findByRole("heading", { level: 1 });

    // Open Import modal from Welcome Home secondary CTA and import a local Course Pack directory
    await user.click(
      screen.getByRole("button", { name: "导入本地 Markdown / Course Pack" })
    );
    const dialog = screen.getByRole("dialog", { name: "导入本地 Course" });
    await user.click(
      within(dialog).getByRole("button", { name: "本地 Course Pack 目录" })
    );
    await user.click(within(dialog).getByRole("button", { name: "浏览目录" }));
    await user.click(within(dialog).getByRole("button", { name: "打开只读手册" }));

    expect(client.commandLog).toContainEqual({
      type: "openCourse",
      payload: { kind: "localPack", path: "/Users/learner/packs/demo" },
    });

    // Plain Markdown renders linear fallback without fabricated empty 3-step cards
    const wb = await screen.findByTestId("course-workbench");
    expect(screen.getByTestId("linear-fallback-flow")).toBeInTheDocument();
    expect(screen.queryByTestId("three-step-flow")).not.toBeInTheDocument();
    expect(screen.getByText("只读预览")).toBeInTheDocument();
    expect(
      screen.getByTestId("command-block-checking-ports:cmd-2")
    ).toHaveTextContent("api");

    // 1. Focus mode via Ctrl+B and restore via Focus Rail button
    expect(wb).not.toHaveClass("is-focus");
    fireEvent.keyDown(window, { key: "b", ctrlKey: true });
    expect(wb).toHaveClass("is-focus");

    await user.click(
      screen.getByRole("button", { name: "展开手册（⌘/Ctrl + B）" })
    );
    expect(wb).not.toHaveClass("is-focus");

    // 2. Splitter drag resizes left manual pane; double-click resets width
    const manualPane = screen.getByTestId("manual-pane");
    const splitter = screen.getByTestId("workbench-splitter");
    vi.spyOn(wb, "getBoundingClientRect").mockReturnValue({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      right: 1200,
      bottom: 800,
      width: 1200,
      height: 800,
      toJSON: () => ({}),
    });

    fireEvent.mouseDown(splitter, { clientX: 460 });
    fireEvent.mouseMove(window, { clientX: 520 });
    fireEvent.mouseUp(window);
    expect(manualPane.style.width).toBe("520px");

    fireEvent.doubleClick(splitter);
    expect(manualPane.style.width).toBe("");

    // 3. Hot-reload event displays banner and Reload refreshes content preserving chapterId
    const updatedPack: LoadedCourse = {
      ...localPackCourse,
      chapters: [
        {
          ...localPackCourse.chapters[0],
          title: "检查端口占用（热重载已更新）",
          blocks: [
            { kind: "prose", html: "<p>已加载磁盘最新勘误内容。</p>" },
            { kind: "command", commandId: "checking-ports:cmd-2" },
          ],
        },
      ],
    };
    client.registerLocalCourse("/Users/learner/packs/demo", updatedPack);

    act(() => {
      client.emitEvent({
        type: "courseSourceChanged",
        payload: {
          courseId: "local-pack-demo",
          changedPaths: ["/Users/learner/packs/demo/README.md"],
          detectedAt: new Date().toISOString(),
        },
      });
    });

    const reloadBanner = await screen.findByTestId("hot-reload-banner");
    expect(reloadBanner).toHaveTextContent("README.md");

    await user.click(within(reloadBanner).getByRole("button", { name: "立即刷新" }));
    expect(client.commandLog).toContainEqual({ type: "reloadActiveCourse" });
    expect(screen.queryByTestId("hot-reload-banner")).not.toBeInTheDocument();
    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "检查端口占用（热重载已更新）",
      })
    ).toBeInTheDocument();
    expect(screen.getByText("已加载磁盘最新勘误内容。")).toBeInTheDocument();
  });
});

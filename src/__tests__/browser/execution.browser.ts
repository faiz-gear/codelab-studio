import { expect, test } from "@playwright/test";

test("xterm renders streamed output and sends ordered input through CoreClient", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));

  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();
  await page.getByRole("button", { name: "Next Chapter" }).click();
  const commandHeader = page.locator(".cmd-head").first();
  await expect(commandHeader).toHaveCSS("height", "38px");
  await expect(commandHeader).toHaveCSS("flex-wrap", "nowrap");
  await commandHeader.getByRole("button", { name: "Run command" }).click();
  const daemon = page.getByRole("tab", { name: "Managed Daemon 1" });
  await expect(daemon).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: "New terminal" }).click();
  const terminal = page.getByRole("tabpanel", { name: /Terminal 1/ });
  await expect(daemon).toHaveAttribute("aria-selected", "false");
  const viewport = terminal.getByRole("region", { name: "Terminal 1" });
  await expect(viewport.locator(".xterm")).toBeVisible();
  await page.evaluate(() => window.workspaceSmoke.writeTerminalOutput("PTY output fixture\r\n"));
  await expect(viewport.getByText("PTY output fixture", { exact: true }).first()).toBeVisible();

  await viewport.locator("textarea").focus();
  await page.keyboard.type("pwd");
  await page.keyboard.press("Enter");
  await expect.poll(() => page.evaluate(() => window.workspaceSmoke.terminalInput())).toBe("pwd\r");
  await page.getByRole("button", { name: "Maximize terminal" }).click();
  await expect(page.getByRole("button", { name: "Restore terminal size" })).toBeVisible();
  await page.getByRole("button", { name: "Clear terminal" }).click();
  await expect(viewport.getByText("PTY output fixture", { exact: true })).toHaveCount(0);

  await page.evaluate(() => {
    for (let batch = 0; batch < 24; batch++) {
      window.workspaceSmoke.writeTerminalOutput("Terminal output batch\r\n".repeat(250));
    }
    window.workspaceSmoke.writeTerminalOutput("Last streamed line\r\n");
  });
  await expect(viewport.getByText("Last streamed line", { exact: true }).first()).toBeVisible();
  expect(errors).toEqual([]);
});

test("Maximize terminal expands terminal viewport height and visible xterm rows, and Restore returns to pre-maximize height preserving output", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();
  await page.getByRole("button", { name: "New terminal" }).click();

  const terminal = page.getByRole("tabpanel", { name: /Terminal 1/ });
  const viewport = terminal.getByRole("region", { name: "Terminal 1" });
  await expect(viewport.locator(".xterm")).toBeVisible();

  await page.evaluate(() => {
    const lines = Array.from({ length: 30 }, (_, i) => `output-line-${String(i + 1).padStart(2, "0")}`).join("\r\n");
    window.workspaceSmoke.writeTerminalOutput(`${lines}\r\n`);
  });
  await expect(viewport.getByText("output-line-30", { exact: true }).first()).toBeVisible();

  const measureTerminal = () =>
    viewport.evaluate((el) => {
      const rect = el.getBoundingClientRect();
      const rows = el.querySelectorAll(".xterm-rows > div").length;
      return { height: Math.round(rect.height), rows };
    });

  await expect.poll(async () => (await measureTerminal()).rows).toBeGreaterThanOrEqual(4);
  const normal = await measureTerminal();

  await page.getByRole("button", { name: "Maximize terminal" }).click();
  await expect(page.getByRole("button", { name: "Restore terminal size" })).toBeVisible();

  await expect
    .poll(async () => (await measureTerminal()).height)
    .toBeGreaterThan(normal.height + 150);
  await expect
    .poll(async () => (await measureTerminal()).rows)
    .toBeGreaterThan(normal.rows + 5);

  const maximized = await measureTerminal();
  expect(maximized.height).toBeGreaterThan(normal.height + 150);
  expect(maximized.rows).toBeGreaterThan(normal.rows + 5);
  await expect(viewport.getByText("output-line-30", { exact: true }).first()).toBeVisible();

  await page.getByRole("button", { name: "Restore terminal size" }).click();
  await expect(page.getByRole("button", { name: "Maximize terminal" })).toBeVisible();
  await expect
    .poll(async () => Math.abs((await measureTerminal()).height - normal.height))
    .toBeLessThanOrEqual(4);
  await expect
    .poll(async () => (await measureTerminal()).rows)
    .toBe(normal.rows);
  await expect(viewport.getByText("output-line-30", { exact: true }).first()).toBeVisible();

  expect(errors).toEqual([]);
});

test("supports resizable workbench splitters (terminal, file tree, manual, and docked AI Tutor) at 1440x900 and 1024x720", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();
  await page.getByRole("button", { name: "New terminal" }).click();

  const terminal = page.getByRole("tabpanel", { name: /Terminal 1/ });
  const viewport = terminal.getByRole("region", { name: "Terminal 1" });
  await expect(viewport.locator(".xterm")).toBeVisible();
  await page.evaluate(() => {
    const lines = Array.from({ length: 40 }, (_, i) => `drag-line-${String(i + 1).padStart(2, "0")}`).join("\r\n");
    window.workspaceSmoke.writeTerminalOutput(`${lines}\r\n`);
  });
  await expect(viewport.getByText("drag-line-40", { exact: true }).first()).toBeVisible();

  const measureViewport = () =>
    viewport.evaluate((el) => ({
      height: Math.round(el.getBoundingClientRect().height),
      rows: el.querySelectorAll(".xterm-rows > div").length,
    }));

  const defaultMetrics = await measureViewport();

  // 1. Drag terminal splitter UP to grow terminal height; verify live height + xterm rows update.
  const termSplitter = page.getByTestId("terminal-splitter");
  const termSplitterBox = (await termSplitter.boundingBox())!;
  await page.mouse.move(termSplitterBox.x + termSplitterBox.width / 2, termSplitterBox.y + termSplitterBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(termSplitterBox.x + termSplitterBox.width / 2, termSplitterBox.y - 120, { steps: 6 });
  await page.mouse.up();

  await expect.poll(async () => (await measureViewport()).height).toBeGreaterThan(defaultMetrics.height + 80);
  await expect.poll(async () => (await measureViewport()).rows).toBeGreaterThan(defaultMetrics.rows + 2);
  const customDragged = await measureViewport();

  // Maximize from custom height -> Restore must return to custom dragged height (preserving session output).
  await page.getByRole("button", { name: "Maximize terminal" }).click();
  await expect(page.getByRole("button", { name: "Restore terminal size" })).toBeVisible();
  await expect.poll(async () => (await measureViewport()).height).toBeGreaterThan(customDragged.height + 120);
  await page.getByRole("button", { name: "Restore terminal size" }).click();
  await expect.poll(async () => Math.abs((await measureViewport()).height - customDragged.height)).toBeLessThanOrEqual(4);
  await expect(viewport.getByText("drag-line-40", { exact: true }).first()).toBeVisible();

  // Dragging terminal splitter while maximized smoothly exits maximize to the dragged height.
  await page.getByRole("button", { name: "Maximize terminal" }).click();
  await expect(page.getByRole("button", { name: "Restore terminal size" })).toBeVisible();
  const maxSplitterBox = (await termSplitter.boundingBox())!;
  await page.mouse.move(maxSplitterBox.x + maxSplitterBox.width / 2, maxSplitterBox.y + maxSplitterBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(maxSplitterBox.x + maxSplitterBox.width / 2, maxSplitterBox.y + 180, { steps: 6 });
  await page.mouse.up();
  await expect(page.getByRole("button", { name: "Maximize terminal" })).toBeVisible();

  // Double-click terminal splitter resets to default height.
  await termSplitter.dblclick();
  await expect.poll(async () => Math.abs((await measureViewport()).height - defaultMetrics.height)).toBeLessThanOrEqual(4);

  // 2. File tree <-> editor splitter at 1440x900
  await page.getByRole("button", { name: "Files", exact: true }).click();
  const tree = page.getByRole("navigation", { name: "Files" });
  const initialTreeWidth = Math.round((await tree.boundingBox())!.width);
  const treeSplitter = page.getByTestId("file-tree-splitter");
  const treeSplitterBox = (await treeSplitter.boundingBox())!;
  await page.mouse.move(treeSplitterBox.x + treeSplitterBox.width / 2, treeSplitterBox.y + 40);
  await page.mouse.down();
  await page.mouse.move(treeSplitterBox.x + 90, treeSplitterBox.y + 40, { steps: 5 });
  await page.mouse.up();
  await expect.poll(async () => Math.round((await tree.boundingBox())!.width)).toBeGreaterThan(initialTreeWidth + 50);
  await treeSplitter.dblclick();
  await expect.poll(async () => Math.abs(Math.round((await tree.boundingBox())!.width) - initialTreeWidth)).toBeLessThanOrEqual(4);
  await page.getByRole("button", { name: "Files", exact: true }).click();

  // 3. Docked AI Tutor <-> editor splitter at 1440x900 preserves >=560px editor floor
  await page.getByRole("button", { name: "AI Tutor" }).click();
  const tutorDrawer = page.getByTestId("tutor-drawer");
  await expect(tutorDrawer).toHaveAttribute("data-layout", "docked");
  const initialTutorWidth = Math.round((await tutorDrawer.boundingBox())!.width);
  const tutorSplitter = page.getByTestId("tutor-splitter");
  const tutorSplitterBox = (await tutorSplitter.boundingBox())!;
  await page.mouse.move(tutorSplitterBox.x + tutorSplitterBox.width / 2, tutorSplitterBox.y + 80);
  await page.mouse.down();
  await page.mouse.move(tutorSplitterBox.x - 100, tutorSplitterBox.y + 80, { steps: 5 });
  await page.mouse.up();
  await expect.poll(async () => Math.round((await tutorDrawer.boundingBox())!.width)).toBeGreaterThan(initialTutorWidth + 50);
  const centerWidth = await page.locator(".pane-center").evaluate((el) => el.getBoundingClientRect().width);
  expect(centerWidth).toBeGreaterThanOrEqual(560);
  await tutorSplitter.dblclick();
  await expect.poll(async () => Math.abs(Math.round((await tutorDrawer.boundingBox())!.width) - initialTutorWidth)).toBeLessThanOrEqual(4);
  await page.getByRole("button", { name: "Close AI Tutor drawer" }).click();

  // 4. Narrow desktop viewport (1024x720): maximize/restore, terminal drag floor, and file-tree overlay mode
  await page.setViewportSize({ width: 1024, height: 720 });
  const narrowNormal = await measureViewport();
  await page.getByRole("button", { name: "Maximize terminal" }).click();
  await expect.poll(async () => (await measureViewport()).height).toBeGreaterThan(narrowNormal.height + 120);
  await expect.poll(async () => (await measureViewport()).rows).toBeGreaterThan(narrowNormal.rows + 4);
  const editorHeightWhenMaximized = await page.locator(".workspace-editor").evaluate((el) => el.getBoundingClientRect().height);
  expect(editorHeightWhenMaximized).toBeGreaterThanOrEqual(120);
  await page.getByRole("button", { name: "Restore terminal size" }).click();
  await expect.poll(async () => Math.abs((await measureViewport()).height - narrowNormal.height)).toBeLessThanOrEqual(4);

  // At 1024x720 (<735px editor container), opening Files switches tree into overlay mode and hides inline splitter
  await page.getByRole("button", { name: "Files", exact: true }).click();
  await expect(page.getByTestId("file-tree-splitter")).toBeHidden();
  const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(scrollWidth).toBeLessThanOrEqual(1024);

  expect(errors).toEqual([]);
});

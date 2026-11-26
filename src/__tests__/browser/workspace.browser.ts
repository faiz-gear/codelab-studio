import { expect, test } from "@playwright/test";

test("Monaco preserves a draft after a disk change and renders a side-by-side diff", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();
  await page.getByRole("button", { name: "Files", exact: true }).click();
  const tree = page.getByRole("navigation", { name: "Files" });
  await tree.getByRole("button", { name: "main.go", exact: true }).click();

  const firstLine = page.getByRole("tabpanel").locator(".monaco-editor .view-line").first();
  await expect(firstLine).toHaveText("package main");
  await firstLine.click();
  await page.keyboard.press("End");
  await page.keyboard.type(" // learner draft");
  await expect(page.getByRole("img", { name: "Unsaved changes" })).toBeVisible();

  const externalText = "package main\n// external edit\n";
  await page.evaluate((text) => window.workspaceSmoke.replaceMainFile(text), externalText);
  const conflict = page.getByRole("region", { name: "Changed on disk" });
  await expect(conflict).toBeVisible();
  await expect(firstLine).toHaveText("package main // learner draft");
  await conflict.getByRole("button", { name: "Keep draft" }).click();
  await expect(conflict).toBeHidden();
  expect(await page.evaluate(() => window.workspaceSmoke.readMainFile())).toBe(externalText);

  await firstLine.click();
  await page.keyboard.press("ControlOrMeta+s");
  await expect
    .poll(() => page.evaluate(() => window.workspaceSmoke.readMainFile()))
    .toBe("package main // learner draft\n");
  await expect(page.getByText("All changes saved", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Compare files" }).click();
  await tree.getByRole("button", { name: "other.go", exact: true }).click();
  const comparison = page.getByRole("region", { name: "main.go and other.go" });
  const diff = comparison.locator(".monaco-diff-editor.side-by-side");
  await expect(diff.getByText("package main // learner draft", { exact: true })).toBeVisible();
  await expect(diff.getByText("package second", { exact: true })).toBeVisible();
  await expect(diff.locator(".line-insert").first()).toBeVisible();
  expect(pageErrors).toEqual([]);
});

test("shows the reviewed Reference Solution in Monaco before enabling confirmation", async ({ page }) => {
  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();
  const trigger = page.getByRole("button", { name: "View reference hints and diff" });
  await trigger.click();

  const dialog = page.getByRole("dialog", { name: "Reference Solution" });
  const diff = dialog.locator(".monaco-diff-editor.side-by-side");
  await expect(diff.getByText("package main", { exact: true }).first()).toBeVisible();
  await expect(diff.getByText("// reference implementation", { exact: true })).toBeVisible();
  expect(await page.evaluate(() => window.workspaceSmoke.readMainFile())).toBe("package main\n");
  await expect(dialog.getByRole("button", { name: "Cancel" })).toBeFocused();

  await dialog.getByRole("button", { name: "Apply Reference Solution" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole("tabpanel").getByText("// reference implementation", { exact: true })).toBeVisible();
  expect(await page.evaluate(() => window.workspaceSmoke.readMainFile())).toBe("package main\n// reference implementation\n");
});

test("Monaco shows completion, hover documentation and diagnostic markers from CoreClient", async ({ page }) => {
  await page.goto("/src/__tests__/browser/workspace.html?language-server");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();
  await page.getByRole("button", { name: "Files", exact: true }).click();
  await page.getByRole("navigation", { name: "Files" }).getByRole("button", { name: "main.go", exact: true }).click();
  await expect(page.getByRole("button", { name: "Go LSP ready" })).toBeVisible();

  const editor = page.locator(".workspace-text-editor .monaco-editor");
  const line = editor.locator(".view-line").first();
  await expect(line).toHaveText("package main");
  await expect(editor.locator(".squiggly-warning")).toBeVisible();
  await line.hover({ position: { x: 8, y: 7 } });
  await expect(editor.locator(".monaco-hover").getByText("Fixture documentation")).toBeVisible();
  await expect(editor.locator(".monaco-hover").getByText("Fixture diagnostic")).toBeVisible();
  await line.click();
  await page.keyboard.press("End");
  await page.keyboard.press("Control+Space");
  await expect(editor.locator(".suggest-widget").getByText("mainFixture")).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(line).toHaveText("package mainFixture");
});

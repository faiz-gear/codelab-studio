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

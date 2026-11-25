import { expect, test } from "@playwright/test";

test("xterm renders streamed output and sends ordered input through CoreClient", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));

  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();
  await page.getByRole("button", { name: "New terminal" }).click();
  const terminal = page.getByRole("tabpanel", { name: /Terminal 1/ });
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
  expect(errors).toEqual([]);
});

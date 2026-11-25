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

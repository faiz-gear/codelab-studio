import { expect, test } from "@playwright/test";

test("Arc UI Select works consistently across Settings, Workspace toolbar, Import modal, and Top-Layer WorkspaceDialog in Light and Dark themes", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();

  // Zero native <select> controls remain in the workbench
  await expect(page.locator("select")).toHaveCount(0);

  // 1. Settings -> ACP Tutor preset Arc Select: keyboard open, navigate, Escape shields modal, Enter selects
  await page.getByRole("button", { name: "Settings" }).click();
  const settings = page.getByRole("dialog", { name: "Settings" });
  await settings.getByRole("tab", { name: "ACP Tutor" }).click();
  await expect(settings.locator("select")).toHaveCount(0);

  const presetCombo = settings.getByRole("combobox", { name: "Local ACP Agent preset" });
  await presetCombo.focus();
  await page.keyboard.press("ArrowDown");
  await expect(settings.getByRole("listbox")).toBeVisible();
  // First Escape closes only the Select menu and keeps Settings open
  await page.keyboard.press("Escape");
  await expect(settings.getByRole("listbox")).toHaveCount(0);
  await expect(settings).toBeVisible();
  await expect(presetCombo).toBeFocused();

  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(presetCombo).toHaveText(/Gemini CLI/);
  await expect(settings.getByLabel("Launch command")).toHaveValue("gemini --acp");

  // 2. Settings -> Editor & Desktop: switch to Dark theme and select Cursor via Arc Select
  await settings.getByRole("tab", { name: "Editor & Desktop" }).click();
  await settings.getByRole("button", { name: "Dark" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");

  const settingsIde = settings.getByRole("combobox", { name: "Preferred external IDE" });
  await settingsIde.click();
  const darkMenuBg = await settings
    .getByRole("listbox")
    .evaluate((el) => getComputedStyle(el).backgroundColor);
  expect(darkMenuBg).not.toBe("rgb(255, 255, 255)");
  await settings.getByRole("option", { name: "Cursor" }).click();
  await expect(settingsIde).toHaveText("Cursor");

  // Return to Workspace directly from Settings
  await settings.getByRole("button", { name: "Return to Workspace" }).click();
  await expect(settings).toHaveCount(0);

  // 3. Workspace toolbar IDE Arc Select reflects saved Cursor and updates back to GoLand
  const toolbarIde = page.getByRole("combobox", { name: "External IDE" });
  await expect(toolbarIde).toHaveText("Cursor");
  await toolbarIde.click();
  await page.getByRole("option", { name: "GoLand" }).click();
  await expect(toolbarIde).toHaveText("GoLand");

  // 4. Top-Layer WorkspaceDialog (<dialog>.showModal()): Arc Select popover stacks ABOVE the modal and is clickable
  await page.getByRole("button", { name: "View reference hints and diff" }).click();
  const refDialog = page.getByRole("dialog", { name: "Reference Solution" });
  await expect(refDialog).toBeVisible();
  const filePicker = refDialog.getByRole("combobox", { name: "File to compare" });
  await filePicker.click();
  const fileOption = refDialog.getByRole("option", { name: "main.go" });
  await expect(fileOption).toBeVisible();
  const optBox = (await fileOption.boundingBox())!;
  const topHitIsOption = await page.evaluate(
    ({ x, y }) => Boolean(document.elementFromPoint(x, y)?.closest('[role="option"]')),
    { x: optBox.x + optBox.width / 2, y: optBox.y + optBox.height / 2 }
  );
  expect(topHitIsOption).toBe(true);
  await fileOption.click();
  await expect(refDialog.getByRole("listbox")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(refDialog).toHaveCount(0);

  // 5. Import modal Author Course Pack language Arc Select + harmonized checkbox
  await page.getByRole("button", { name: "Switch or import Course" }).click();
  await page.getByRole("menuitem", { name: /Import Local Markdown/ }).click();
  const importDialog = page.getByRole("dialog", { name: "Import or Author a Course Pack" });
  await importDialog.getByRole("button", { name: "Author Course Pack" }).click();
  await expect(importDialog.locator("select")).toHaveCount(0);
  const langCombo = importDialog.getByRole("combobox", { name: "Language" });
  await langCombo.click();
  await importDialog.getByRole("option", { name: "Python" }).click();
  await expect(langCombo).toHaveText("Python");
  const refCheckbox = importDialog.getByRole("checkbox", { name: "Include a Reference Solution" });
  await expect(refCheckbox).toHaveClass(/arc-checkbox/);
  await refCheckbox.click();
  await expect(refCheckbox).toBeChecked();
  await page.keyboard.press("Escape");

  expect(errors).toEqual([]);
});

test("Settings category navigation, inactive tab collapse (#44), task-grouped ACP Tutor, and Return to Workspace work cleanly at 1440x900 and 1024x720", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();

  // Create editor draft and terminal output to verify Return to Workspace preserves live state
  await page.getByRole("button", { name: "Files", exact: true }).click();
  await page.getByRole("navigation", { name: "Files" }).getByRole("button", { name: "main.go", exact: true }).click();
  const firstLine = page.locator(".workspace-text-editor .monaco-editor .view-line").first();
  await expect(firstLine).toHaveText("package main");
  await firstLine.click();
  await page.keyboard.press("End");
  await page.keyboard.type(" // keep-me");
  await page.getByRole("button", { name: "New terminal" }).click();
  await page.evaluate(() => window.workspaceSmoke.writeTerminalOutput("kept-terminal-output\r\n"));
  await expect(page.getByText("kept-terminal-output", { exact: true }).first()).toBeVisible();

  for (const vp of [
    { name: "1440x900", width: 1440, height: 900 },
    { name: "1024x720", width: 1024, height: 720 },
  ]) {
    await page.setViewportSize({ width: vp.width, height: vp.height });
    await page.getByRole("button", { name: "Settings" }).click();
    const dialog = page.getByRole("dialog", { name: "Settings" });
    await expect(dialog).toBeVisible();

    // Verify all 4 tabs align at the top and hidden panels take 0px height (#44 compatibility)
    const tabs = ["ACP Tutor", "4-Language Runtimes", "Editor & Desktop", "Privacy & Local Data"];
    for (const tabName of tabs) {
      await dialog.getByRole("tab", { name: tabName }).click();
      const metrics = await dialog.evaluate((el) => {
        const host = el.querySelector<HTMLElement>(".settings-panel-host")!;
        const hostTop = host.getBoundingClientRect().top;
        const panels = Array.from(el.querySelectorAll<HTMLElement>('[role="tabpanel"]'));
        const hiddenHeights = panels
          .filter((p) => p.hidden)
          .map((p) => ({
            id: p.id,
            height: Math.round(p.getBoundingClientRect().height),
            display: getComputedStyle(p).display,
          }));
        const active = panels.find((p) => !p.hidden)!;
        const firstChildTop = active.firstElementChild
          ? active.firstElementChild.getBoundingClientRect().top - hostTop
          : 0;
        return {
          hiddenHeights,
          firstChildTop: Math.round(firstChildTop),
          overflowX: active.scrollWidth > active.clientWidth + 1,
        };
      });
      for (const hidden of metrics.hiddenHeights) {
        expect(hidden.height, `${vp.name} ${tabName} hidden ${hidden.id} height`).toBe(0);
        expect(hidden.display, `${vp.name} ${tabName} hidden ${hidden.id} display`).toBe("none");
      }
      expect(metrics.firstChildTop, `${vp.name} ${tabName} starts near top`).toBeLessThanOrEqual(28);
      expect(metrics.overflowX, `${vp.name} ${tabName} horizontal overflow`).toBe(false);
    }

    // Verify ACP Tutor task-grouped sections & keyboard accessibility (including narrow 1024x720)
    await dialog.getByRole("tab", { name: "ACP Tutor" }).click();
    const tutorPanel = dialog.getByTestId("tutor-settings-panel");
    expect(await tutorPanel.textContent()).not.toMatch(/BYOK|API Key|Keychain/i);
    await expect(tutorPanel.getByRole("heading", { name: "ACP Agent setup" })).toBeVisible();
    await expect(tutorPanel.getByRole("heading", { name: "Handshake test & live feedback" })).toBeVisible();
    await expect(tutorPanel.getByRole("heading", { name: "Permission Policy" })).toBeVisible();
    await expect(tutorPanel.getByRole("heading", { name: "Teaching style" })).toBeVisible();

    // Teaching style segmented control uses Arc UI .seg styling (bordered recessed tray)
    const styleSeg = tutorPanel.getByRole("group", { name: "Switch Tutor teaching style" });
    await expect(styleSeg).toHaveClass(/\bseg\b/);

    // Keyboard radiogroup & handshake test work in narrow viewport
    const guidedRadio = tutorPanel.getByRole("radio", { name: /Guided Read-Only/ });
    await guidedRadio.focus();
    await page.keyboard.press("ArrowRight");
    await expect(tutorPanel.getByRole("radio", { name: /Strict Ask-All/ })).toHaveAttribute("aria-checked", "true");
    await page.keyboard.press("ArrowLeft");
    await expect(guidedRadio).toHaveAttribute("aria-checked", "true");

    await tutorPanel.getByRole("button", { name: "Test ACP Handshake" }).click();
    await expect(tutorPanel.getByRole("status")).toContainText("ACP handshake succeeded");

    // Return directly to Workspace preserving unsaved editor draft and terminal output
    await dialog.getByRole("button", { name: "Return to Workspace" }).click();
    await expect(dialog).toHaveCount(0);
    await expect(firstLine).toHaveText("package main // keep-me");
    await expect(page.getByText("kept-terminal-output", { exact: true }).first()).toBeVisible();
  }

  expect(errors).toEqual([]);
});

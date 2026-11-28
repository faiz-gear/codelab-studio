import { expect, test, type Locator, type Page } from "@playwright/test";

const VIEWPORTS = [
  { name: "1024x720", width: 1024, height: 720 },
  { name: "1440x900", width: 1440, height: 900 },
] as const;

const SETTINGS_TABS = [
  "ACP Tutor",
  "4-Language Runtimes",
  "Editor & Desktop",
  "Privacy & Local Data",
] as const;

async function openBoundGoWorkspace(page: Page) {
  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();
  await expect(page.getByRole("button", { name: "Files", exact: true })).toBeVisible();
}

interface PanelLayoutSnapshot {
  dialogBottom: number;
  viewportHeight: number;
  hostTop: number;
  hostHeight: number;
  activePanelTop: number;
  activePanelHeight: number;
  activeFirstChildTop: number;
  activeScrollHeight: number;
  activeClientHeight: number;
  activeOverflowY: string;
  inactivePanels: Array<{ id: string; width: number; height: number; display: string }>;
}

async function measureSettingsPanels(dialog: Locator): Promise<PanelLayoutSnapshot> {
  return dialog.evaluate((dialogEl) => {
    const dialogRect = dialogEl.getBoundingClientRect();
    const host = dialogEl.querySelector<HTMLElement>(".settings-panel-host");
    if (!host) throw new Error("Missing .settings-panel-host");
    const hostRect = host.getBoundingClientRect();

    const panels = Array.from(host.querySelectorAll<HTMLElement>('[role="tabpanel"]'));
    const active = panels.find((panel) => !panel.hidden);
    if (!active) throw new Error("Missing active Settings tabpanel");
    const activeRect = active.getBoundingClientRect();
    const firstChild = active.firstElementChild as HTMLElement | null;
    if (!firstChild) throw new Error("Active Settings tabpanel has no rendered content");
    const firstChildRect = firstChild.getBoundingClientRect();

    const inactivePanels = panels
      .filter((panel) => panel.hidden)
      .map((panel) => {
        const rect = panel.getBoundingClientRect();
        return {
          id: panel.id,
          width: Math.round(rect.width),
          height: Math.round(rect.height),
          display: getComputedStyle(panel).display,
        };
      });

    return {
      dialogBottom: Math.round(dialogRect.bottom),
      viewportHeight: window.innerHeight,
      hostTop: hostRect.top,
      hostHeight: hostRect.height,
      activePanelTop: activeRect.top,
      activePanelHeight: activeRect.height,
      activeFirstChildTop: firstChildRect.top,
      activeScrollHeight: active.scrollHeight,
      activeClientHeight: active.clientHeight,
      activeOverflowY: getComputedStyle(active).overflowY,
      inactivePanels,
    };
  });
}

test("inactive Settings tab panels occupy zero layout space, all four tabs align to the top at 1024x720 and 1440x900, and long panels scroll inside the dialog", async ({
  page,
}) => {
  await openBoundGoWorkspace(page);

  for (const vp of VIEWPORTS) {
    await page.setViewportSize({ width: vp.width, height: vp.height });

    const settingsBtn = page.getByRole("button", { name: "Settings" });
    await settingsBtn.click();
    const dialog = page.getByRole("dialog", { name: "Settings" });
    await expect(dialog).toBeVisible();

    let baselineContentOffset: number | null = null;
    let verifiedScrollablePanel = false;

    for (const tabName of SETTINGS_TABS) {
      await dialog.getByRole("tab", { name: tabName }).click();
      if (tabName === "Privacy & Local Data") {
        await expect(dialog.getByTestId("local-metrics-grid")).toBeVisible();
      }

      const m = await measureSettingsPanels(dialog);

      expect(m.inactivePanels, `${vp.name} ${tabName} inactive panel count`).toHaveLength(3);
      for (const inactive of m.inactivePanels) {
        expect(
          inactive.height,
          `${vp.name} ${tabName}: hidden panel ${inactive.id} must not reserve vertical space`
        ).toBe(0);
        expect(
          inactive.width,
          `${vp.name} ${tabName}: hidden panel ${inactive.id} must not reserve horizontal space`
        ).toBe(0);
        expect(
          inactive.display,
          `${vp.name} ${tabName}: hidden panel ${inactive.id} must have display: none`
        ).toBe("none");
      }

      expect(
        Math.abs(m.activePanelTop - m.hostTop),
        `${vp.name} ${tabName}: active tabpanel must start at top of .settings-panel-host`
      ).toBeLessThanOrEqual(1);
      expect(
        Math.abs(m.activePanelHeight - m.hostHeight),
        `${vp.name} ${tabName}: active tabpanel must occupy full .settings-panel-host height`
      ).toBeLessThanOrEqual(1);

      const contentOffset = Math.round(m.activeFirstChildTop - m.hostTop);
      if (baselineContentOffset === null) {
        baselineContentOffset = contentOffset;
      }
      expect(
        contentOffset,
        `${vp.name} ${tabName}: first content must start near panel host top (expected ~18px padding)`
      ).toBeLessThanOrEqual(24);
      expect(
        Math.abs(contentOffset - baselineContentOffset),
        `${vp.name} ${tabName}: first content top offset (${contentOffset}px) must match ACP Tutor (${baselineContentOffset}px)`
      ).toBeLessThanOrEqual(2);

      expect(
        m.dialogBottom,
        `${vp.name} ${tabName}: Settings dialog must stay within viewport`
      ).toBeLessThanOrEqual(m.viewportHeight);

      if (m.activeScrollHeight > m.activeClientHeight + 1) {
        expect(["auto", "scroll"]).toContain(m.activeOverflowY);
        const scrolledTop = await dialog.evaluate((dialogEl) => {
          const active = Array.from(
            dialogEl.querySelectorAll<HTMLElement>('.settings-panel-host [role="tabpanel"]')
          ).find((panel) => !panel.hidden);
          if (!active) return 0;
          active.scrollTop = active.scrollHeight;
          const after = active.scrollTop;
          active.scrollTop = 0;
          return after;
        });
        expect(
          scrolledTop,
          `${vp.name} ${tabName}: overflowing active tabpanel must scroll within the dialog`
        ).toBeGreaterThan(0);
        verifiedScrollablePanel = true;
      }
    }

    expect(
      verifiedScrollablePanel,
      `${vp.name}: expected at least one long Settings panel to overflow and scroll within the dialog`
    ).toBe(true);

    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await expect(settingsBtn).toBeFocused();
  }
});

test("supports keyboard tab navigation, live settings changes, and Escape focus return in Settings", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openBoundGoWorkspace(page);

  const settingsBtn = page.getByRole("button", { name: "Settings" });
  await settingsBtn.click();
  const dialog = page.getByRole("dialog", { name: "Settings" });
  await expect(dialog).toBeVisible();

  const runtimeTab = dialog.getByRole("tab", { name: "4-Language Runtimes" });
  const editorTab = dialog.getByRole("tab", { name: "Editor & Desktop" });
  const privacyTab = dialog.getByRole("tab", { name: "Privacy & Local Data" });
  const tutorTab = dialog.getByRole("tab", { name: "ACP Tutor" });

  await expect(runtimeTab).toHaveAttribute("aria-selected", "true");
  await expect(runtimeTab).toBeFocused();

  await page.keyboard.press("ArrowRight");
  await expect(editorTab).toHaveAttribute("aria-selected", "true");
  await expect(editorTab).toBeFocused();

  await dialog.getByRole("button", { name: "Dark" }).click();
  await dialog.getByRole("button", { name: "Large 15" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  const codeSize = await page.evaluate(() =>
    document.documentElement.style.getPropertyValue("--code-size").trim()
  );
  expect(codeSize).toBe("15px");

  // Return focus to active tab to verify End / Home / ArrowLeft keyboard nav
  await editorTab.focus();
  await page.keyboard.press("End");
  await expect(privacyTab).toHaveAttribute("aria-selected", "true");
  await expect(privacyTab).toBeFocused();
  await expect(dialog.getByTestId("local-metrics-grid")).toBeVisible();

  await page.keyboard.press("Home");
  await expect(tutorTab).toHaveAttribute("aria-selected", "true");
  await expect(tutorTab).toBeFocused();
  await expect(dialog.getByTestId("tutor-settings-panel")).toBeVisible();

  await page.keyboard.press("ArrowLeft");
  await expect(privacyTab).toHaveAttribute("aria-selected", "true");
  await expect(privacyTab).toBeFocused();

  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(settingsBtn).toBeFocused();
});

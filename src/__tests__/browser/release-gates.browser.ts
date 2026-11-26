import { expect, test, type Page } from "@playwright/test";

const BREAKPOINTS = [
  { name: "1024x720", width: 1024, height: 720 },
  { name: "1280x800", width: 1280, height: 800 },
  { name: "1320x800", width: 1320, height: 800 },
  { name: "1440x900", width: 1440, height: 900 },
] as const;

async function openWorkbenchExercise1(page: Page) {
  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button", { name: "Start Built-in Go / GoFrame Lab" }).click();
  await page.getByRole("button", { name: "Choose Workspace" }).click();
  // Navigate to Exercise 1 (contains a daemon Command Block with cwd + badge + actions)
  await page.getByRole("button", { name: "Next Chapter" }).click();
  await expect(page.getByTestId("command-block-ex-1:cmd-1")).toBeVisible();
}

function rectsOverlap(
  a: { x: number; y: number; width: number; height: number },
  b: { x: number; y: number; width: number; height: number }
): boolean {
  return (
    a.x < b.x + b.width &&
    a.x + a.width > b.x &&
    a.y < b.y + b.height &&
    a.y + a.height > b.y
  );
}

test("enforces zero external telemetry network requests across settings, workbench, and tutor interactions", async ({
  page,
}) => {
  const externalRequests: string[] = [];
  await page.route("**/*", async (route) => {
    const url = new URL(route.request().url());
    const isLocal =
      url.hostname === "127.0.0.1" ||
      url.hostname === "localhost" ||
      url.protocol === "data:" ||
      url.protocol === "blob:";
    if (!isLocal) {
      externalRequests.push(route.request().url());
      await route.abort();
      return;
    }
    await route.continue();
  });

  await openWorkbenchExercise1(page);
  await page.getByRole("button", { name: "Files", exact: true }).click();
  await page.getByRole("button", { name: "Settings" }).click();
  const settings = page.getByRole("dialog", { name: "Settings" });
  await settings.getByRole("tab", { name: "ACP Tutor" }).click();
  await settings.getByRole("tab", { name: "Editor & Desktop" }).click();
  await settings.getByRole("button", { name: "Dark" }).click();
  await settings.getByRole("button", { name: "Violet" }).click();
  await settings.getByRole("tab", { name: "Privacy & Local Data" }).click();
  await expect(settings.getByTestId("local-metrics-grid")).toBeVisible();
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "AI Tutor" }).click();
  await expect(page.getByTestId("tutor-drawer")).toBeVisible();
  expect(externalRequests).toEqual([]);
});

test("maintains zero horizontal overflow, single-row TopBar, unclipped Command Block headers, adaptive Tutor/Tree layouts, and non-overlapping Toasts across 1024x720..1440x900 in zh-CN and en-US", async ({
  page,
}) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.setViewportSize({ width: 1440, height: 900 });
  await openWorkbenchExercise1(page);

  for (const locale of ["en-US", "zh-CN"] as const) {
    await page.evaluate((loc) => window.workspaceSmoke.setLocale(loc), locale);

    for (const bp of BREAKPOINTS) {
      await page.setViewportSize({ width: bp.width, height: bp.height });

      // 1. Zero horizontal overflow & single-row TopBar with zero wrapped or clipped items
      const layoutMetrics = await page.evaluate(() => {
        const doc = document.documentElement;
        const topbar = document.querySelector(".topbar")!;
        const topbarRect = topbar.getBoundingClientRect();
        const topbarControls = Array.from(
          topbar.querySelectorAll<HTMLElement>(":scope > button, .course-trigger, .topbar-progress, .top-right > button, .top-right .preflight-anchor > button")
        ).map((el) => {
          const r = el.getBoundingClientRect();
          return {
            text: el.textContent?.trim() ?? "",
            top: r.top,
            bottom: r.bottom,
            right: r.right,
            height: r.height,
          };
        });

        const cmdHeads = Array.from(document.querySelectorAll<HTMLElement>(".cmd-card")).map(
          (card) => {
            const cardRect = card.getBoundingClientRect();
            const head = card.querySelector<HTMLElement>(".cmd-head")!;
            const headRect = head.getBoundingClientRect();
            const actions = card.querySelector<HTMLElement>(".cmd-actions")!;
            const actionsRect = actions.getBoundingClientRect();
            const buttons = Array.from(actions.querySelectorAll<HTMLElement>("button")).map(
              (btn) => {
                const br = btn.getBoundingClientRect();
                return { right: br.right, left: br.left, height: br.height };
              }
            );
            return {
              cardRight: cardRect.right,
              headHeight: headRect.height,
              actionsRight: actionsRect.right,
              buttons,
            };
          }
        );

        return {
          scrollWidth: doc.scrollWidth,
          innerWidth: window.innerWidth,
          topbarHeight: topbarRect.height,
          topbarBottom: topbarRect.bottom,
          topbarControls,
          cmdHeads,
        };
      });

      expect(
        layoutMetrics.scrollWidth,
        `${locale} ${bp.name} horizontal overflow`
      ).toBeLessThanOrEqual(layoutMetrics.innerWidth);
      expect(
        layoutMetrics.topbarHeight,
        `${locale} ${bp.name} topbar height`
      ).toBeLessThanOrEqual(49);

      for (const control of layoutMetrics.topbarControls) {
        expect(
          control.right,
          `${locale} ${bp.name} topbar control "${control.text}" clipped horizontally`
        ).toBeLessThanOrEqual(bp.width + 1);
        expect(
          control.bottom,
          `${locale} ${bp.name} topbar control "${control.text}" wrapped onto second row`
        ).toBeLessThanOrEqual(layoutMetrics.topbarBottom + 1);
      }

      expect(layoutMetrics.cmdHeads.length).toBeGreaterThan(0);
      for (const cmd of layoutMetrics.cmdHeads) {
        expect(
          cmd.headHeight,
          `${locale} ${bp.name} command head wrapped`
        ).toBeLessThanOrEqual(40);
        expect(
          cmd.actionsRight,
          `${locale} ${bp.name} command actions clipped`
        ).toBeLessThanOrEqual(cmd.cardRight + 1);
      }
    }
  }

  // Switch back to en-US for deterministic accessible labels in layout transitions
  await page.evaluate(() => window.workspaceSmoke.setLocale("en-US"));

  // 2. <1280px (1024x720): AI Tutor drawer floats as overlay
  await page.setViewportSize({ width: 1024, height: 720 });
  const tutorBtn = page.getByRole("button", { name: "AI Tutor" });
  await tutorBtn.click();
  const tutorDrawer = page.getByTestId("tutor-drawer");
  await expect(tutorDrawer).toBeVisible();
  await expect(tutorDrawer).toHaveAttribute("data-layout", "overlay");

  // Trigger a Toast (Copy command) and verify top-center Toast does NOT overlap AI composer
  await page.getByRole("button", { name: "Copy command" }).first().click();
  const toast = page.locator(".toast").first();
  await expect(toast).toBeVisible();
  const toastBox = await toast.boundingBox();
  const composerBox = await page.locator(".tutor-composer-wrap").boundingBox();
  expect(toastBox).not.toBeNull();
  expect(composerBox).not.toBeNull();
  expect(rectsOverlap(toastBox!, composerBox!)).toBe(false);
  await page.getByRole("button", { name: "Close AI Tutor drawer" }).click();

  // 3. 1280-1360px (1320x800): opening AI Tutor auto-collapses file tree and keeps >=520px editor floor
  await page.setViewportSize({ width: 1320, height: 800 });
  const filesBtn = page.getByRole("button", { name: "Files", exact: true });
  await filesBtn.click();
  await expect(filesBtn).toHaveAttribute("aria-expanded", "true");
  await tutorBtn.click();
  await expect(tutorDrawer).toHaveAttribute("data-layout", "docked");
  await expect(filesBtn).toHaveAttribute("aria-expanded", "false");
  const centerWidth1320 = await page
    .locator(".pane-center")
    .evaluate((el) => el.getBoundingClientRect().width);
  expect(centerWidth1320).toBeGreaterThanOrEqual(520);

  // 4. 1440x900: docked 3-pane keeps >=560px editor floor; resizing back to 1320 collapses open file tree
  await page.setViewportSize({ width: 1440, height: 900 });
  const centerWidth1440 = await page
    .locator(".pane-center")
    .evaluate((el) => el.getBoundingClientRect().width);
  expect(centerWidth1440).toBeGreaterThanOrEqual(560);
  await filesBtn.click();
  await expect(filesBtn).toHaveAttribute("aria-expanded", "true");
  await page.setViewportSize({ width: 1320, height: 800 });
  await expect(filesBtn).toHaveAttribute("aria-expanded", "false");

  // 5. Top-center Toast does not overlap modal action footer
  await page.getByRole("button", { name: "Copy command" }).first().click();
  await page.getByRole("button", { name: "Switch or import Course" }).click();
  await page
    .getByRole("menuitem", { name: /Import Local Markdown/ })
    .click();
  const modalFoot = page.locator(".modal-foot");
  await expect(modalFoot).toBeVisible();
  const modalFootBox = await modalFoot.boundingBox();
  const activeToastBox = await page.locator(".toast").first().boundingBox();
  if (activeToastBox && modalFootBox) {
    expect(rectsOverlap(activeToastBox, modalFootBox)).toBe(false);
  }
  await page.keyboard.press("Escape");

  // 6. Toasts triggered inside Settings modal render above the modal (not behind <dialog> top layer)
  await page.getByRole("button", { name: "Settings" }).click();
  const settingsDialog = page.getByRole("dialog", { name: "Settings" });
  await settingsDialog.getByRole("tab", { name: "Privacy & Local Data" }).click();
  await settingsDialog.getByRole("button", { name: "Clear Tutor history" }).click();
  const settingsToast = page.locator(".toast").first();
  await expect(settingsToast).toBeVisible();
  const settingsToastBox = await settingsToast.boundingBox();
  expect(settingsToastBox).not.toBeNull();
  const topElementIsToast = await page.evaluate(({ x, y }) => {
    const el = document.elementFromPoint(x, y);
    return Boolean(el?.closest(".toast"));
  }, {
    x: settingsToastBox!.x + settingsToastBox!.width / 2,
    y: settingsToastBox!.y + settingsToastBox!.height / 2,
  });
  expect(topElementIsToast).toBe(true);
  await page.keyboard.press("Escape");

  expect(pageErrors).toEqual([]);
});

test("displays selection Ask-AI pill within 150ms, supports keyboard shortcuts & focus-visible, and honors prefers-reduced-motion", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openWorkbenchExercise1(page);

  // Measure selection Ask-AI pill response time <= 150ms
  const elapsedMs = await page.evaluate(async () => {
    const target = document.querySelector(".manual-inner .md-prose p")!;
    const textNode = target.firstChild!;
    const range = document.createRange();
    range.setStart(textNode, 0);
    range.setEnd(textNode, Math.min(8, textNode.textContent?.length ?? 0));
    const sel = window.getSelection()!;
    sel.removeAllRanges();
    sel.addRange(range);

    const started = performance.now();
    target.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));

    return await new Promise<number>((resolve, reject) => {
      const deadline = performance.now() + 1000;
      const poll = () => {
        if (document.querySelector('[data-testid="selection-ask-pill"]')) {
          resolve(performance.now() - started);
          return;
        }
        if (performance.now() > deadline) {
          reject(new Error("Selection Ask-AI pill did not appear"));
          return;
        }
        requestAnimationFrame(poll);
      };
      requestAnimationFrame(poll);
    });
  });
  expect(elapsedMs).toBeLessThanOrEqual(150);

  const pill = page.getByTestId("selection-ask-pill");
  await expect(pill).toBeVisible();
  await pill.getByRole("button", { name: "Explain concept" }).click();
  await expect(page.getByTestId("tutor-drawer")).toBeVisible();
  await page.getByRole("button", { name: "Close AI Tutor drawer" }).click();

  // Keyboard Focus Mode shortcut (⌘/Ctrl + B)
  const workbench = page.getByTestId("course-workbench");
  await page.keyboard.press("ControlOrMeta+b");
  await expect(workbench).toHaveClass(/is-focus/);
  await page.keyboard.press("ControlOrMeta+b");
  await expect(workbench).not.toHaveClass(/is-focus/);

  // Keyboard Settings modal focus restoration + :focus-visible ring
  const settingsBtn = page.getByRole("button", { name: "Settings" });
  await settingsBtn.click();
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(settingsBtn).toBeFocused();
  const focusRingShadow = await settingsBtn.evaluate(
    (el) => getComputedStyle(el).boxShadow
  );
  expect(focusRingShadow).not.toBe("none");

  // prefers-reduced-motion collapses all duration tokens to 0ms
  await page.emulateMedia({ reducedMotion: "reduce" });
  const durations = await page.evaluate(() => {
    const cs = getComputedStyle(document.documentElement);
    return [
      cs.getPropertyValue("--duration-instant").trim(),
      cs.getPropertyValue("--duration-fast").trim(),
      cs.getPropertyValue("--duration-standard").trim(),
      cs.getPropertyValue("--duration-considered").trim(),
      cs.getPropertyValue("--duration-spring").trim(),
    ];
  });
  expect(durations).toEqual(["0ms", "0ms", "0ms", "0ms", "0ms"]);
});

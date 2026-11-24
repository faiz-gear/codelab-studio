import type { GlobalSettings } from "../core/contracts";

export const APPEARANCE_CACHE_KEY = "codelab.appearance.v1";

export function applySettingsToDocument(
  settings: GlobalSettings,
  options: { disableTransitions?: boolean } = {}
): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  const themeChanged = root.dataset.theme !== settings.theme;

  if (options.disableTransitions && themeChanged) {
    root.dataset.themeSwitching = "true";
  }

  root.dataset.theme = settings.theme;
  root.dataset.accent = settings.accent;
  root.lang = settings.locale;
  root.style.setProperty("--code-size", `${settings.codeFontSize}px`);
  root.style.setProperty(
    "--code-lh",
    `${Math.round(settings.codeFontSize + 8.5)}px`
  );

  if (options.disableTransitions && themeChanged) {
    // Force style flush then re-enable transitions on the next animation frame
    void root.offsetHeight;
    requestAnimationFrame(() => {
      delete root.dataset.themeSwitching;
    });
  }

  try {
    window.localStorage.setItem(
      APPEARANCE_CACHE_KEY,
      JSON.stringify({
        theme: settings.theme,
        accent: settings.accent,
        codeFontSize: settings.codeFontSize,
        locale: settings.locale,
      })
    );
  } catch {
    // Ignore restricted storage environments.
  }
}

export function buildThemeConfigSnippet(settings: GlobalSettings): string {
  const lineHeight = Math.round(settings.codeFontSize + 8.5);
  return [
    `/* CodeLab Studio · Arc UI v3.2 Theme Configuration */`,
    `<html lang="${settings.locale}" data-theme="${settings.theme}" data-accent="${settings.accent}">`,
    `:root {`,
    `  --code-size: ${settings.codeFontSize}px;`,
    `  --code-lh: ${lineHeight}px;`,
    `}`,
  ].join("\n");
}

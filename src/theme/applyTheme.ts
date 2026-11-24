import {
  ACCENT_PRESETS,
  DEFAULT_SETTINGS,
  type AccentPreset,
  type GlobalSettings,
  type Locale,
  type ThemeMode,
} from "../core/contracts";

export const APPEARANCE_CACHE_KEY = "codelab.appearance.v1";

const VALID_THEMES = new Set<ThemeMode>(["light", "dark"]);
const VALID_ACCENTS = new Set<AccentPreset>(ACCENT_PRESETS);
const VALID_LOCALES = new Set<Locale>(["zh-CN", "en-US"]);

export function computeCodeLineHeight(size: number): number {
  return Math.round(size + 8.5);
}

export function readCachedSettings(): GlobalSettings {
  if (typeof window === "undefined") return { ...DEFAULT_SETTINGS };
  try {
    const raw = window.localStorage.getItem(APPEARANCE_CACHE_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    const parsed = JSON.parse(raw) as Partial<GlobalSettings> | null;
    if (!parsed || typeof parsed !== "object") return { ...DEFAULT_SETTINGS };

    const theme =
      parsed.theme && VALID_THEMES.has(parsed.theme)
        ? parsed.theme
        : DEFAULT_SETTINGS.theme;
    const accent =
      parsed.accent && VALID_ACCENTS.has(parsed.accent)
        ? parsed.accent
        : DEFAULT_SETTINGS.accent;
    const codeFontSize =
      typeof parsed.codeFontSize === "number" &&
      Number.isFinite(parsed.codeFontSize) &&
      parsed.codeFontSize >= 11 &&
      parsed.codeFontSize <= 20
        ? parsed.codeFontSize
        : DEFAULT_SETTINGS.codeFontSize;
    const locale =
      parsed.locale && VALID_LOCALES.has(parsed.locale)
        ? parsed.locale
        : DEFAULT_SETTINGS.locale;

    return { theme, accent, codeFontSize, locale };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

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
    `${computeCodeLineHeight(settings.codeFontSize)}px`
  );

  if (options.disableTransitions && themeChanged) {
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
  const lineHeight = computeCodeLineHeight(settings.codeFontSize);
  return [
    `/* CodeLab Studio · Arc UI v3.2 Theme Configuration */`,
    `<html lang="${settings.locale}" data-theme="${settings.theme}" data-accent="${settings.accent}">`,
    `:root {`,
    `  --code-size: ${settings.codeFontSize}px;`,
    `  --code-lh: ${lineHeight}px;`,
    `}`,
  ].join("\n");
}

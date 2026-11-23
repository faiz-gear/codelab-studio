import { useEffect, useState } from "react";
import { LOCALES, type Locale, resolveInitialLocale, t } from "./i18n";

const THEMES = ["light", "dark"] as const;
type ThemeMode = (typeof THEMES)[number];

const ACCENTS = [
  "neutral",
  "violet",
  "blue",
  "green",
  "amber",
  "orange",
  "coral",
  "rose",
] as const;
type AccentPreset = (typeof ACCENTS)[number];

const THEME_STORAGE_KEY = "codelab.theme";
const ACCENT_STORAGE_KEY = "codelab.accent";

function isThemeMode(value: string | undefined): value is ThemeMode {
  return value === "light" || value === "dark";
}

function isAccentPreset(value: string | undefined): value is AccentPreset {
  return (
    value === "neutral" ||
    value === "violet" ||
    value === "blue" ||
    value === "green" ||
    value === "amber" ||
    value === "orange" ||
    value === "coral" ||
    value === "rose"
  );
}

function readInitialTheme(): ThemeMode {
  const current = document.documentElement.dataset.theme;
  return isThemeMode(current) ? current : "light";
}

function readInitialAccent(): AccentPreset {
  const current = document.documentElement.dataset.accent;
  return isAccentPreset(current) ? current : "neutral";
}

function persistPreference(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Ignore storage quota or sandbox errors.
  }
}

export function App() {
  const [locale, setLocale] = useState<Locale>(() => resolveInitialLocale(navigator.language));
  const [theme, setTheme] = useState<ThemeMode>(readInitialTheme);
  const [accent, setAccent] = useState<AccentPreset>(readInitialAccent);
  const [viewport, setViewport] = useState(() => ({
    width: window.innerWidth,
    height: window.innerHeight,
  }));

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    persistPreference(THEME_STORAGE_KEY, theme);
  }, [theme]);

  useEffect(() => {
    document.documentElement.dataset.accent = accent;
    persistPreference(ACCENT_STORAGE_KEY, accent);
  }, [accent]);

  useEffect(() => {
    const handleResize = () => {
      setViewport({
        width: window.innerWidth,
        height: window.innerHeight,
      });
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  return (
    <main className="min-h-screen bg-background text-foreground flex items-center justify-center p-6">
      <section className="w-full max-w-xl rounded-panel border border-border bg-surface p-6 shadow-resting space-y-5">
        <header className="space-y-1">
          <h1 className="font-display text-2xl font-semibold tracking-tight text-foreground">
            {t(locale, "shell.title")}
          </h1>
          <p className="text-sm text-text-secondary">{t(locale, "shell.subtitle")}</p>
        </header>

        <div className="grid grid-cols-2 gap-3 rounded-control border border-border-subtle bg-surface-muted p-3 text-sm">
          <div>
            <div className="text-xs text-text-muted">{t(locale, "shell.locale.label")}</div>
            <div className="font-mono text-foreground" data-testid="shell-locale">
              {locale}
            </div>
          </div>
          <div>
            <div className="text-xs text-text-muted">{t(locale, "shell.viewport.label")}</div>
            <div className="font-mono text-foreground" data-testid="shell-viewport">
              {viewport.width} x {viewport.height}
            </div>
          </div>
        </div>

        <div className="space-y-2">
          <div className="text-xs font-medium text-text-secondary">
            {t(locale, "shell.locale.label")}
          </div>
          <div className="flex flex-wrap gap-2">
            {LOCALES.map((option) => {
              const active = option === locale;
              return (
                <button
                  key={option}
                  type="button"
                  aria-pressed={active}
                  onClick={() => setLocale(option)}
                  className={
                    active
                      ? "rounded-control bg-accent px-3 py-1.5 text-xs font-medium text-accent-foreground transition-colors duration-fast"
                      : "rounded-control border border-border bg-surface-muted px-3 py-1.5 text-xs font-medium text-foreground transition-colors duration-fast"
                  }
                >
                  {option}
                </button>
              );
            })}
          </div>
        </div>

        <div className="space-y-2">
          <div className="text-xs font-medium text-text-secondary">
            {t(locale, "shell.theme.label")}
          </div>
          <div className="flex flex-wrap gap-2">
            {THEMES.map((option) => {
              const active = option === theme;
              const labelKey = option === "light" ? "shell.theme.light" : "shell.theme.dark";
              return (
                <button
                  key={option}
                  type="button"
                  aria-pressed={active}
                  data-theme-option={option}
                  onClick={() => setTheme(option)}
                  className={
                    active
                      ? "rounded-control bg-accent px-3 py-1.5 text-xs font-medium text-accent-foreground transition-colors duration-fast"
                      : "rounded-control border border-border bg-surface-muted px-3 py-1.5 text-xs font-medium text-foreground transition-colors duration-fast"
                  }
                >
                  {t(locale, labelKey)}
                </button>
              );
            })}
          </div>
        </div>

        <div className="space-y-2">
          <div className="text-xs font-medium text-text-secondary">
            {t(locale, "shell.accent.label")}
          </div>
          <div className="flex flex-wrap gap-2">
            {ACCENTS.map((option) => {
              const active = option === accent;
              return (
                <button
                  key={option}
                  type="button"
                  aria-pressed={active}
                  data-accent-option={option}
                  onClick={() => setAccent(option)}
                  className={
                    active
                      ? "rounded-control bg-accent px-3 py-1.5 text-xs font-medium text-accent-foreground transition-colors duration-fast"
                      : "rounded-control border border-border bg-surface-muted px-3 py-1.5 text-xs font-medium text-foreground transition-colors duration-fast"
                  }
                >
                  {option}
                </button>
              );
            })}
          </div>
        </div>

        <footer className="border-t border-border-subtle pt-3 text-xs text-text-muted">
          {t(locale, "shell.status.offlineReady")}
        </footer>
      </section>
    </main>
  );
}

import { useEffect, useRef, useState } from "react";
import { CoursePreview } from "./features/course-preview/CoursePreview";
import { LOCALES, type Locale, type MessageKey, resolveInitialLocale, t } from "./i18n";
import { appBootstrap, courseInspectSource, isTauriRuntime, pickMarkdownFile } from "./lib/ipc/client";
import type { ParsedCourse, SessionEpoch } from "./lib/ipc/generated";

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

type BootstrapViewState =
  | { kind: "browser" }
  | { kind: "loading" }
  | { kind: "ready"; epoch: SessionEpoch }
  | { kind: "error" };

type PreviewState =
  | { kind: "idle" | "picking" | "loading" }
  | { kind: "ready"; course: ParsedCourse }
  | { kind: "error"; key: MessageKey };

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

function ipcStatusMessageKey(state: BootstrapViewState): MessageKey {
  switch (state.kind) {
    case "browser":
      return "shell.ipc.browser";
    case "loading":
      return "shell.ipc.connecting";
    case "ready":
      return "shell.ipc.ready";
    case "error":
      return "shell.ipc.error";
    default: {
      const _exhaustive: never = state;
      return _exhaustive;
    }
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
  const [bootstrap, setBootstrap] = useState<BootstrapViewState>(() =>
    isTauriRuntime() ? { kind: "loading" } : { kind: "browser" },
  );

  const [preview, setPreview] = useState<PreviewState>({ kind: "idle" });
  const openMarkdownButton = useRef<HTMLButtonElement>(null);

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

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let cancelled = false;
    void appBootstrap()
      .then((state) => {
        if (cancelled) {
          return;
        }
        setLocale(state.config.locale);
        setBootstrap({ kind: "ready", epoch: state.epoch });
      })
      .catch(() => {
        if (cancelled) {
          return;
        }
        setBootstrap({ kind: "error" });
      });

    return () => {
      cancelled = true;
    };
  }, []);

  async function openMarkdown() {
    setPreview({ kind: "picking" });
    try {
      const filePath = await pickMarkdownFile(t(locale, "preview.open"));
      if (filePath === null) {
        setPreview({ kind: "idle" });
        return;
      }
      setPreview({ kind: "loading" });
      const { course } = await courseInspectSource({ kind: "localMarkdown", filePath });
      setPreview({ kind: "ready", course });
    } catch (error) {
      const code = typeof error === "object" && error !== null && "code" in error ? error.code : null;
      setPreview({
        kind: "error",
        key: code === "MARKDOWN_LIMIT_EXCEEDED" ? "preview.tooLarge" : "preview.openFailed",
      });
    }
  }

  if (preview.kind === "ready") {
    return (
      <CoursePreview course={preview.course} locale={locale} onClose={() => {
        setPreview({ kind: "idle" });
        requestAnimationFrame(() => openMarkdownButton.current?.focus());
      }} />
    );
  }

  return (
    <main className="min-h-screen bg-background text-foreground flex items-center justify-center p-6">
      <section className="w-full max-w-xl rounded-panel border border-border bg-surface p-6 shadow-resting space-y-5">
        <header className="space-y-1">
          <h1 className="font-display text-2xl font-semibold tracking-tight text-foreground">
            {t(locale, "shell.title")}
          </h1>
          <p className="text-sm text-text-secondary">{t(locale, "shell.subtitle")}</p>
        </header>

        {bootstrap.kind === "ready" && (
          <div className="space-y-2">
            <button
              ref={openMarkdownButton}
              type="button"
              className="rounded-control bg-accent px-4 py-2 text-sm font-medium text-accent-foreground hover:opacity-90 disabled:opacity-60"
              disabled={preview.kind === "picking" || preview.kind === "loading"}
              onClick={() => void openMarkdown()}
            >
              {t(locale, "preview.open")}
            </button>
            <p role="status" className="text-sm text-text-secondary">
              {preview.kind === "loading" && t(locale, "preview.loading")}
            </p>
            {preview.kind === "error" && <p role="alert" className="text-sm">{t(locale, preview.key)}</p>}
          </div>
        )}

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
          <div>
            <div className="text-xs text-text-muted">{t(locale, "shell.epoch.label")}</div>
            <div className="font-mono text-foreground" data-testid="shell-epoch">
              {bootstrap.kind === "ready" ? String(bootstrap.epoch) : "-"}
            </div>
          </div>
          <div>
            <div className="text-xs text-text-muted">{t(locale, "shell.ipc.label")}</div>
            <div className="font-mono text-foreground" data-testid="shell-ipc-status">
              {t(locale, ipcStatusMessageKey(bootstrap))}
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

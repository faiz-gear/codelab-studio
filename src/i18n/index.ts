import { enUS } from "./en-US";
import { zhCN } from "./zh-CN";

export const LOCALES = ["zh-CN", "en-US"] as const;
export type Locale = (typeof LOCALES)[number];

export type MessageKey =
  | "app.name"
  | "shell.title"
  | "shell.subtitle"
  | "shell.locale.label"
  | "shell.theme.label"
  | "shell.theme.light"
  | "shell.theme.dark"
  | "shell.accent.label"
  | "shell.viewport.label"
  | "shell.status.offlineReady"
  | "error.unknown"
  | "error.invalidInput"
  | "error.notReady";

export type MessageParams = Readonly<Record<string, string | number>>;

const MESSAGES: Record<Locale, Record<MessageKey, string>> = {
  "zh-CN": zhCN,
  "en-US": enUS,
};

export function isLocale(value: unknown): value is Locale {
  return value === "zh-CN" || value === "en-US";
}

export function resolveInitialLocale(navigatorLang?: string | null): Locale {
  if (typeof navigatorLang !== "string") {
    return "en-US";
  }
  const normalized = navigatorLang.trim().toLowerCase();
  return normalized.startsWith("zh") ? "zh-CN" : "en-US";
}

export function t(locale: Locale, key: MessageKey, params?: MessageParams): string {
  const template = MESSAGES[locale][key];
  if (!params) {
    return template;
  }
  return template.replace(/\{([a-zA-Z0-9_]+)\}/g, (match, name: string) => {
    const value = params[name];
    return value === undefined ? match : String(value);
  });
}

import type { DomainError, Locale, StoreKind } from "../core/contracts";
import { enUS } from "./en-US";
import { zhCN, type Messages } from "./zh-CN";

const DICTIONARIES: Record<Locale, Messages> = {
  "zh-CN": zhCN,
  "en-US": enUS,
};

export function getMessages(locale: Locale): Messages {
  return DICTIONARIES[locale] ?? zhCN;
}

export function interpolate(
  template: string,
  params?: Record<string, string | number>
): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (full, key: string) =>
    key in params ? String(params[key]) : full
  );
}

export function formatDomainError(
  error: DomainError,
  locale: Locale
): string {
  const messages = getMessages(locale);
  const template = messages.errors[error.code];
  const rawDetails = error.details ?? {};
  const localizedDetails: Record<string, string> = { ...rawDetails };
  if (rawDetails.store && rawDetails.store in messages.stores) {
    localizedDetails.store = messages.stores[rawDetails.store as StoreKind];
  }
  return interpolate(template, localizedDetails);
}

export function formatRelativeTime(
  isoTimestamp: string,
  locale: Locale,
  nowMs: number = Date.now()
): string {
  const parsed = Date.parse(isoTimestamp);
  const t = getMessages(locale).relativeTime;
  if (Number.isNaN(parsed)) {
    return t.justNow;
  }
  const diffMinutes = Math.max(0, Math.floor((nowMs - parsed) / 60_000));
  if (diffMinutes < 1) return t.justNow;
  if (diffMinutes < 60) return interpolate(t.minutesAgo, { n: diffMinutes });
  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return interpolate(t.hoursAgo, { n: diffHours });
  const diffDays = Math.floor(diffHours / 24);
  return interpolate(t.daysAgo, { n: diffDays });
}

export type { Messages };

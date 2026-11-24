export type ThemeMode = "light" | "dark";

export type AccentPreset =
  | "neutral"
  | "violet"
  | "blue"
  | "green"
  | "amber"
  | "orange"
  | "coral"
  | "rose";

export const ACCENT_PRESETS: readonly AccentPreset[] = [
  "neutral",
  "violet",
  "blue",
  "green",
  "amber",
  "orange",
  "coral",
  "rose",
] as const;

export type Locale = "zh-CN" | "en-US";

export type CourseLanguage = "Go" | "JS/TS" | "Python" | "Java";

export const COURSE_LANGUAGES: readonly CourseLanguage[] = [
  "Go",
  "JS/TS",
  "Python",
  "Java",
] as const;

export const CODE_FONT_SIZE_PRESETS = [12.5, 13.5, 15] as const;

export type ErrorCode =
  | "INVALID_CODE_FONT_SIZE"
  | "INVALID_WORKSPACE_PATH"
  | "INVALID_COURSE_METADATA"
  | "INVALID_PROGRESS_COUNT"
  | "INVALID_TRUST_SOURCE"
  | "STORAGE_READ_FAILED"
  | "STORAGE_WRITE_FAILED"
  | "STORAGE_CORRUPTED"
  | "SHUTDOWN_FAILED";

export interface DomainError {
  code: ErrorCode;
  details?: Record<string, string>;
}

export interface GlobalSettings {
  theme: ThemeMode;
  accent: AccentPreset;
  codeFontSize: number;
  locale: Locale;
}

export const DEFAULT_SETTINGS: GlobalSettings = {
  theme: "light",
  accent: "neutral",
  codeFontSize: 13.5,
  locale: "zh-CN",
};

export interface UpdateSettingsPatch {
  theme?: ThemeMode;
  accent?: AccentPreset;
  codeFontSize?: number;
  locale?: Locale;
}

export interface RecentWorkspace {
  workspacePath: string;
  courseId: string;
  courseTitle: string;
  language: CourseLanguage;
  completedExercises: number;
  totalExercises: number;
  lastOpenedAt: string;
}

export interface RecordRecentWorkspaceInput {
  workspacePath: string;
  courseId: string;
  courseTitle: string;
  language: CourseLanguage;
  completedExercises: number;
  totalExercises: number;
}

export type TrustSourceKind = "localPath" | "gitRepo";

export interface TrustEntry {
  sourceKind: TrustSourceKind;
  normalizedSource: string;
  trustedAt: string;
}

export interface TrustStatus {
  sourceKind: TrustSourceKind;
  normalizedSource: string;
  trusted: boolean;
  trustedAt: string | null;
}

export type StoreKind = "settings" | "recents" | "trust";

export interface StorageRecoveryNotice {
  store: StoreKind;
  errorCode: ErrorCode;
  backupPath: string | null;
}

export interface ShutdownReport {
  alreadyShutDown: boolean;
  hooksExecuted: number;
}

export interface AppBootstrap {
  settings: GlobalSettings;
  recents: RecentWorkspace[];
  trustEntries: TrustEntry[];
  recoveryNotices: StorageRecoveryNotice[];
}

export type AppCommand =
  | { type: "bootstrap" }
  | { type: "getSettings" }
  | { type: "updateSettings"; payload: UpdateSettingsPatch }
  | { type: "listRecents" }
  | { type: "recordRecentWorkspace"; payload: RecordRecentWorkspaceInput }
  | { type: "removeRecentWorkspace"; payload: { workspacePath: string } }
  | { type: "listTrustEntries" }
  | {
      type: "checkTrust";
      payload: { sourceKind: TrustSourceKind; source: string };
    }
  | {
      type: "grantTrust";
      payload: { sourceKind: TrustSourceKind; source: string };
    }
  | {
      type: "revokeTrust";
      payload: { sourceKind: TrustSourceKind; source: string };
    }
  | { type: "shutdown" };

export type AppResponse =
  | { type: "bootstrap"; payload: AppBootstrap }
  | { type: "settings"; payload: GlobalSettings }
  | { type: "recents"; payload: RecentWorkspace[] }
  | { type: "trustEntries"; payload: TrustEntry[] }
  | { type: "trustStatus"; payload: TrustStatus }
  | { type: "shutdown"; payload: ShutdownReport };

export type AppEvent =
  | { type: "settingsUpdated"; payload: GlobalSettings }
  | { type: "recentsUpdated"; payload: RecentWorkspace[] }
  | { type: "trustUpdated"; payload: TrustEntry[] }
  | { type: "shutdownCompleted"; payload: ShutdownReport };

export class CoreCommandError extends Error {
  readonly domainError: DomainError;

  constructor(domainError: DomainError) {
    super(domainError.code);
    this.name = "CoreCommandError";
    this.domainError = domainError;
  }
}

export function isDomainError(value: unknown): value is DomainError {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as { code?: unknown; details?: unknown };
  return typeof candidate.code === "string";
}

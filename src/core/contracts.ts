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

export const ERROR_CODES = [
  "INVALID_CODE_FONT_SIZE",
  "INVALID_WORKSPACE_PATH",
  "INVALID_COURSE_METADATA",
  "INVALID_PROGRESS_COUNT",
  "INVALID_TRUST_SOURCE",
  "COURSE_NOT_FOUND",
  "COURSE_PATH_INVALID",
  "COURSE_MARKDOWN_TOO_LARGE",
  "COURSE_MARKDOWN_INVALID",
  "COURSE_MANIFEST_INVALID",
  "COURSE_CHECKS_INVALID",
  "NO_ACTIVE_COURSE",
  "STORAGE_READ_FAILED",
  "STORAGE_WRITE_FAILED",
  "STORAGE_CORRUPTED",
  "SHUTDOWN_FAILED",
  "IPC_UNAVAILABLE",
  "UNEXPECTED_ERROR",
] as const;

export type ErrorCode = (typeof ERROR_CODES)[number];

const KNOWN_ERROR_CODES = new Set<string>(ERROR_CODES);

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

export interface TrustSource {
  sourceKind: TrustSourceKind;
  source: string;
}

export interface TrustEntry {
  sourceKind: TrustSourceKind;
  normalizedSource: string;
  trustedAt: string;
}

export interface TrustState {
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

export type CourseSource =
  | { kind: "builtin"; courseId: string }
  | { kind: "localMarkdown"; path: string }
  | { kind: "localPack"; path: string };

export type OpenCourseRequest =
  | { kind: "builtin"; courseId: string }
  | { kind: "localMarkdown"; path: string }
  | { kind: "localPack"; path: string };

export function courseSourceKey(source: CourseSource): string {
  return source.kind === "builtin"
    ? `builtin:${source.courseId}`
    : `${source.kind}:${source.path}`;
}

export function courseSourceLocation(source: CourseSource): string {
  return source.kind === "builtin"
    ? `builtin://${source.courseId}`
    : source.path;
}

export function openRequestFromSource(source: CourseSource): OpenCourseRequest {
  return source;
}

export type ChapterKind = "guide" | "exercise";

export type ChapterPresentation = "threeStep" | "linearFallback";

export type CommandMode = "oneshot" | "daemon";

export type ValueOrigin =
  | "explicit"
  | "inferred"
  | "chapterDefault"
  | "workspaceRoot";

export interface CommandBlock {
  id: string;
  shell: string;
  command: string;
  cwd: string | null;
  cwdOrigin: ValueOrigin;
  mode: CommandMode;
  modeOrigin: ValueOrigin;
  label: string | null;
  port: number | null;
  isSecondary: boolean;
}

export type CheckKind =
  | "httpAssert"
  | "commandTest"
  | "testSuite"
  | "selfCheck";

export interface DaemonRequirement {
  label: string;
  port: number;
  serviceKey: string | null;
}

export interface CourseCheckSpec {
  id: string;
  title: string;
  kind: CheckKind;
  required: boolean;
  cwd: string | null;
  command: string | null;
  method: string | null;
  url: string | null;
}

export type ContentBlock =
  | { kind: "prose"; html: string }
  | { kind: "command"; commandId: string };

export interface ThreeStepProjection {
  step1IntroHtml: string | null;
  step1CommandIds: string[];
  step2EntryFile: string | null;
  step2Html: string | null;
  step3IntroHtml: string | null;
  step3Checks: CourseCheckSpec[];
  requiresDaemon: DaemonRequirement | null;
  secondaryCommandIds: string[];
  reflectionsHtml: string | null;
}

export interface Chapter {
  chapterId: string;
  title: string;
  kind: ChapterKind;
  presentation: ChapterPresentation;
  defaultCwd: string | null;
  entryFile: string | null;
  summaryHtml: string | null;
  blocks: ContentBlock[];
  commands: CommandBlock[];
  threeStep: ThreeStepProjection | null;
  reflectionsHtml: string | null;
}

export interface CourseSummary {
  courseId: string;
  title: string;
  description: string;
  language: CourseLanguage | null;
  source: CourseSource;
  totalChapters: number;
  totalExercises: number;
  isBuiltin: boolean;
}

export interface LoadedCourse {
  courseId: string;
  title: string;
  description: string;
  language: CourseLanguage | null;
  version: string | null;
  author: string | null;
  dependencies: string[];
  source: CourseSource;
  entryMarkdownPath: string | null;
  isBuiltin: boolean;
  introHtml: string | null;
  totalExercises: number;
  chapters: Chapter[];
}

export interface CourseSourceChangedEvent {
  courseId: string;
  changedPaths: string[];
  detectedAt: string;
}

export interface AppBootstrap {
  settings: GlobalSettings;
  recents: RecentWorkspace[];
  trustEntries: TrustEntry[];
  recoveryNotices: StorageRecoveryNotice[];
  builtinCourses: CourseSummary[];
}

export type AppCommand =
  | { type: "bootstrap" }
  | { type: "getSettings" }
  | { type: "updateSettings"; payload: UpdateSettingsPatch }
  | { type: "listRecents" }
  | { type: "recordRecentWorkspace"; payload: RecordRecentWorkspaceInput }
  | { type: "removeRecentWorkspace"; payload: { workspacePath: string } }
  | { type: "listTrustEntries" }
  | { type: "checkTrust"; payload: TrustSource }
  | { type: "grantTrust"; payload: TrustSource }
  | { type: "revokeTrust"; payload: TrustSource }
  | { type: "listBuiltinCourses" }
  | { type: "openCourse"; payload: OpenCourseRequest }
  | { type: "reloadActiveCourse" }
  | { type: "closeActiveCourse" }
  | { type: "shutdown" };

export type AppResponse =
  | { type: "bootstrap"; payload: AppBootstrap }
  | { type: "settings"; payload: GlobalSettings }
  | { type: "recents"; payload: RecentWorkspace[] }
  | { type: "trustEntries"; payload: TrustEntry[] }
  | { type: "trustState"; payload: TrustState }
  | { type: "builtinCourses"; payload: CourseSummary[] }
  | { type: "loadedCourse"; payload: LoadedCourse }
  | { type: "courseClosed" }
  | { type: "shutdown"; payload: ShutdownReport };

export type AppEvent =
  | { type: "settingsUpdated"; payload: GlobalSettings }
  | { type: "recentsUpdated"; payload: RecentWorkspace[] }
  | { type: "trustUpdated"; payload: TrustEntry[] }
  | { type: "courseLoaded"; payload: LoadedCourse }
  | { type: "courseSourceChanged"; payload: CourseSourceChangedEvent }
  | { type: "shutdownCompleted"; payload: ShutdownReport };

export type LocalPathPickKind = "markdownFile" | "coursePackDir";

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
  if (
    typeof candidate.code !== "string" ||
    !KNOWN_ERROR_CODES.has(candidate.code)
  ) {
    return false;
  }
  if (
    candidate.details !== undefined &&
    (typeof candidate.details !== "object" || candidate.details === null)
  ) {
    return false;
  }
  return true;
}

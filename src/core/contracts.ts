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
  "TEST_SUITE_ADAPTER_UNAVAILABLE",
  "GIT_COURSE_FETCH_FAILED",
  "GIT_COURSE_CACHE_FAILED",
  "NO_ACTIVE_GIT_COURSE",
  "NO_ACTIVE_DRAFT",
  "INVALID_DRAFT_PATH",
  "DRAFT_DIRECTORY_NOT_EMPTY",
  "DRAFT_WRITE_FAILED",
  "TRIAL_WORKSPACE_FAILED",
  "NO_ACTIVE_COURSE",
  "COURSE_UNTRUSTED",
  "NO_ACTIVE_WORKSPACE",
  "WORKSPACE_PATH_OUTSIDE_ROOT",
  "WORKSPACE_BOUND_TO_ANOTHER_COURSE",
  "WORKSPACE_FILE_NOT_FOUND",
  "WORKSPACE_FILE_EXISTS",
  "WORKSPACE_FILE_READ_FAILED",
  "WORKSPACE_FILE_WRITE_FAILED",
  "WORKSPACE_FILE_NOT_TEXT",
  "WORKSPACE_WATCH_FAILED",
  "WORKSPACE_STARTER_FAILED",
  "EXTERNAL_IDE_OPEN_FAILED",
  "STORAGE_READ_FAILED",
  "STORAGE_WRITE_FAILED",
  "STORAGE_CORRUPTED",
  "RUNTIME_TASK_START_FAILED",
  "RUNTIME_PLATFORM_UNSUPPORTED",
  "RUNTIME_DOWNLOAD_FAILED",
  "RUNTIME_INTEGRITY_FAILED",
  "RUNTIME_INSTALL_FAILED",
  "EXECUTION_FAILED",
  "VERIFICATION_BUSY",
  "CHECK_NOT_FOUND",
  "EXERCISE_COMPLETION_BLOCKED",
  "DAEMON_PORT_CONFLICT",
  "DAEMON_NOT_READY",
  "COMMAND_BLOCK_NOT_FOUND",
  "COMMAND_CWD_INVALID",
  "COMMAND_SHELL_UNAVAILABLE",
  "TERMINAL_NOT_FOUND",
  "ACP_COMMAND_INVALID",
  "ACP_AGENT_START_FAILED",
  "ACP_HANDSHAKE_FAILED",
  "ACP_PROTOCOL_ERROR",
  "ACP_PROCESS_EXITED",
  "TUTOR_PROMPT_EMPTY",
  "TUTOR_BUSY",
  "TUTOR_PERMISSION_NOT_FOUND",
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

export type AcpAgentPreset =
  | "claudeCode"
  | "geminiCli"
  | "codexCli"
  | "openCode"
  | "custom";

export const ACP_AGENT_PRESETS: readonly {
  id: AcpAgentPreset;
  label: string;
  defaultCommand: string;
}[] = [
  { id: "claudeCode", label: "Claude Code", defaultCommand: "claude --acp" },
  { id: "geminiCli", label: "Gemini CLI", defaultCommand: "gemini --acp" },
  { id: "codexCli", label: "Codex CLI", defaultCommand: "codex --acp" },
  { id: "openCode", label: "OpenCode", defaultCommand: "opencode acp" },
  { id: "custom", label: "Custom", defaultCommand: "" },
] as const;

export type PermissionPolicy = "guided_readonly" | "strict_ask_all";
export type TeachingStyle = "socratic" | "direct";

export interface TutorSettings {
  preset: AcpAgentPreset;
  command: string;
  permissionPolicy: PermissionPolicy;
  teachingStyle: TeachingStyle;
}

export const DEFAULT_TUTOR_SETTINGS: TutorSettings = {
  preset: "claudeCode",
  command: "claude --acp",
  permissionPolicy: "guided_readonly",
  teachingStyle: "socratic",
};

export interface TutorSettingsPatch {
  preset?: AcpAgentPreset;
  command?: string;
  permissionPolicy?: PermissionPolicy;
  teachingStyle?: TeachingStyle;
}

export interface GlobalSettings {
  theme: ThemeMode;
  accent: AccentPreset;
  codeFontSize: number;
  locale: Locale;
  tutor: TutorSettings;
}

export const DEFAULT_SETTINGS: GlobalSettings = {
  theme: "light",
  accent: "neutral",
  codeFontSize: 13.5,
  locale: "zh-CN",
  tutor: DEFAULT_TUTOR_SETTINGS,
};

export interface UpdateSettingsPatch {
  theme?: ThemeMode;
  accent?: AccentPreset;
  codeFontSize?: number;
  locale?: Locale;
  tutor?: TutorSettingsPatch;
}

export interface RecentWorkspace {
  workspacePath: string;
  courseId: string;
  courseTitle: string;
  language: CourseLanguage | null;
  courseSource?: CourseSource | null;
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

export type ProbeStatus = "ok" | "warn" | "missing";
export type ProbeProblem = "versionTooOld" | "timedOut" | "failed" | "proxyDisabled" | "invalidProxy";
export type HostTool =
  | "bun" | "node" | "go" | "javac" | "python3" | "python" | "uv" | "uvPython"
  | "git" | "curl" | "typeScriptLanguageServer" | "gopls" | "jdtls" | "pyright" | "goProxy";

export type OptionalLsp = "typeScriptLanguageServer" | "gopls" | "jdtls" | "pyright";

export interface ToolCheck {
  tool: HostTool;
  status: ProbeStatus;
  value: string | null;
  executable: string | null;
  problem: ProbeProblem | null;
}

export interface ManagedRuntime {
  version: string;
  executable: string;
  testExecutor: string;
  uv: string | null;
}

export type InstallProgress =
  | { phase: "preparing" }
  | { phase: "downloading"; artifact: string; receivedBytes: number; totalBytes: number | null }
  | { phase: "extracting"; artifact: string }
  | { phase: "installed" }
  | { phase: "failed"; error: DomainError };

export interface RuntimePreference {
  language: CourseLanguage;
  preferManaged: boolean;
}

export interface LanguagePreflight extends RuntimePreference {
  status: ProbeStatus;
  system: ToolCheck[];
  lsp: (ToolCheck & { tool: OptionalLsp }) | null;
  managed: ManagedRuntime | null;
  selected: { source: "system"; tool: HostTool } | { source: "managed" } | null;
  canInstall: boolean;
  install: InstallProgress | null;
}

export interface PreflightReport {
  revision: number;
  checked: boolean;
  probing: boolean;
  installingCount: number;
  languages: LanguagePreflight[];
  tools: ToolCheck[];
  goProxy: ToolCheck | null;
  installDir: string;
}

export type StoreKind =
  | "settings"
  | "runtimePreferences"
  | "recents"
  | "trust"
  | "workspace"
  | "progress"
  | "metrics"
  | "tutorHistory";

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
  | { kind: "localPack"; path: string }
  | { kind: "gitPack"; url: string };

export type OpenCourseRequest = CourseSource;

export function courseSourceKey(source: CourseSource): string {
  return source.kind === "builtin" ? `builtin:${source.courseId}` : `${source.kind}:${courseSourceLocation(source)}`;
}

export function courseSourceLocation(source: CourseSource): string {
  if (source.kind === "builtin") return `builtin://${source.courseId}`;
  return source.kind === "gitPack" ? source.url : source.path;
}

export function openRequestFromSource(source: CourseSource): OpenCourseRequest {
  return source;
}

export interface CourseDraftRequest {
  path: string;
  title: string;
  language: CourseLanguage;
  referenceSolution: boolean;
}

export interface DraftDiagnostic {
  path: string;
  error: DomainError;
}

export interface DraftPreview {
  course: LoadedCourse | null;
  diagnostics: DraftDiagnostic[];
}

export interface CourseDraft {
  id: number;
  root: string;
  files: string[];
  preview: DraftPreview;
  requiresRegistryTrust: boolean;
}

export interface DraftFilePath {
  draftId: number;
  path: string;
}

export interface DraftEdit {
  path: string;
  text: string;
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
  headers?: Record<string, string>;
  body?: string | null;
  expect?: { status?: number | null; headers: Record<string, string>; bodyContains?: string | null; bodyEquals?: string | null };
  timeoutSec?: number | null;
  expectExitCode?: number | null;
  forbidOutput?: string[];
  requireTests?: string[];
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
  hasReferenceHint?: boolean;
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
  trusted: boolean;
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

export interface WorkspaceSummary {
  id: number;
  root: string;
  recoveryNotices: StorageRecoveryNotice[];
}

export interface WorkspacePath {
  workspaceId: number;
  path: string;
}

export interface WorkspaceEntry {
  path: string;
  name: string;
  kind: "directory" | "file";
}

export type WorkspaceFile = {
  path: string;
  sizeBytes: number;
  revision: string | null;
} & ({ kind: "text"; text: string } | { kind: "binary" | "tooLarge" | "missing" });

export interface WorkspaceComparison {
  current: WorkspaceFile;
  proposed: string;
}

export interface WorkspaceFilesChanged {
  workspaceId: number;
  paths: string[];
}

export type ExternalIde = "vsCode" | "cursor" | "goLand" | "intelliJIdea";


export interface WorkspaceTerminalRequest {
  workspaceId: number;
}

export interface TerminalTarget extends WorkspaceTerminalRequest {
  terminalId: number;
}

export interface CommandDraft {
  path: string;
  text: string;
  expectedRevision: string | null;
}

export interface RunCommandBlock extends WorkspaceTerminalRequest {
  commandId: string;
  confirmation?: number;
  restartTerminalId?: number;
  drafts: CommandDraft[];
}

export interface CommandConfirmation {
  token: number;
  cwd: string;
  command: string;
}

export interface TerminalSummary {
  id: number;
  commandId: string | null;
  label: string | null;
  mode: CommandMode | null;
  cwd: string;
  startedAt: string;
  port: number | null;
  running: boolean;
  exitCode: number | null;
  portConflict: boolean;
}

export interface TerminalOutput {
  cursor: number;
  reset: boolean;
  data: string;
  more: boolean;
}

export interface TerminalUpdates {
  terminals: TerminalSummary[];
  output: TerminalOutput | null;
  recentErrors: string[];
}

export type CommandCwdOverrides = Record<string, string>;

export interface ExerciseTarget extends WorkspaceTerminalRequest { chapterId: string }
export interface RunChecks extends ExerciseTarget { drafts: CommandDraft[]; confirmation?: number }
export type CheckStatus = "running" | "passed" | "failed" | "timedOut" | "skipped";
export interface CheckResult {
  checkId: string; status: CheckStatus; durationMs: number;
  diffs: { label: string; expected: string; actual: string }[];
  rawOutput: string;
  testCases: { name: string; status: CheckStatus; durationMs: number; output: string }[];
  error: DomainError | null;
}
export interface ChapterProgress {
  completionState: "not_started" | "in_progress" | "completed";
  lastCheckResults: CheckResult[];
  selfChecks: Record<string, boolean>;
}
export interface LearningProgress {
  workspaceId: number; completedExercises: number; totalExercises: number;
  runningChapterId: string | null; chapters: Record<string, ChapterProgress>;
}

export type TutorRole = "user" | "assistant";
export type TutorMessageStatus = "streaming" | "completed" | "cancelled" | "failed";
export type TutorPermissionKind = "read" | "write" | "execute";
export type TutorPermissionStatus =
  | "pending"
  | "autoAllowed"
  | "approved"
  | "denied"
  | "blockedSensitive"
  | "cancelled";

export interface TutorPermissionRequest {
  id: string;
  turnId: string;
  kind: TutorPermissionKind;
  title: string;
  targetPath: string | null;
  command: string | null;
  outsideWorkspace: boolean;
  sensitive: boolean;
  highRisk: boolean;
  status: TutorPermissionStatus;
  createdAt: string;
  resolvedAt: string | null;
}

export interface TutorFileChange {
  path: string;
  beforeText: string | null;
  afterText: string;
  createdAt: string;
}

export interface TutorMessage {
  id: string;
  turnId: string;
  role: TutorRole;
  content: string;
  teachingStyle: TeachingStyle;
  status: TutorMessageStatus;
  error: DomainError | null;
  permissions: TutorPermissionRequest[];
  fileChanges: TutorFileChange[];
  createdAt: string;
}

export type TutorSessionStatus =
  | "idle"
  | "starting"
  | "ready"
  | "streaming"
  | "awaitingPermission"
  | "error";

export interface TutorSessionSnapshot {
  workspaceId: number;
  status: TutorSessionStatus;
  processRunning: boolean;
  sessionReused: boolean;
  preset: AcpAgentPreset;
  command: string;
  agentName: string | null;
  permissionPolicy: PermissionPolicy;
  teachingStyle: TeachingStyle;
  activeTurnId: string | null;
  lastError: DomainError | null;
  canRetry: boolean;
  messages: TutorMessage[];
}

export interface TutorHandshakeRequest {
  preset: AcpAgentPreset;
  command: string;
}

export interface TutorHandshakeResult {
  preset: AcpAgentPreset;
  command: string;
  protocolVersion: string | null;
  agentName: string | null;
  agentVersion: string | null;
}

export interface SendTutorPrompt {
  workspaceId: number;
  prompt: string;
  teachingStyle?: TeachingStyle;
  contextPreamble?: string;
}

export interface RespondTutorPermission {
  workspaceId: number;
  requestId: string;
  allow: boolean;
}

export interface WorkspaceTutorRequest {
  workspaceId: number;
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
  | { type: "refreshGitCourse" }
  | { type: "createCourseDraft"; payload: CourseDraftRequest }
  | { type: "openCourseDraft"; payload: { path: string } }
  | { type: "readCourseDraftFile"; payload: DraftFilePath }
  | { type: "writeCourseDraftFile"; payload: DraftFilePath & { text: string; expectedRevision: string | null } }
  | { type: "previewCourseDraft"; payload: { draftId: number; edits: DraftEdit[] } }
  | { type: "startTrialWorkspace"; payload: { draftId: number } }
  | { type: "closeActiveCourse" }
  | { type: "bindWorkspace"; payload: { path: string } }
  | { type: "openRecentWorkspace"; payload: { path: string } }
  | { type: "listWorkspaceFiles"; payload: WorkspacePath }
  | { type: "readWorkspaceFile"; payload: WorkspacePath }
  | { type: "writeWorkspaceFile"; payload: WorkspacePath & { text: string; expectedRevision: string | null } }
  | { type: "previewChapterSolution"; payload: { workspaceId: number; chapterId: string } }
  | { type: "listStarterFiles"; payload: { workspaceId: number } }
  | { type: "previewStarterFile"; payload: WorkspacePath }
  | { type: "applyWorkspaceComparison"; payload: WorkspacePath & { text: string; expectedRevision: string | null } }
  | { type: "createWorkspaceFile"; payload: WorkspacePath }
  | { type: "renameWorkspaceFile"; payload: WorkspacePath & { newPath: string } }
  | { type: "openWorkspaceIde"; payload: WorkspacePath & { ide: ExternalIde } }
  | { type: "openTerminal"; payload: WorkspaceTerminalRequest }
  | { type: "readTerminals"; payload: WorkspaceTerminalRequest & { terminalId?: number; cursor?: number; includeRecentErrors?: boolean } }
  | { type: "writeTerminal"; payload: TerminalTarget & { text: string } }
  | { type: "resizeTerminal"; payload: TerminalTarget & { cols: number; rows: number } }
  | { type: "stopTerminal"; payload: TerminalTarget }
  | { type: "closeTerminal"; payload: TerminalTarget }
  | { type: "runCommandBlock"; payload: RunCommandBlock }
  | { type: "getCommandCwdOverrides"; payload: WorkspaceTerminalRequest }
  | { type: "setCommandCwdOverride"; payload: WorkspaceTerminalRequest & { commandId: string; cwd: string | null } }
  | { type: "runChecks"; payload: RunChecks }
  | { type: "getProgress"; payload: WorkspaceTerminalRequest }
  | { type: "resetProgress"; payload: WorkspaceTerminalRequest }
  | { type: "exportLearningRecord"; payload: WorkspaceTerminalRequest }
  | { type: "setSelfCheck"; payload: ExerciseTarget & { checkId: string; checked: boolean } }
  | { type: "completeExercise"; payload: ExerciseTarget }
  | { type: "getPreflight" }
  | { type: "refreshPreflight" }
  | { type: "setRuntimePreference"; payload: RuntimePreference }
  | { type: "installManagedRuntime"; payload: { language: CourseLanguage } }
  | { type: "testTutorHandshake"; payload: TutorHandshakeRequest }
  | { type: "getTutorSession"; payload: WorkspaceTutorRequest }
  | { type: "sendTutorPrompt"; payload: SendTutorPrompt }
  | { type: "stopTutorTurn"; payload: WorkspaceTutorRequest }
  | { type: "retryTutorTurn"; payload: WorkspaceTutorRequest }
  | { type: "respondTutorPermission"; payload: RespondTutorPermission }
  | { type: "clearTutorSession"; payload: WorkspaceTutorRequest }
  | { type: "shutdown" };

export type PreflightCommand = Extract<
  AppCommand,
  { type: "getPreflight" | "refreshPreflight" | "setRuntimePreference" | "installManagedRuntime" }
>;

export type AppResponse =
  | { type: "bootstrap"; payload: AppBootstrap }
  | { type: "settings"; payload: GlobalSettings }
  | { type: "recents"; payload: RecentWorkspace[] }
  | { type: "trustEntries"; payload: TrustEntry[] }
  | { type: "trustState"; payload: TrustState }
  | { type: "builtinCourses"; payload: CourseSummary[] }
  | { type: "loadedCourse"; payload: LoadedCourse }
  | { type: "courseDraft"; payload: CourseDraft }
  | { type: "draftPreview"; payload: DraftPreview }
  | { type: "trialTrustRequired"; payload: TrustSource }
  | { type: "trialWorkspaceOpened"; payload: { course: LoadedCourse; workspace: WorkspaceSummary } }
  | { type: "courseClosed" }
  | { type: "workspaceBound"; payload: WorkspaceSummary }
  | { type: "recentWorkspaceOpened"; payload: { course: LoadedCourse; workspace: WorkspaceSummary | null } }
  | { type: "workspaceEntries"; payload: WorkspaceEntry[] }
  | { type: "workspaceFile"; payload: WorkspaceFile }
  | { type: "workspaceFileConflict"; payload: WorkspaceFile }
  | { type: "workspaceComparisons"; payload: WorkspaceComparison[] }
  | { type: "starterFiles"; payload: string[] }
  | { type: "workspaceIdeOpened" }
  | { type: "terminalOpened"; payload: TerminalSummary }
  | { type: "terminals"; payload: TerminalUpdates }
  | { type: "terminalUpdated" }
  | { type: "commandCwdOverrides"; payload: CommandCwdOverrides }
  | { type: "commandConfirmationRequired"; payload: CommandConfirmation }
  | { type: "commandSaveConflict"; payload: { file: WorkspaceFile; saved: WorkspaceFile[] } }
  | { type: "commandFailed"; payload: { error: DomainError; saved: WorkspaceFile[] } }
  | { type: "commandStarted"; payload: { terminal: TerminalSummary; saved: WorkspaceFile[] } }
  | { type: "verificationStarted"; payload: { progress: LearningProgress; saved: WorkspaceFile[] } }
  | { type: "progress"; payload: LearningProgress }
  | { type: "learningRecord"; payload: Record<string, unknown> }
  | { type: "preflight"; payload: PreflightReport }
  | { type: "tutorHandshake"; payload: TutorHandshakeResult }
  | { type: "tutorSession"; payload: TutorSessionSnapshot }
  | { type: "shutdown"; payload: ShutdownReport };

export type AppEvent =
  | { type: "progressUpdated"; payload: LearningProgress }
  | { type: "settingsUpdated"; payload: GlobalSettings }
  | { type: "recentsUpdated"; payload: RecentWorkspace[] }
  | { type: "trustUpdated"; payload: TrustEntry[] }
  | { type: "courseLoaded"; payload: LoadedCourse }
  | { type: "courseSourceChanged"; payload: CourseSourceChangedEvent }
  | { type: "courseTrustChanged"; payload: { source: CourseSource; trusted: boolean } }
  | { type: "workspaceFilesChanged"; payload: WorkspaceFilesChanged }
  | { type: "preflightUpdated"; payload: PreflightReport }
  | { type: "tutorSessionUpdated"; payload: TutorSessionSnapshot }
  | { type: "shutdownCompleted"; payload: ShutdownReport };

export type LocalPathPickKind = "markdownFile" | "coursePackDir" | "workspaceDir";

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

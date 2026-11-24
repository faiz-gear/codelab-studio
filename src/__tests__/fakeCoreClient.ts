import type { CoreClient, CoreEventListener, Unsubscribe } from "../core/client";
import {
  CoreCommandError,
  DEFAULT_SETTINGS,
  type AppCommand,
  type AppEvent,
  type AppResponse,
  type CourseSummary,
  type DomainError,
  type GlobalSettings,
  type LoadedCourse,
  type LocalPathPickKind,
  type OpenCourseRequest,
  type PreflightReport,
  type RecentWorkspace,
  type StorageRecoveryNotice,
  type TrustEntry,
} from "../core/contracts";

export function createBuiltinGoCourseFixture(): LoadedCourse {
  return {
    courseId: "go-goframe-practice",
    title: "Go 与 GoFrame 开发实践手册：从第一性原理出发",
    description:
      "围绕同一个任务管理 API，先用 Go 标准库实现，再迁移到 GoFrame，覆盖练习 0 至 9。",
    language: "Go",
    version: "1.0.0",
    author: "CodeLab Studio",
    dependencies: ["go>=1.22"],
    source: { kind: "builtin", courseId: "go-goframe-practice" },
    entryMarkdownPath: null,
    isBuiltin: true,
    introHtml: "<p>先用 Go 标准库实现，再迁移到 GoFrame。</p>",
    totalExercises: 10,
    chapters: [
      {
        chapterId: "工作目录与使用方式",
        title: "工作目录与使用方式",
        kind: "guide",
        presentation: "linearFallback",
        defaultCwd: null,
        entryFile: null,
        summaryHtml: "<p>选择一个本地空目录作为学习 Workspace。</p>",
        blocks: [
          {
            kind: "prose",
            html: "<p>选择一个本地空目录作为学习 Workspace。</p>",
          },
          { kind: "command", commandId: "工作目录与使用方式:cmd-1" },
        ],
        commands: [
          {
            id: "工作目录与使用方式:cmd-1",
            shell: "bash",
            command: "pwd",
            cwd: null,
            cwdOrigin: "workspaceRoot",
            mode: "oneshot",
            modeOrigin: "inferred",
            label: null,
            port: null,
            isSecondary: false,
          },
        ],
        threeStep: null,
        reflectionsHtml: null,
      },
      {
        chapterId: "ex-0",
        title: "练习 0：写清楚系统的规则",
        kind: "exercise",
        presentation: "threeStep",
        defaultCwd: null,
        entryFile: null,
        summaryHtml: "<p>先写一页需求，再写代码。</p>",
        blocks: [{ kind: "prose", html: "<p>定义 Task 结构与接口合同。</p>" }],
        commands: [],
        threeStep: {
          step1IntroHtml: null,
          step1CommandIds: [],
          step2EntryFile: null,
          step2Html: "<p>定义 Task 结构与接口合同。</p>",
          step3IntroHtml: null,
          step3Checks: [
            {
              id: "s1",
              title: "POST /tasks 正常创建返回 201",
              kind: "selfCheck",
              required: false,
              cwd: null,
              command: null,
              method: null,
              url: null,
            },
          ],
          requiresDaemon: null,
          secondaryCommandIds: [],
          reflectionsHtml:
            "<p>“任务不存在”和“数据库无法访问”，为什么不能都返回同一个错误？</p>",
        },
        reflectionsHtml:
          "<p>“任务不存在”和“数据库无法访问”，为什么不能都返回同一个错误？</p>",
      },
      {
        chapterId: "ex-1",
        title: "练习 1：让一个请求完成往返",
        kind: "exercise",
        presentation: "threeStep",
        defaultCwd: null,
        entryFile: "task-api/main.go",
        summaryHtml: "<p>先理解 HTTP 服务最小需要什么。</p>",
        blocks: [
          { kind: "prose", html: "<p>在学习工作区建立项目并启动服务。</p>" },
          { kind: "command", commandId: "ex-1:cmd-1" },
          { kind: "command", commandId: "ex-1:cmd-2" },
        ],
        commands: [
          {
            id: "ex-1:cmd-1",
            shell: "bash",
            command: "go run .",
            cwd: "task-api",
            cwdOrigin: "inferred",
            mode: "daemon",
            modeOrigin: "inferred",
            label: null,
            port: 8080,
            isSecondary: false,
          },
          {
            id: "ex-1:cmd-2",
            shell: "bash",
            command: "curl -i http://127.0.0.1:8080/healthz",
            cwd: "task-api",
            cwdOrigin: "inferred",
            mode: "oneshot",
            modeOrigin: "inferred",
            label: null,
            port: null,
            isSecondary: true,
          },
        ],
        threeStep: {
          step1IntroHtml: "<p>启动本地 HTTP 服务：</p>",
          step1CommandIds: ["ex-1:cmd-1"],
          step2EntryFile: "task-api/main.go",
          step2Html:
            "<p>注册 <code>GET /healthz</code> 路由并返回 JSON 响应。</p>",
          step3IntroHtml: null,
          step3Checks: [
            {
              id: "v1",
              title: 'GET /healthz 返回 200 {"status":"ok"}',
              kind: "httpAssert",
              required: true,
              cwd: null,
              command: null,
              method: "GET",
              url: "http://127.0.0.1:8080/healthz",
            },
            {
              id: "v2",
              title: "POST /healthz 返回 405 Method Not Allowed",
              kind: "httpAssert",
              required: true,
              cwd: null,
              command: null,
              method: "POST",
              url: "http://127.0.0.1:8080/healthz",
            },
          ],
          requiresDaemon: {
            label: "go run .",
            port: 8080,
            serviceKey: "task-api",
          },
          secondaryCommandIds: ["ex-1:cmd-2"],
          reflectionsHtml:
            "<ul><li><code>http.Server</code> 和 <code>http.ServeMux</code> 各负责什么？</li></ul>",
        },
        reflectionsHtml:
          "<ul><li><code>http.Server</code> 和 <code>http.ServeMux</code> 各负责什么？</li></ul>",
      },
    ],
  };
}

export function summarizeCourse(course: LoadedCourse): CourseSummary {
  return {
    courseId: course.courseId,
    title: course.title,
    description: course.description,
    language: course.language,
    source: course.source,
    totalChapters: course.chapters.length,
    totalExercises: course.totalExercises,
    isBuiltin: course.isBuiltin,
  };
}

export interface FakeCoreClientOptions {
  preflight?: PreflightReport;
  settings?: Partial<GlobalSettings>;
  recents?: RecentWorkspace[];
  trustEntries?: TrustEntry[];
  recoveryNotices?: StorageRecoveryNotice[];
  builtinCourses?: LoadedCourse[];
  localCourses?: Record<string, LoadedCourse>;
  pickedPaths?: Partial<Record<LocalPathPickKind, string | null>>;
}

export class FakeCoreClient implements CoreClient {
  private preflight: PreflightReport;
  private settings: GlobalSettings;
  private recents: RecentWorkspace[];
  private trustEntries: TrustEntry[];
  private recoveryNotices: StorageRecoveryNotice[];
  private builtinCourses = new Map<string, LoadedCourse>();
  private localCourses = new Map<string, LoadedCourse>();
  private pickedPaths: Partial<Record<LocalPathPickKind, string | null>>;
  private activeCourseKey: { kind: "builtin" | "local"; key: string } | null =
    null;
  private shutDown = false;
  private readonly listeners = new Set<CoreEventListener>();
  private nextErrors = new Map<AppCommand["type"], DomainError>();
  readonly commandLog: AppCommand[] = [];

  constructor(options: FakeCoreClientOptions = {}) {
    this.settings = { ...DEFAULT_SETTINGS, ...options.settings };
    this.preflight = options.preflight ?? {
      revision: 0,
      checked: true,
      probing: false,
      installingCount: 0,
      languages: [],
      tools: [],
      goProxy: null,
      installDir: "/codelab/runtimes",
    };
    this.recents = options.recents ? [...options.recents] : [];
    this.trustEntries = options.trustEntries ? [...options.trustEntries] : [];
    this.recoveryNotices = options.recoveryNotices
      ? [...options.recoveryNotices]
      : [];
    const builtins = options.builtinCourses ?? [createBuiltinGoCourseFixture()];
    for (const course of builtins) {
      this.builtinCourses.set(course.courseId, course);
    }
    if (options.localCourses) {
      for (const [path, course] of Object.entries(options.localCourses)) {
        this.localCourses.set(path, course);
      }
    }
    this.pickedPaths = { ...options.pickedPaths };
  }

  registerLocalCourse(path: string, course: LoadedCourse): void {
    this.localCourses.set(path, course);
  }

  setPickedPath(kind: LocalPathPickKind, path: string | null): void {
    this.pickedPaths[kind] = path;
  }

  failNext(commandType: AppCommand["type"], error: DomainError): void {
    this.nextErrors.set(commandType, error);
  }

  emitEvent(event: AppEvent): void {
    if (event.type === "preflightUpdated") {
      this.preflight = event.payload;
    }
    for (const listener of this.listeners) {
      listener(event);
    }
  }

  async pickLocalPath(kind: LocalPathPickKind): Promise<string | null> {
    return this.pickedPaths[kind] ?? null;
  }

  private resolveCourse(request: OpenCourseRequest): {
    course: LoadedCourse;
    activeKey: { kind: "builtin" | "local"; key: string };
  } {
    if (request.kind === "builtin") {
      const found = this.builtinCourses.get(request.courseId);
      if (!found) {
        throw new CoreCommandError({
          code: "COURSE_NOT_FOUND",
          details: { source: request.courseId },
        });
      }
      return {
        course: found,
        activeKey: { kind: "builtin", key: request.courseId },
      };
    }

    const trimmed = request.path.trim();
    if (!trimmed.startsWith("/")) {
      throw new CoreCommandError({
        code: "COURSE_PATH_INVALID",
        details: { path: trimmed },
      });
    }
    const found = this.localCourses.get(trimmed);
    if (!found) {
      throw new CoreCommandError({
        code: "COURSE_NOT_FOUND",
        details: { source: trimmed },
      });
    }
    return { course: found, activeKey: { kind: "local", key: trimmed } };
  }

  async dispatch(command: AppCommand): Promise<AppResponse> {
    this.commandLog.push(command);

    const injected = this.nextErrors.get(command.type);
    if (injected) {
      this.nextErrors.delete(command.type);
      throw new CoreCommandError(injected);
    }

    switch (command.type) {
      case "bootstrap":
        return {
          type: "bootstrap",
          payload: {
            settings: { ...this.settings },
            recents: [...this.recents],
            trustEntries: [...this.trustEntries],
            recoveryNotices: [...this.recoveryNotices],
            builtinCourses: Array.from(this.builtinCourses.values()).map(
              summarizeCourse
            ),
          },
        };
      case "getSettings":
        return { type: "settings", payload: { ...this.settings } };
      case "updateSettings": {
        const patch = command.payload;
        if (
          patch.codeFontSize !== undefined &&
          (!Number.isFinite(patch.codeFontSize) ||
            patch.codeFontSize < 11 ||
            patch.codeFontSize > 20)
        ) {
          throw new CoreCommandError({
            code: "INVALID_CODE_FONT_SIZE",
            details: { min: "11", max: "20" },
          });
        }
        this.settings = {
          ...this.settings,
          ...(patch.theme !== undefined ? { theme: patch.theme } : {}),
          ...(patch.accent !== undefined ? { accent: patch.accent } : {}),
          ...(patch.codeFontSize !== undefined
            ? { codeFontSize: patch.codeFontSize }
            : {}),
          ...(patch.locale !== undefined ? { locale: patch.locale } : {}),
        };
        this.emitEvent({
          type: "settingsUpdated",
          payload: { ...this.settings },
        });
        return { type: "settings", payload: { ...this.settings } };
      }
      case "listRecents":
        return { type: "recents", payload: [...this.recents] };
      case "recordRecentWorkspace": {
        const input = command.payload;
        const entry: RecentWorkspace = {
          ...input,
          workspacePath: input.workspacePath.trim(),
          lastOpenedAt: new Date().toISOString(),
        };
        this.recents = [
          entry,
          ...this.recents.filter((r) => r.workspacePath !== entry.workspacePath),
        ].slice(0, 20);
        this.emitEvent({ type: "recentsUpdated", payload: [...this.recents] });
        return { type: "recents", payload: [...this.recents] };
      }
      case "removeRecentWorkspace": {
        const target = command.payload.workspacePath.trim();
        this.recents = this.recents.filter((r) => r.workspacePath !== target);
        this.emitEvent({ type: "recentsUpdated", payload: [...this.recents] });
        return { type: "recents", payload: [...this.recents] };
      }
      case "listTrustEntries":
        return { type: "trustEntries", payload: [...this.trustEntries] };
      case "checkTrust": {
        const normalized = command.payload.source.trim();
        const found = this.trustEntries.find(
          (entry) =>
            entry.sourceKind === command.payload.sourceKind &&
            entry.normalizedSource === normalized
        );
        return {
          type: "trustState",
          payload: {
            sourceKind: command.payload.sourceKind,
            normalizedSource: normalized,
            trusted: Boolean(found),
            trustedAt: found?.trustedAt ?? null,
          },
        };
      }
      case "grantTrust": {
        const normalized = command.payload.source.trim();
        const trustedAt = new Date().toISOString();
        this.trustEntries = [
          {
            sourceKind: command.payload.sourceKind,
            normalizedSource: normalized,
            trustedAt,
          },
          ...this.trustEntries.filter(
            (e) =>
              !(
                e.sourceKind === command.payload.sourceKind &&
                e.normalizedSource === normalized
              )
          ),
        ];
        this.emitEvent({
          type: "trustUpdated",
          payload: [...this.trustEntries],
        });
        return {
          type: "trustState",
          payload: {
            sourceKind: command.payload.sourceKind,
            normalizedSource: normalized,
            trusted: true,
            trustedAt,
          },
        };
      }
      case "revokeTrust": {
        const normalized = command.payload.source.trim();
        this.trustEntries = this.trustEntries.filter(
          (e) =>
            !(
              e.sourceKind === command.payload.sourceKind &&
              e.normalizedSource === normalized
            )
        );
        this.emitEvent({
          type: "trustUpdated",
          payload: [...this.trustEntries],
        });
        return { type: "trustEntries", payload: [...this.trustEntries] };
      }
      case "listBuiltinCourses":
        return {
          type: "builtinCourses",
          payload: Array.from(this.builtinCourses.values()).map(summarizeCourse),
        };
      case "openCourse": {
        const { course, activeKey } = this.resolveCourse(command.payload);
        this.activeCourseKey = activeKey;
        this.emitEvent({ type: "courseLoaded", payload: course });
        return { type: "loadedCourse", payload: course };
      }
      case "reloadActiveCourse": {
        if (!this.activeCourseKey) {
          throw new CoreCommandError({ code: "NO_ACTIVE_COURSE" });
        }
        const course =
          this.activeCourseKey.kind === "builtin"
            ? this.builtinCourses.get(this.activeCourseKey.key)
            : this.localCourses.get(this.activeCourseKey.key);
        if (!course) {
          throw new CoreCommandError({
            code: "COURSE_NOT_FOUND",
            details: { source: this.activeCourseKey.key },
          });
        }
        this.emitEvent({ type: "courseLoaded", payload: course });
        return { type: "loadedCourse", payload: course };
      }
      case "closeActiveCourse": {
        this.activeCourseKey = null;
        return { type: "courseClosed" };
      }
      case "getPreflight":
      case "refreshPreflight":
        return { type: "preflight", payload: this.preflight };
      case "setRuntimePreference":
      case "installManagedRuntime": {
        const languages = this.preflight.languages.map((row) => {
          if (row.language !== command.payload.language) return row;
          if (command.type === "setRuntimePreference") {
            return {
              ...row,
              preferManaged: command.payload.preferManaged,
              canInstall: command.payload.preferManaged,
            };
          }
          return { ...row, canInstall: false, install: { phase: "preparing" as const } };
        });
        this.emitEvent({
          type: "preflightUpdated",
          payload: {
            ...this.preflight,
            revision: this.preflight.revision + 1,
            installingCount: command.type === "installManagedRuntime"
              ? this.preflight.installingCount + 1
              : this.preflight.installingCount,
            languages,
          },
        });
        return { type: "preflight", payload: this.preflight };
      }
      case "shutdown": {
        const report = {
          alreadyShutDown: this.shutDown,
          hooksExecuted: 0,
        };
        if (!this.shutDown) {
          this.shutDown = true;
          this.emitEvent({ type: "shutdownCompleted", payload: report });
        }
        return { type: "shutdown", payload: report };
      }
    }
  }

  async subscribe(listener: CoreEventListener): Promise<Unsubscribe> {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }
}

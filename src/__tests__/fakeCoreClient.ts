import type { CoreClient, CoreEventListener, Unsubscribe } from "../core/client";
import {
  ACP_AGENT_PRESETS,
  CoreCommandError,
  DEFAULT_SETTINGS,
  DEFAULT_TUTOR_SETTINGS,
  MAX_CONTEXT_INPUT_TOKENS,
  type AppCommand,
  type AppEvent,
  type AppResponse,
  type ContextChip,
  type ContextDimension,
  type ContextQuote,
  type ContextSnapshot,
  type ContextSnapshotInput,
  type CourseSummary,
  type DomainError,
  type GlobalSettings,
  type LoadedCourse,
  type LocalMetricEvent,
  type LocalMetricRecord,
  type LocalPathPickKind,
  type MetricEventDetail,
  type MetricEventName,
  type OpenCourseRequest,
  type PreflightReport,
  type RecentWorkspace,
  type StorageRecoveryNotice,
  type TrustEntry,
  type TutorFileChange,
  type TutorHandshakeResult,
  type TutorMessage,
  type TutorPermissionRequest,
  type TutorSessionSnapshot,
  type WorkspaceEntry,
  type WorkspaceFile,
  type WorkspaceMetrics,
  type WorkspaceSummary,
  type TerminalSummary,
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
    trusted: true,
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

export function createBuiltinTsCourseFixture(): LoadedCourse {
  return {
    courseId: "ts-data-pipeline",
    title: "TypeScript 数据流水线实战",
    description: "多文件函数判题：筛选过滤、去重聚合与并发限流。",
    language: "JS/TS",
    version: "1.0.0",
    author: "CodeLab Studio",
    dependencies: ["bun>=1.1 | node>=22"],
    source: { kind: "builtin", courseId: "ts-data-pipeline" },
    entryMarkdownPath: null,
    isBuiltin: true,
    trusted: true,
    introHtml: "<p>多文件 TypeScript 数据流水线练习。</p>",
    totalExercises: 8,
    chapters: [
      {
        chapterId: "ex-1",
        title: "练习 1：成年用户筛选与不可变映射",
        kind: "exercise",
        presentation: "threeStep",
        defaultCwd: null,
        entryFile: "src/filter.ts",
        summaryHtml: "<p>补全 <code>getAdultNames</code>，筛出成年用户姓名。</p>",
        blocks: [{ kind: "prose", html: "<p>编辑 <code>src/filter.ts</code>。</p>" }],
        commands: [
          {
            id: "ex-1:cmd-1",
            shell: "bash",
            command: "bun test tests/ex1-filter.test.ts",
            cwd: null,
            cwdOrigin: "workspaceRoot",
            mode: "oneshot",
            modeOrigin: "explicit",
            label: null,
            port: null,
            isSecondary: false,
          },
        ],
        threeStep: {
          step1IntroHtml: null,
          step1CommandIds: ["ex-1:cmd-1"],
          step2EntryFile: "src/filter.ts",
          step2Html: "<p>仅保留 <code>age &gt;= 18</code> 的用户。</p>",
          step3IntroHtml: null,
          step3Checks: [
            {
              id: "v1",
              title: "用户筛选单测全部通过（边界 18 岁与不可变性）",
              kind: "testSuite",
              required: true,
              cwd: null,
              command: "bun test tests/ex1-filter.test.ts",
              method: null,
              url: null,
              timeoutSec: 5,
            },
          ],
          requiresDaemon: null,
          secondaryCommandIds: [],
          reflectionsHtml: null,
        },
        reflectionsHtml: null,
      },
    ],
  };
}

export function createBuiltinPythonCourseFixture(): LoadedCourse {
  return {
    courseId: "python-data-clean",
    title: "Python 数据清洗与订单统计",
    description: "通过 uv 隔离环境运行多模块 pytest 自动化验收。",
    language: "Python",
    version: "1.0.0",
    author: "CodeLab Studio",
    dependencies: ["python>=3.11", "pytest", "pytest-json-report"],
    source: { kind: "builtin", courseId: "python-data-clean" },
    entryMarkdownPath: null,
    isBuiltin: true,
    trusted: true,
    introHtml: "<p>使用 pytest --json-report 验证数据清洗结果。</p>",
    totalExercises: 6,
    chapters: [
      {
        chapterId: "ex-1",
        title: "练习 1：清洗无效订单并按类目汇总金额",
        kind: "exercise",
        presentation: "threeStep",
        defaultCwd: null,
        entryFile: "src/cleaner.py",
        summaryHtml: "<p>过滤非正金额与取消订单并用 Decimal 汇总。</p>",
        blocks: [{ kind: "prose", html: "<p>编辑 <code>src/cleaner.py</code>。</p>" }],
        commands: [
          {
            id: "ex-1:cmd-1",
            shell: "bash",
            command: "pytest -q tests/test_ex1_cleaner.py",
            cwd: null,
            cwdOrigin: "workspaceRoot",
            mode: "oneshot",
            modeOrigin: "explicit",
            label: null,
            port: null,
            isSecondary: false,
          },
        ],
        threeStep: {
          step1IntroHtml: null,
          step1CommandIds: ["ex-1:cmd-1"],
          step2EntryFile: "src/cleaner.py",
          step2Html: "<p>实现 <code>sum_valid_by_category</code>。</p>",
          step3IntroHtml: null,
          step3Checks: [
            {
              id: "v1",
              title: "订单清洗与类目金额汇总 pytest 用例全部通过",
              kind: "testSuite",
              required: true,
              cwd: null,
              command: "pytest --json-report tests/test_ex1_cleaner.py",
              method: null,
              url: null,
              timeoutSec: 8,
            },
          ],
          requiresDaemon: null,
          secondaryCommandIds: [],
          reflectionsHtml: null,
        },
        reflectionsHtml: null,
      },
    ],
  };
}

export function createBuiltinJavaCourseFixture(): LoadedCourse {
  return {
    courseId: "java-rate-limiter",
    title: "Java 21 并发限流器实战",
    description: "Temurin JDK 21 与 JUnit 5 多线程安全验收。",
    language: "Java",
    version: "1.0.0",
    author: "CodeLab Studio",
    dependencies: ["jdk>=21", "junit5"],
    source: { kind: "builtin", courseId: "java-rate-limiter" },
    entryMarkdownPath: null,
    isBuiltin: true,
    trusted: true,
    introHtml: "<p>使用 JDK 21 与 JUnit 5 验证并发限流器。</p>",
    totalExercises: 7,
    chapters: [
      {
        chapterId: "ex-1",
        title: "练习 1：线程安全的固定令牌桶",
        kind: "exercise",
        presentation: "threeStep",
        defaultCwd: null,
        entryFile: "src/main/java/com/codelab/ratelimiter/TokenBucket.java",
        summaryHtml: "<p>修复 <code>TokenBucket.tryAcquire()</code> 并发竞态。</p>",
        blocks: [{ kind: "prose", html: "<p>编辑 <code>TokenBucket.java</code>。</p>" }],
        commands: [],
        threeStep: {
          step1IntroHtml: null,
          step1CommandIds: [],
          step2EntryFile: "src/main/java/com/codelab/ratelimiter/TokenBucket.java",
          step2Html: "<p>添加同步控制并修正边界条件。</p>",
          step3IntroHtml: null,
          step3Checks: [
            {
              id: "v1",
              title: "固定令牌桶边界与多线程安全 JUnit 5 测试通过",
              kind: "testSuite",
              required: true,
              cwd: null,
              command: "junit --select-class com.codelab.ratelimiter.Ex1TokenBucketTest",
              method: null,
              url: null,
              timeoutSec: 12,
            },
          ],
          requiresDaemon: null,
          secondaryCommandIds: [],
          reflectionsHtml: null,
        },
        reflectionsHtml: null,
      },
    ],
  };
}

export function createBuiltinCourseFixtures(): LoadedCourse[] {
  return [
    createBuiltinGoCourseFixture(),
    createBuiltinTsCourseFixture(),
    createBuiltinPythonCourseFixture(),
    createBuiltinJavaCourseFixture(),
  ];
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
  chapterSolutions?: Record<string, Record<string, string>>;
  starterFiles?: Record<string, string>;
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

  constructor(private options: FakeCoreClientOptions = {}) {
    this.settings = {
      ...DEFAULT_SETTINGS,
      ...options.settings,
      tutor: {
        ...DEFAULT_TUTOR_SETTINGS,
        ...(options.settings?.tutor ?? {}),
      },
    };
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
    const builtins = options.builtinCourses ?? createBuiltinCourseFixtures();
    for (const course of builtins) {
      this.builtinCourses.set(course.courseId, course);
    }
    if (options.localCourses) {
      for (const [path, course] of Object.entries(options.localCourses)) {
        this.localCourses.set(path, course);
      }
    }
    this.pickedPaths = { ...options.pickedPaths };
    this.pendingMetrics.push(this.makeMetricRecord({ detailKind: "appLaunched" }));
  }

  private workspace: WorkspaceSummary | null = null;
  private nextWorkspaceId = 1;
  private nextTerminalId = 1;
  private pendingConfirmation: number | null = null;
  private terminals: (TerminalSummary & { workspaceId: number })[] = [];
  private cwdOverrides = new Map<string, Record<string, string>>();
  private terminalErrors: string[] = [];
  private terminalOutput = new Map<number, string>();
  private workspaceMetrics = new Map<string, WorkspaceMetrics>();
  private pendingMetrics: LocalMetricRecord[] = [];

  private metricEventName(detail: MetricEventDetail): MetricEventName {
    switch (detail.detailKind) {
      case "appLaunched": return "app_launched";
      case "courseOpened": return "course_opened";
      case "preflightExecuted": return "preflight_executed";
      case "runtimeInstalled": return "runtime_installed";
      case "cmdBlockClicked": return "cmd_block_clicked";
      case "cmdRiskModalAction": return "cmd_risk_modal_action";
      case "cmdFinished": return "cmd_finished";
      case "fileTreeOpen": return "file_tree_open";
      case "selectionAiClicked": return "selection_ai_clicked";
      case "aiModeSwitched": return "ai_mode_switched";
      case "acpPermissionDecision": return "acp_permission_decision";
      case "verifyChapterClicked": return "verify_chapter_clicked";
      case "chapterMarkedDone": return "chapter_marked_done";
    }
  }

  private makeMetricRecord(detail: MetricEventDetail): LocalMetricRecord {
    return {
      event: this.metricEventName(detail),
      recordedAt: new Date().toISOString(),
      ...detail,
    };
  }

  private metricsForRoot(root: string): WorkspaceMetrics {
    const existing = this.workspaceMetrics.get(root);
    if (existing) return existing;
    const initial: WorkspaceMetrics = { courseOpened: 0, counts: {}, events: [] };
    this.workspaceMetrics.set(root, initial);
    return initial;
  }

  private pushMetricRecord(root: string, record: LocalMetricRecord) {
    const metrics = this.metricsForRoot(root);
    const nextCount = (metrics.counts[record.event] ?? 0) + 1;
    metrics.counts[record.event] = nextCount;
    if (record.event === "course_opened") {
      metrics.courseOpened = nextCount;
    }
    metrics.events.push(record);
    if (metrics.events.length > 500) {
      metrics.events.splice(0, metrics.events.length - 500);
    }
  }

  private recordMetric(detail: MetricEventDetail) {
    const record = this.makeMetricRecord(detail);
    if (this.workspace) {
      this.pushMetricRecord(this.workspace.root, record);
    } else {
      this.pendingMetrics.push(record);
    }
  }

  private localEventToDetail(event: LocalMetricEvent): MetricEventDetail {
    switch (event.kind) {
      case "fileTreeOpen":
        return { detailKind: "fileTreeOpen" };
      case "selectionAiClicked":
        return { detailKind: "selectionAiClicked", zone: event.zone, action: event.action };
      case "cmdRiskModalCancelled":
        return { detailKind: "cmdRiskModalAction", source: event.source, decision: "cancel" };
    }
  }

  writeTerminalOutput(text: string) {
    const id = this.terminals.at(-1)?.id;
    if (id !== undefined) this.terminalOutput.set(id, (this.terminalOutput.get(id) ?? "") + text);
  }

  markPortConflict(terminalId: number, error: string) {
    this.terminals = this.terminals.map((terminal) =>
      terminal.id === terminalId ? { ...terminal, portConflict: true } : terminal);
    this.terminalErrors.push(error);
  }

  private addTerminal(workspaceId: number, cwd: string, block?: LoadedCourse["chapters"][number]["commands"][number]) {
    const terminal = {
      workspaceId, id: this.nextTerminalId++, cwd, commandId: block?.id ?? null,
      label: block?.label ?? null, mode: block?.mode ?? null, port: block?.port ?? null,
      startedAt: new Date().toISOString(), running: true, exitCode: null, portConflict: false,
    };
    this.terminals.push(terminal);
    return terminal;
  }
  private nextFileRevision = 1;
  private readonly disk = new Map<string, Map<string, WorkspaceFile>>();

  writeDiskFile(root: string, path: string, text: string): WorkspaceFile {
    const file: WorkspaceFile = {
      kind: "text",
      path,
      text,
      sizeBytes: text.length,
      revision: String(this.nextFileRevision++),
    };
    this.files(root).set(path, file);
    this.notifyDiskChange(root, path);
    return file;
  }

  deleteDiskFile(root: string, path: string) {
    this.files(root).delete(path);
    this.notifyDiskChange(root, path);
  }

  readDiskFile(root: string, path: string): WorkspaceFile | undefined {
    return this.files(root).get(path);
  }

  private notifyDiskChange(root: string, path: string) {
    if (this.workspace?.root === root) {
      this.emitEvent({ type: "workspaceFilesChanged", payload: { workspaceId: this.workspace.id, paths: [path] } });
    }
  }

  private files(root: string) {
    const existing = this.disk.get(root);
    if (existing) return existing;
    const files = new Map<string, WorkspaceFile>();
    this.disk.set(root, files);
    return files;
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
    if (event.type === "progressUpdated") this.progress = event.payload;
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

    const trimmed = (request.kind === "gitPack" ? request.url : request.path).trim();
    if (request.kind !== "gitPack" && !trimmed.startsWith("/")) {
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
    return { course: this.withTrust(found), activeKey: { kind: "local", key: trimmed } };
  }

  private activeCourse(): LoadedCourse | null {
    if (!this.activeCourseKey) return null;
    const { kind, key } = this.activeCourseKey;
    return (kind === "builtin" ? this.builtinCourses : this.localCourses).get(key) ?? null;
  }

  private withTrust(course: LoadedCourse): LoadedCourse {
    const source = course.source;
    return {
      ...course,
      trusted: source.kind === "builtin" || this.trustEntries.some(
        (entry) => source.kind === "gitPack"
          ? entry.sourceKind === "gitRepo" && entry.normalizedSource === source.url
          : entry.sourceKind === "localPath" && entry.normalizedSource === source.path
      ),
    };
  }

  private publishTrust() {
    const active = this.activeCourse();
    if (active) {
      const { source, trusted } = this.withTrust(active);
      this.emitEvent({ type: "courseTrustChanged", payload: { source, trusted } });
    }
    this.emitEvent({ type: "trustUpdated", payload: [...this.trustEntries] });
  }

  private bindWorkspace(root: string) {
    const active = this.activeCourse();
    if (!active) throw new CoreCommandError({ code: "NO_ACTIVE_COURSE" });
    if (!this.withTrust(active).trusted) throw new CoreCommandError({ code: "COURSE_UNTRUSTED" });
    this.workspace = { id: this.nextWorkspaceId++, root, recoveryNotices: [] };
    for (const pending of this.pendingMetrics.splice(0)) {
      this.pushMetricRecord(root, pending);
    }
    this.pushMetricRecord(
      root,
      this.makeMetricRecord({
        detailKind: "courseOpened",
        courseKind: active.source.kind,
        language: active.language,
      })
    );
    this.recents = [{
      workspacePath: root,
      courseSource: active.source,
      courseId: active.courseId,
      courseTitle: active.title,
      language: active.language,
      completedExercises: 0,
      totalExercises: active.totalExercises,
      lastOpenedAt: new Date().toISOString(),
    }, ...this.recents.filter((recent) => recent.workspacePath !== root)];
    this.emitEvent({ type: "recentsUpdated", payload: [...this.recents] });
    return this.workspace;
  }

  private workspaceRoot(workspaceId: number): string {
    const active = this.activeCourse();
    if (active && !this.withTrust(active).trusted) throw new CoreCommandError({ code: "COURSE_UNTRUSTED" });
    if (!this.workspace || this.workspace.id !== workspaceId) throw new CoreCommandError({ code: "NO_ACTIVE_WORKSPACE" });
    return this.workspace.root;
  }

  progress: import("../core/contracts").LearningProgress | null = null;
  private tutorSeq = 1;
  private tutorSessions = new Map<number, TutorSessionSnapshot>();
  private tutorHandshakeOverride: TutorHandshakeResult | null = null;

  setTutorHandshakeResult(result: TutorHandshakeResult | null) {
    this.tutorHandshakeOverride = result;
  }

  private estimateTokens(text: string): number {
    let ascii = 0;
    let nonAscii = 0;
    for (const ch of text) {
      if ((ch.codePointAt(0) ?? 0) <= 0x7f) ascii += 1;
      else nonAscii += 1;
    }
    return nonAscii + Math.ceil(ascii / 4);
  }

  private isSensitivePath(rawPath: string): boolean {
    const parts = rawPath.replaceAll("\\", "/").split("/").filter(Boolean);
    return parts.some((part) => {
      const lower = part.toLowerCase();
      return (
        lower === ".codelab" ||
        lower === ".env" ||
        lower.startsWith(".env.") ||
        lower.includes("secret") ||
        lower.includes("key")
      );
    });
  }

  private stripHtml(html: string): string {
    return html.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim();
  }

  private buildTutorContextSnapshot(input: ContextSnapshotInput): ContextSnapshot {
    const root = this.workspaceRoot(input.workspaceId);
    const course = this.activeCourse();
    const excluded = new Set<ContextDimension>(input.excludedDimensions ?? []);
    let sensitiveFiltered = false;

    let quote: ContextQuote | null = input.quote?.text.trim()
      ? { ...input.quote, text: input.quote.text.trim() }
      : null;
    if (quote?.filePath && this.isSensitivePath(quote.filePath)) {
      sensitiveFiltered = true;
      quote = null;
    }

    const chapter =
      course?.chapters.find((c) => c.chapterId === input.chapterId) ??
      course?.chapters.find((c) => c.kind === "exercise") ??
      course?.chapters[0] ??
      null;

    const chips: ContextChip[] = [];
    const sections: string[] = [];

    if (quote) {
      sections.push(`### User Selection (source: ${quote.source})\n${quote.text}`);
    }

    if (!excluded.has("terminalErrors") && this.terminalErrors.length > 0) {
      const lines = this.terminalErrors.slice(-200);
      const preview = `### Terminal Error Tail (latest ${lines.length} lines)\n${lines.join("\n")}`;
      sections.push(preview);
      chips.push({
        dimension: "terminalErrors",
        label: String(lines.length),
        detail: null,
        preview,
        estimatedTokens: this.estimateTokens(preview),
        truncated: false,
      });
    }

    if (!excluded.has("verification") && chapter) {
      const specs = chapter.threeStep?.step3Checks ?? [];
      const results = this.progress?.chapters[chapter.chapterId]?.lastCheckResults ?? [];
      if (specs.length > 0 || results.length > 0) {
        const passed = results.filter((r) => r.status === "passed").length;
        const total = results.length || specs.length;
        const lines =
          results.length > 0
            ? results.map((r) => {
                const title = specs.find((s) => s.id === r.checkId)?.title ?? r.checkId;
                const diffText = r.diffs
                  .map((d) => `  - diff (${d.label}): expected \`${d.expected}\` | actual \`${d.actual}\``)
                  .join("\n");
                return `- [${r.status}] ${title} (${r.durationMs}ms)${diffText ? `\n${diffText}` : ""}`;
              })
            : specs.map((s) => `- [pending] ${s.title}`);
        const preview = `### Verification Status (chapter: ${chapter.chapterId})\n${lines.join("\n")}`;
        sections.push(preview);
        chips.push({
          dimension: "verification",
          label: `${passed}/${total}`,
          detail: this.progress?.chapters[chapter.chapterId]?.completionState ?? "not_started",
          preview,
          estimatedTokens: this.estimateTokens(preview),
          truncated: false,
        });
      }
    }

    if (!excluded.has("activeFile") && input.activeFilePath?.trim()) {
      const filePath = input.activeFilePath.trim();
      if (this.isSensitivePath(filePath)) {
        sensitiveFiltered = true;
      } else {
        const disk = this.files(root).get(filePath);
        const rawText = input.activeFileText ?? (disk?.kind === "text" ? disk.text : null);
        if (rawText && rawText.trim().length > 0) {
          const allLines = rawText.split("\n");
          const start = Math.max(1, (input.activeFileStartLine ?? 1) - 40);
          const end = Math.min(allLines.length, (input.activeFileEndLine ?? Math.min(allLines.length, 120)) + 40);
          const body = allLines
            .slice(start - 1, end)
            .map((line, idx) => `${String(start + idx).padStart(4, " ")} | ${line}`)
            .join("\n");
          const preview = `### Active File Snippet (${filePath}, L${start}-L${end})\n${body}`;
          sections.push(preview);
          chips.push({
            dimension: "activeFile",
            label: filePath,
            detail: `L${start}-L${end}`,
            preview,
            estimatedTokens: this.estimateTokens(preview),
            truncated: false,
          });
        }
      }
    }

    if (!excluded.has("chapter") && chapter) {
      const summary = this.stripHtml(
        [chapter.summaryHtml, chapter.threeStep?.step2Html, chapter.reflectionsHtml]
          .filter(Boolean)
          .join(" ")
      );
      const preview = `### Current Chapter (${chapter.chapterId}: ${chapter.title})\n${summary}`;
      sections.push(preview);
      chips.push({
        dimension: "chapter",
        label: chapter.title,
        detail: chapter.chapterId,
        preview,
        estimatedTokens: this.estimateTokens(preview),
        truncated: false,
      });
    }

    // Keep chips in canonical 4D order: Chapter, ActiveFile, TerminalErrors, Verification.
    const order: ContextDimension[] = ["chapter", "activeFile", "terminalErrors", "verification"];
    chips.sort((a, b) => order.indexOf(a.dimension) - order.indexOf(b.dimension));

    const renderedPreamble =
      sections.length > 0 ? `[Context Snapshot]\n\n${sections.join("\n\n")}` : "";
    const styleTokens = 32;
    const promptTokens = this.estimateTokens(input.prompt ?? "");
    const estimatedTokens = Math.min(
      MAX_CONTEXT_INPUT_TOKENS,
      styleTokens + promptTokens + this.estimateTokens(renderedPreamble)
    );

    return {
      workspaceId: input.workspaceId,
      chapterId: chapter?.chapterId ?? null,
      quote,
      chips,
      excludedDimensions: [...excluded],
      estimatedTokens,
      maxTokens: MAX_CONTEXT_INPUT_TOKENS,
      truncated: chips.some((c) => c.truncated),
      sensitiveFiltered,
      renderedPreamble,
    };
  }

  private ensureTutorSession(workspaceId: number): TutorSessionSnapshot {
    this.workspaceRoot(workspaceId);
    const existing = this.tutorSessions.get(workspaceId);
    if (existing) return existing;
    const initial: TutorSessionSnapshot = {
      workspaceId,
      status: "idle",
      processRunning: false,
      sessionReused: false,
      preset: this.settings.tutor.preset,
      command: this.settings.tutor.command,
      agentName: null,
      permissionPolicy: this.settings.tutor.permissionPolicy,
      teachingStyle: this.settings.tutor.teachingStyle,
      activeTurnId: null,
      lastError: null,
      canRetry: false,
      messages: [],
    };
    this.tutorSessions.set(workspaceId, initial);
    return initial;
  }

  private updateTutorSession(
    workspaceId: number,
    updater: (session: TutorSessionSnapshot) => TutorSessionSnapshot
  ): TutorSessionSnapshot {
    const next = updater(this.ensureTutorSession(workspaceId));
    this.tutorSessions.set(workspaceId, next);
    this.emitEvent({ type: "tutorSessionUpdated", payload: next });
    return next;
  }

  emitTutorChunk(workspaceId: number, delta: string) {
    return this.updateTutorSession(workspaceId, (session) => {
      const messages = session.messages.map((msg, idx) =>
        idx === session.messages.length - 1 && msg.role === "assistant"
          ? { ...msg, content: msg.content + delta }
          : msg
      );
      return { ...session, status: "streaming", messages };
    });
  }

  emitTutorPermission(
    workspaceId: number,
    permission: Omit<TutorPermissionRequest, "turnId" | "createdAt" | "resolvedAt">
  ) {
    return this.updateTutorSession(workspaceId, (session) => {
      const turnId = session.activeTurnId ?? "turn-1";
      const record: TutorPermissionRequest = {
        ...permission,
        turnId,
        createdAt: new Date().toISOString(),
        resolvedAt: permission.status === "pending" ? null : new Date().toISOString(),
      };
      const messages = session.messages.map((msg, idx) =>
        idx === session.messages.length - 1 && msg.role === "assistant"
          ? { ...msg, permissions: [...msg.permissions, record] }
          : msg
      );
      return {
        ...session,
        status: permission.status === "pending" ? "awaitingPermission" : session.status,
        messages,
      };
    });
  }

  emitTutorFileChange(
    workspaceId: number,
    path: string,
    afterText: string
  ): TutorSessionSnapshot {
    const root = this.workspaceRoot(workspaceId);
    const existing = this.files(root).get(path);
    const beforeText = existing?.kind === "text" ? existing.text : null;
    this.writeDiskFile(root, path, afterText);
    const change: TutorFileChange = {
      path,
      beforeText,
      afterText,
      createdAt: new Date().toISOString(),
    };
    return this.updateTutorSession(workspaceId, (session) => {
      const messages = session.messages.map((msg, idx) =>
        idx === session.messages.length - 1 && msg.role === "assistant"
          ? { ...msg, fileChanges: [...msg.fileChanges, change] }
          : msg
      );
      return { ...session, messages };
    });
  }

  emitTutorTurnCompleted(workspaceId: number) {
    return this.updateTutorSession(workspaceId, (session) => {
      const messages = session.messages.map((msg, idx) =>
        idx === session.messages.length - 1 && msg.role === "assistant"
          ? { ...msg, status: "completed" as const }
          : msg
      );
      return {
        ...session,
        status: "ready",
        activeTurnId: null,
        messages,
      };
    });
  }

  emitTutorCrash(workspaceId: number) {
    return this.emitTutorTurnFailed(workspaceId, { code: "ACP_PROCESS_EXITED" });
  }

  emitTutorTurnFailed(workspaceId: number, error: DomainError) {
    return this.updateTutorSession(workspaceId, (session) => {
      const messages = session.messages.map((msg, idx) =>
        idx === session.messages.length - 1 && msg.role === "assistant"
          ? { ...msg, status: "failed" as const, error }
          : msg
      );
      return {
        ...session,
        status: "error",
        processRunning: false,
        activeTurnId: null,
        lastError: error,
        canRetry: true,
        messages,
      };
    });
  }

  private fileSnapshot(root: string, path: string): WorkspaceFile {
    return this.files(root).get(path) ?? { kind: "missing", path, revision: null, sizeBytes: 0 };
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
        const nextTutor = patch.tutor
          ? {
              ...this.settings.tutor,
              ...(patch.tutor.preset !== undefined
                ? {
                    preset: patch.tutor.preset,
                    ...(patch.tutor.command === undefined && patch.tutor.preset !== "custom"
                      ? {
                          command:
                            ACP_AGENT_PRESETS.find((p) => p.id === patch.tutor?.preset)
                              ?.defaultCommand ?? this.settings.tutor.command,
                        }
                      : {}),
                  }
                : {}),
              ...(patch.tutor.command !== undefined
                ? { command: patch.tutor.command.trim() }
                : {}),
              ...(patch.tutor.permissionPolicy !== undefined
                ? { permissionPolicy: patch.tutor.permissionPolicy }
                : {}),
              ...(patch.tutor.teachingStyle !== undefined
                ? { teachingStyle: patch.tutor.teachingStyle }
                : {}),
            }
          : this.settings.tutor;
        const previousTeachingStyle = this.settings.tutor.teachingStyle;
        const agentChanged =
          nextTutor.preset !== this.settings.tutor.preset ||
          nextTutor.command !== this.settings.tutor.command;
        this.settings = {
          ...this.settings,
          ...(patch.theme !== undefined ? { theme: patch.theme } : {}),
          ...(patch.accent !== undefined ? { accent: patch.accent } : {}),
          ...(patch.codeFontSize !== undefined
            ? { codeFontSize: patch.codeFontSize }
            : {}),
          ...(patch.locale !== undefined ? { locale: patch.locale } : {}),
          ...(patch.preferredIde !== undefined
            ? { preferredIde: patch.preferredIde }
            : {}),
          tutor: nextTutor,
        };
        if (nextTutor.teachingStyle !== previousTeachingStyle) {
          this.recordMetric({
            detailKind: "aiModeSwitched",
            from: previousTeachingStyle,
            to: nextTutor.teachingStyle,
          });
        }
        if (this.workspace && this.tutorSessions.has(this.workspace.id)) {
          this.updateTutorSession(this.workspace.id, (session) => ({
            ...session,
            preset: nextTutor.preset,
            command: nextTutor.command,
            permissionPolicy: nextTutor.permissionPolicy,
            teachingStyle: nextTutor.teachingStyle,
            ...(agentChanged
              ? { processRunning: false, sessionReused: false, status: "idle", activeTurnId: null }
              : {}),
          }));
        }
        this.emitEvent({
          type: "settingsUpdated",
          payload: { ...this.settings, tutor: { ...this.settings.tutor } },
        });
        return {
          type: "settings",
          payload: { ...this.settings, tutor: { ...this.settings.tutor } },
        };
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
        this.publishTrust();
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
        this.publishTrust();
        return { type: "trustEntries", payload: [...this.trustEntries] };
      }
      case "listBuiltinCourses":
        return {
          type: "builtinCourses",
          payload: Array.from(this.builtinCourses.values()).map(summarizeCourse),
        };
      case "openCourse": {
        const { course, activeKey } = this.resolveCourse(command.payload);
        if (this.activeCourseKey?.key !== activeKey.key) this.workspace = null;
        this.activeCourseKey = activeKey;
        this.emitEvent({ type: "courseLoaded", payload: course });
        return { type: "loadedCourse", payload: course };
      }
      case "refreshGitCourse":
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
        const loaded = this.withTrust(course);
        this.emitEvent({ type: "courseLoaded", payload: loaded });
        return { type: "loadedCourse", payload: loaded };
      }
      case "closeActiveCourse": {
        this.activeCourseKey = null;
        this.workspace = null;
        return { type: "courseClosed" };
      }
      case "bindWorkspace":
        return { type: "workspaceBound", payload: this.bindWorkspace(command.payload.path) };
      case "openRecentWorkspace": {
        const recent = this.recents.find((item) => item.workspacePath === command.payload.path);
        if (!recent) throw new CoreCommandError({ code: "INVALID_WORKSPACE_PATH" });
        const { course, activeKey } = this.resolveCourse(recent.courseSource ?? { kind: "builtin", courseId: recent.courseId });
        this.activeCourseKey = activeKey;
        this.workspace = null;
        const workspace = course.trusted ? this.bindWorkspace(recent.workspacePath) : null;
        return { type: "recentWorkspaceOpened", payload: { course, workspace } };
      }
      case "listWorkspaceFiles": {
        const { workspaceId, path } = command.payload;
        const root = this.workspaceRoot(workspaceId);
        const prefix = path ? path + "/" : "";
        const entries = new Map<string, WorkspaceEntry>();
        for (const file of this.files(root).keys()) {
          if (!file.startsWith(prefix)) continue;
          const parts = file.slice(prefix.length).split("/");
          entries.set(parts[0], { path: prefix + parts[0], name: parts[0], kind: parts.length > 1 ? "directory" : "file" });
        }
        return { type: "workspaceEntries", payload: [...entries.values()] };
      }
      case "previewChapterSolution": {
        const { workspaceId, chapterId } = command.payload;
        const root = this.workspaceRoot(workspaceId);
        const files = this.options.chapterSolutions?.[chapterId] ?? {};
        return {
          type: "workspaceComparisons",
          payload: Object.entries(files).map(([path, proposed]) => ({
            current: this.fileSnapshot(root, path), proposed,
          })),
        };
      }
      case "listStarterFiles": {
        this.workspaceRoot(command.payload.workspaceId);
        return { type: "starterFiles", payload: Object.keys(this.options.starterFiles ?? {}).sort() };
      }
      case "previewStarterFile": {
        const { workspaceId, path } = command.payload;
        const root = this.workspaceRoot(workspaceId);
        const proposed = this.options.starterFiles?.[path];
        if (proposed === undefined) throw new CoreCommandError({ code: "WORKSPACE_FILE_NOT_FOUND", details: { path } });
        return { type: "workspaceComparisons", payload: [{ current: this.fileSnapshot(root, path), proposed }] };
      }
      case "syncLanguageDocument":
        return { type: "languageDocumentSynced" };
      case "requestLanguageFeature":
        return { type: "languageFeature", payload: null };
      case "readWorkspaceFile":
      case "writeWorkspaceFile":
      case "applyWorkspaceComparison": {
        const { workspaceId, path } = command.payload;
        const root = this.workspaceRoot(workspaceId);
        const file = this.fileSnapshot(root, path);
        if (command.type === "readWorkspaceFile") return { type: "workspaceFile", payload: file };
        if (file.revision !== command.payload.expectedRevision) return { type: "workspaceFileConflict", payload: file };
        return { type: "workspaceFile", payload: this.writeDiskFile(root, path, command.payload.text) };
      }
      case "createWorkspaceFile": {
        const { workspaceId, path } = command.payload;
        const root = this.workspaceRoot(workspaceId);
        if (this.files(root).has(path)) throw new CoreCommandError({ code: "WORKSPACE_FILE_EXISTS", details: { path } });
        return { type: "workspaceFile", payload: this.writeDiskFile(root, path, "") };
      }
      case "renameWorkspaceFile": {
        const { workspaceId, path, newPath } = command.payload;
        const root = this.workspaceRoot(workspaceId);
        const file = this.files(root).get(path);
        if (!file || file.kind !== "text") throw new CoreCommandError({ code: "WORKSPACE_FILE_NOT_FOUND", details: { path } });
        this.files(root).delete(path);
        return { type: "workspaceFile", payload: this.writeDiskFile(root, newPath, file.text) };
      }
      case "openWorkspaceIde":
        this.workspaceRoot(command.payload.workspaceId);
        return { type: "workspaceIdeOpened" };
      case "openTerminal": {
        const { workspaceId } = command.payload;
        return { type: "terminalOpened", payload: this.addTerminal(workspaceId, this.workspaceRoot(workspaceId)) };
      }
      case "getCommandCwdOverrides":
      case "setCommandCwdOverride": {
        const root = this.workspaceRoot(command.payload.workspaceId);
        const overrides = { ...this.cwdOverrides.get(root) };
        if (command.type === "setCommandCwdOverride") {
          const { commandId, cwd } = command.payload;
          if (cwd === null) delete overrides[commandId];
          else overrides[commandId] = cwd;
          this.cwdOverrides.set(root, overrides);
        }
        return { type: "commandCwdOverrides", payload: overrides };
      }
      case "runCommandBlock": {
        const { workspaceId, commandId, confirmation, drafts } = command.payload;
        const root = this.workspaceRoot(workspaceId);
        const block = this.activeCourse()?.chapters.flatMap(({ commands }) => commands).find(({ id }) => id === commandId);
        if (!block) throw new CoreCommandError({ code: "COMMAND_BLOCK_NOT_FOUND" });
        const dir = this.cwdOverrides.get(root)?.[commandId] ?? block.cwd;
        const cwd = dir && dir !== "." ? `${root}/${dir}` : root;
        if (block.command === "printf test | sh" && (!confirmation || this.pendingConfirmation !== confirmation)) {
          const token = this.nextTerminalId++;
          this.pendingConfirmation = token;
          return { type: "commandConfirmationRequired", payload: { token, cwd, command: block.command } };
        }
        this.pendingConfirmation = null;
        if (confirmation) {
          this.recordMetric({
            detailKind: "cmdRiskModalAction",
            source: "commandBlock",
            decision: "confirm",
          });
        }
        this.recordMetric({
          detailKind: "cmdBlockClicked",
          mode: block.mode,
          isRestart: Boolean(command.payload.restartTerminalId),
        });
        for (const draft of drafts) {
          const file = this.files(root).get(draft.path) ?? { kind: "missing" as const, path: draft.path, revision: null, sizeBytes: 0 };
          if (file.revision !== draft.expectedRevision) return { type: "commandSaveConflict", payload: { file, saved: [] } };
        }
        const saved = drafts.map((draft) => this.writeDiskFile(root, draft.path, draft.text));
        return { type: "commandStarted", payload: { terminal: this.addTerminal(workspaceId, cwd, block), saved } };
      }
      case "readTerminals": {
        const { workspaceId, terminalId, cursor = 0, includeRecentErrors } = command.payload;
        this.workspaceRoot(workspaceId);
        const bytes = terminalId === undefined ? null : new TextEncoder().encode(this.terminalOutput.get(terminalId) ?? "");
        return { type: "terminals", payload: {
          terminals: this.terminals.filter((item) => item.workspaceId === workspaceId),
          output: bytes === null ? null : {
            cursor: bytes.length, reset: false,
            data: btoa(Array.from(bytes.slice(cursor), (byte) => String.fromCharCode(byte)).join("")),
            more: false,
          },
          recentErrors: includeRecentErrors ? this.terminalErrors : [],
        } };
      }
      case "closeTerminal":
        this.terminals = this.terminals.filter((item) => item.id !== command.payload.terminalId);
        return { type: "terminalUpdated" };
      case "stopTerminal": {
        const target = this.terminals.find((item) => item.id === command.payload.terminalId);
        this.terminals = this.terminals.map((item) => item.id === command.payload.terminalId
          ? { ...item, running: false, exitCode: 130 }
          : item);
        if (target?.running) {
          this.recordMetric({
            detailKind: "cmdFinished",
            mode: target.mode,
            exitCode: 130,
            stopped: true,
            durationMs: 0,
          });
        }
        return { type: "terminalUpdated" };
      }
      case "resizeTerminal":
      case "writeTerminal":
        return { type: "terminalUpdated" };
      case "getProgress": {
        const progress = this.progress ?? {
          workspaceId: command.payload.workspaceId,
          completedExercises: 0,
          totalExercises: this.activeCourse()?.totalExercises ?? 10,
          runningChapterId: null,
          chapters: {},
        };
        this.progress = progress;
        return { type: "progress", payload: progress };
      }
      case "resetProgress": {
        const root = this.workspaceRoot(command.payload.workspaceId);
        const totalExercises = this.activeCourse()?.totalExercises ?? this.progress?.totalExercises ?? 10;
        const reset: import("../core/contracts").LearningProgress = {
          workspaceId: command.payload.workspaceId,
          completedExercises: 0,
          totalExercises,
          runningChapterId: null,
          chapters: {},
        };
        this.progress = reset;
        this.recents = this.recents.map((item) =>
          item.workspacePath === root ? { ...item, completedExercises: 0 } : item
        );
        this.emitEvent({ type: "progressUpdated", payload: reset });
        this.emitEvent({ type: "recentsUpdated", payload: [...this.recents] });
        return { type: "progress", payload: reset };
      }
      case "setSelfCheck":
      case "completeExercise": {
        const progress = this.progress ?? {
          workspaceId: command.payload.workspaceId,
          completedExercises: 0,
          totalExercises: this.activeCourse()?.totalExercises ?? 10,
          runningChapterId: null,
          chapters: {},
        };
        this.progress = progress;
        return { type: "progress", payload: progress };
      }
      case "runChecks": {
        const { workspaceId, chapterId, drafts } = command.payload;
        const root = this.workspaceRoot(workspaceId);
        const chapter = this.activeCourse()?.chapters.find((c) => c.chapterId === chapterId);
        this.recordMetric({
          detailKind: "verifyChapterClicked",
          checkCount: chapter?.threeStep?.step3Checks.length ?? 0,
          requiresDaemon: Boolean(chapter?.threeStep?.requiresDaemon),
        });
        for (const draft of drafts) {
          const file = this.fileSnapshot(root, draft.path);
          if (file.revision !== draft.expectedRevision) return { type: "commandSaveConflict", payload: { file, saved: [] } };
        }
        const saved = drafts.map((draft) => this.writeDiskFile(root, draft.path, draft.text));
        const progress = this.progress ?? { workspaceId, completedExercises: 0, totalExercises: 10, runningChapterId: null, chapters: {} };
        this.progress = { ...progress, runningChapterId: chapterId };
        return { type: "verificationStarted", payload: { progress: this.progress, saved } };
      }
      case "exportLearningRecord": return { type: "learningRecord", payload: { progress: this.progress } };
      case "getPreflight":
        return { type: "preflight", payload: this.preflight };
      case "refreshPreflight": {
        const readyCount = this.preflight.languages.filter((row) => row.status === "ok").length;
        this.recordMetric({
          detailKind: "preflightExecuted",
          readyCount,
          totalLanguages: this.preflight.languages.length,
        });
        return { type: "preflight", payload: this.preflight };
      }
      case "setRuntimePreference":
      case "installManagedRuntime": {
        if (command.type === "installManagedRuntime") {
          this.recordMetric({
            detailKind: "runtimeInstalled",
            language: command.payload.language,
            succeeded: true,
          });
        }
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
      case "testTutorHandshake": {
        const active = this.activeCourse();
        if (active && !this.withTrust(active).trusted) {
          throw new CoreCommandError({ code: "COURSE_UNTRUSTED" });
        }
        if (!command.payload.command.trim()) {
          throw new CoreCommandError({ code: "ACP_COMMAND_INVALID" });
        }
        const presetLabel =
          ACP_AGENT_PRESETS.find((p) => p.id === command.payload.preset)?.label ??
          "Custom ACP Agent";
        return {
          type: "tutorHandshake",
          payload: this.tutorHandshakeOverride ?? {
            preset: command.payload.preset,
            command: command.payload.command.trim(),
            protocolVersion: "1",
            agentName: presetLabel,
            agentVersion: "1.0.0",
          },
        };
      }
      case "getTutorSession": {
        return {
          type: "tutorSession",
          payload: this.ensureTutorSession(command.payload.workspaceId),
        };
      }
      case "buildTutorContext": {
        return {
          type: "tutorContext",
          payload: this.buildTutorContextSnapshot(command.payload),
        };
      }
      case "sendTutorPrompt": {
        const prompt = command.payload.prompt.trim();
        if (!prompt) throw new CoreCommandError({ code: "TUTOR_PROMPT_EMPTY" });
        const session = this.ensureTutorSession(command.payload.workspaceId);
        if (session.activeTurnId) throw new CoreCommandError({ code: "TUTOR_BUSY" });
        const turnId = `turn-${this.tutorSeq++}`;
        const style = command.payload.teachingStyle ?? this.settings.tutor.teachingStyle;
        const resolvedContext = command.payload.context
          ? this.buildTutorContextSnapshot({
              ...command.payload.context,
              workspaceId: command.payload.workspaceId,
              prompt,
              teachingStyle: style,
            })
          : null;
        const now = new Date().toISOString();
        const userMsg: TutorMessage = {
          id: `msg-${this.tutorSeq++}`,
          turnId,
          role: "user",
          content: prompt,
          teachingStyle: style,
          status: "completed",
          error: null,
          quote: resolvedContext?.quote ?? null,
          contextDimensions: resolvedContext?.chips.map((c) => c.dimension) ?? [],
          permissions: [],
          fileChanges: [],
          createdAt: now,
        };
        const assistantMsg: TutorMessage = {
          id: `msg-${this.tutorSeq++}`,
          turnId,
          role: "assistant",
          content: "",
          teachingStyle: style,
          status: "streaming",
          error: null,
          permissions: [],
          fileChanges: [],
          createdAt: now,
        };
        const presetLabel =
          ACP_AGENT_PRESETS.find((p) => p.id === this.settings.tutor.preset)?.label ??
          "Custom ACP Agent";
        const next = this.updateTutorSession(command.payload.workspaceId, (current) => ({
          ...current,
          status: "streaming",
          processRunning: true,
          sessionReused: current.processRunning,
          agentName: current.agentName ?? presetLabel,
          teachingStyle: style,
          activeTurnId: turnId,
          lastError: null,
          canRetry: false,
          messages: [...current.messages, userMsg, assistantMsg],
        }));
        return { type: "tutorSession", payload: next };
      }
      case "stopTutorTurn": {
        const next = this.updateTutorSession(command.payload.workspaceId, (current) => {
          const turnId = current.activeTurnId;
          const now = new Date().toISOString();
          const messages = current.messages.map((msg) =>
            msg.turnId === turnId && msg.role === "assistant"
              ? {
                  ...msg,
                  status: "cancelled" as const,
                  permissions: msg.permissions.map((p) =>
                    p.status === "pending"
                      ? { ...p, status: "cancelled" as const, resolvedAt: now }
                      : p
                  ),
                }
              : msg
          );
          return {
            ...current,
            status: current.processRunning ? "ready" : "idle",
            activeTurnId: null,
            messages,
          };
        });
        return { type: "tutorSession", payload: next };
      }
      case "retryTutorTurn": {
        const current = this.ensureTutorSession(command.payload.workspaceId);
        const lastUser = [...current.messages].reverse().find((m) => m.role === "user");
        if (!lastUser) throw new CoreCommandError({ code: "TUTOR_PROMPT_EMPTY" });
        return this.dispatch({
          type: "sendTutorPrompt",
          payload: {
            workspaceId: command.payload.workspaceId,
            prompt: lastUser.content,
            teachingStyle: this.settings.tutor.teachingStyle,
          },
        });
      }
      case "respondTutorPermission": {
        const { workspaceId, requestId, allow } = command.payload;
        const now = new Date().toISOString();
        const next = this.updateTutorSession(workspaceId, (current) => {
          let found = false;
          const messages = current.messages.map((msg) => ({
            ...msg,
            permissions: msg.permissions.map((perm) => {
              if (perm.id !== requestId || perm.status !== "pending") return perm;
              found = true;
              return {
                ...perm,
                status: allow ? ("approved" as const) : ("denied" as const),
                resolvedAt: now,
              };
            }),
          }));
          if (!found) {
            throw new CoreCommandError({ code: "TUTOR_PERMISSION_NOT_FOUND" });
          }
          const hasPending = messages.some((m) =>
            m.permissions.some((p) => p.status === "pending")
          );
          return {
            ...current,
            status: hasPending ? "awaitingPermission" : "streaming",
            messages,
          };
        });
        return { type: "tutorSession", payload: next };
      }
      case "clearTutorSession": {
        const next = this.updateTutorSession(command.payload.workspaceId, (current) => ({
          ...current,
          status: "idle",
          processRunning: false,
          sessionReused: false,
          activeTurnId: null,
          lastError: null,
          canRetry: false,
          messages: [],
        }));
        return { type: "tutorSession", payload: next };
      }
      case "getWorkspaceMetrics": {
        const root = this.workspaceRoot(command.payload.workspaceId);
        const metrics = this.metricsForRoot(root);
        return {
          type: "workspaceMetrics",
          payload: {
            courseOpened: metrics.courseOpened,
            counts: { ...metrics.counts },
            events: [...metrics.events],
          },
        };
      }
      case "recordLocalMetric": {
        const root = this.workspaceRoot(command.payload.workspaceId);
        this.pushMetricRecord(
          root,
          this.makeMetricRecord(this.localEventToDetail(command.payload.event))
        );
        const metrics = this.metricsForRoot(root);
        return {
          type: "workspaceMetrics",
          payload: {
            courseOpened: metrics.courseOpened,
            counts: { ...metrics.counts },
            events: [...metrics.events],
          },
        };
      }
      case "createCourseDraft":
      case "openCourseDraft":
      case "readCourseDraftFile":
      case "writeCourseDraftFile":
      case "previewCourseDraft":
      case "startTrialWorkspace":
        throw new CoreCommandError({ code: "NO_ACTIVE_DRAFT" });
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

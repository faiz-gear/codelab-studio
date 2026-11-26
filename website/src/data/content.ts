export type Locale = "zh-CN" | "en-US";
export type PlatformId = "macos" | "windows" | "linux";
export type CourseId = "go" | "ts" | "python" | "java";

export const REPO_URL = "https://github.com/faiz-gear/codelab-studio";
export const RELEASE_VERSION = "v0.1.0";
export const RELEASE_TAG_URL = `${REPO_URL}/releases/tag/${RELEASE_VERSION}`;
export const CHECKSUMS_URL = `${REPO_URL}/releases/download/${RELEASE_VERSION}/SHA256SUMS.txt`;

export interface InstallerAsset {
  id: PlatformId;
  fileName: string;
  sizeLabel: string;
  downloadUrl: string;
  verifyCmd: string;
}

export const INSTALLERS: Record<PlatformId, InstallerAsset> = {
  macos: {
    id: "macos",
    fileName: "CodeLab-Studio-0.1.0-macos-universal.dmg",
    sizeLabel: "16.2 MB",
    downloadUrl: `${REPO_URL}/releases/download/${RELEASE_VERSION}/CodeLab-Studio-0.1.0-macos-universal.dmg`,
    verifyCmd: "shasum -a 256 CodeLab-Studio-0.1.0-macos-universal.dmg",
  },
  windows: {
    id: "windows",
    fileName: "CodeLab-Studio-0.1.0-windows-x64.exe",
    sizeLabel: "6.1 MB",
    downloadUrl: `${REPO_URL}/releases/download/${RELEASE_VERSION}/CodeLab-Studio-0.1.0-windows-x64.exe`,
    verifyCmd: "Get-FileHash .\\CodeLab-Studio-0.1.0-windows-x64.exe -Algorithm SHA256",
  },
  linux: {
    id: "linux",
    fileName: "CodeLab-Studio-0.1.0-linux-x64.AppImage",
    sizeLabel: "88.5 MB",
    downloadUrl: `${REPO_URL}/releases/download/${RELEASE_VERSION}/CodeLab-Studio-0.1.0-linux-x64.AppImage`,
    verifyCmd: "sha256sum CodeLab-Studio-0.1.0-linux-x64.AppImage",
  },
};

export const GO_EX1_RAW_MARKDOWN: Record<Locale, string> = {
  "zh-CN": `## 练习 1：让一个请求完成往返

先理解 HTTP 服务最小需要什么。在 \`task-api/main.go\` 中为健康检查路由加上 \`GET\` 方法前缀（Go 1.22+）：

\`\`\`go
mux.HandleFunc("GET /healthz", healthz)
\`\`\`

启动本地服务：

\`\`\`bash
go run .
\`\`\`

在另一个终端逐条验证：

\`\`\`bash
curl -i http://127.0.0.1:8080/healthz
curl -i -X POST http://127.0.0.1:8080/healthz
curl -i http://127.0.0.1:8080/missing
\`\`\`

**验收：**
- GET /healthz 返回 200 和 {"status":"ok"}
- POST /healthz 返回 405 Method Not Allowed
- GET /missing 返回 404 Not Found`,
  "en-US": `## Exercise 1: Complete one HTTP round-trip

See the smallest piece an HTTP server needs. In \`task-api/main.go\`, prefix the health route with \`GET\` (Go 1.22+):

\`\`\`go
mux.HandleFunc("GET /healthz", healthz)
\`\`\`

Start the local server:

\`\`\`bash
go run .
\`\`\`

In a second terminal, verify each request:

\`\`\`bash
curl -i http://127.0.0.1:8080/healthz
curl -i -X POST http://127.0.0.1:8080/healthz
curl -i http://127.0.0.1:8080/missing
\`\`\`

**Checks:**
- GET /healthz returns 200 with {"status":"ok"}
- POST /healthz returns 405 Method Not Allowed
- GET /missing returns 404 Not Found`,
};

export const GO_EX1_STARTER_CODE: Record<Locale, string> = {
  "zh-CN": `package main

import (
\t"encoding/json"
\t"net/http"
)

func healthz(w http.ResponseWriter, _ *http.Request) {
\tw.Header().Set("Content-Type", "application/json")
\t_ = json.NewEncoder(w).Encode(map[string]string{
\t\t"status": "ok",
\t})
}

func routes() *http.ServeMux {
\tmux := http.NewServeMux()
\t// TODO: 给 /healthz 补上 GET 方法前缀
\tmux.HandleFunc("/healthz", healthz)
\treturn mux
}`,
  "en-US": `package main

import (
\t"encoding/json"
\t"net/http"
)

func healthz(w http.ResponseWriter, _ *http.Request) {
\tw.Header().Set("Content-Type", "application/json")
\t_ = json.NewEncoder(w).Encode(map[string]string{
\t\t"status": "ok",
\t})
}

func routes() *http.ServeMux {
\tmux := http.NewServeMux()
\t// TODO: prefix /healthz with GET method
\tmux.HandleFunc("/healthz", healthz)
\treturn mux
}`,
};

export const GO_EX1_SOLUTION_CODE: Record<Locale, string> = {
  "zh-CN": `package main

import (
\t"encoding/json"
\t"net/http"
)

func healthz(w http.ResponseWriter, _ *http.Request) {
\tw.Header().Set("Content-Type", "application/json")
\t_ = json.NewEncoder(w).Encode(map[string]string{
\t\t"status": "ok",
\t})
}

func routes() *http.ServeMux {
\tmux := http.NewServeMux()
\t// 已补全 Go 1.22+ 方法路由前缀
\tmux.HandleFunc("GET /healthz", healthz)
\treturn mux
}`,
  "en-US": `package main

import (
\t"encoding/json"
\t"net/http"
)

func healthz(w http.ResponseWriter, _ *http.Request) {
\tw.Header().Set("Content-Type", "application/json")
\t_ = json.NewEncoder(w).Encode(map[string]string{
\t\t"status": "ok",
\t})
}

func routes() *http.ServeMux {
\tmux := http.NewServeMux()
\t// Added Go 1.22+ method route prefix
\tmux.HandleFunc("GET /healthz", healthz)
\treturn mux
}`,
};

export interface CourseItem {
  id: CourseId;
  badge: string;
  exerciseCount: number;
  runtime: string;
  runner: Record<Locale, string>;
  entryFile: string;
  accent: "teal" | "yellow" | "pink";
  title: Record<Locale, string>;
  summary: Record<Locale, string>;
  chapters: Record<Locale, string[]>;
  snippet: string;
}

export const BUILTIN_COURSES: CourseItem[] = [
  {
    id: "go",
    badge: "Go",
    exerciseCount: 10,
    runtime: "Go 1.23.12",
    runner: {
      "zh-CN": "go test -json -race + HTTP 断言",
      "en-US": "go test -json -race + HTTP assertions",
    },
    entryFile: "task-api/main.go",
    accent: "teal",
    title: {
      "zh-CN": "Go 与 GoFrame 开发实践手册",
      "en-US": "Go & GoFrame Engineering Handbook",
    },
    summary: {
      "zh-CN": "围绕同一个任务管理 API，从标准库 net/http、并发锁、SQLite 持久化到迁移 GoFrame ORM，共 10 个递进实战练习。",
      "en-US": "Build one Task API from Go stdlib net/http, concurrency, and SQLite persistence through a full GoFrame migration across 10 exercises.",
    },
    chapters: {
      "zh-CN": [
        "练习 0：写清楚系统的规则",
        "练习 1：让一个请求完成往返",
        "练习 2：在内存中实现任务管理",
        "练习 3：把业务规则从 HTTP 中提取出来",
        "练习 4：让状态跨越进程重启",
        "练习 5：让一次动作中的多次写入保持一致",
        "练习 6：让请求停止时，工作也能停止",
        "练习 7：迁移到 GoFrame HTTP 层",
        "练习 8：迁移到 GoFrame ORM",
        "练习 9：端到端行为一致性回归验收",
      ],
      "en-US": [
        "Ex 0: Define system rules & API contract",
        "Ex 1: Complete one HTTP round-trip",
        "Ex 2: In-memory task store",
        "Ex 3: Extract domain rules from HTTP",
        "Ex 4: Persist state across restarts",
        "Ex 5: Keep multi-write transactions atomic",
        "Ex 6: Propagate context cancellation",
        "Ex 7: Migrate HTTP layer to GoFrame",
        "Ex 8: Migrate storage to GoFrame ORM",
        "Ex 9: End-to-end parity verification",
      ],
    },
    snippet: `mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, _ *http.Request) {
    w.Header().Set("Content-Type", "application/json; charset=utf-8")
    _ = json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
})`,
  },
  {
    id: "ts",
    badge: "TypeScript",
    exerciseCount: 8,
    runtime: "Bun 1.3.13 / Node 22+",
    runner: {
      "zh-CN": "bun test / Node 原生 TS",
      "en-US": "bun test / native Node TS",
    },
    entryFile: "src/pipeline.ts",
    accent: "yellow",
    title: {
      "zh-CN": "TypeScript 数据流水线实战",
      "en-US": "TypeScript Data Pipeline Practice",
    },
    summary: {
      "zh-CN": "多文件函数判题：从不可变过滤、字段归一化、去重聚合到确定性游标分页、Promise.allSettled 容错与有界并发调度。",
      "en-US": "Multi-file function grading: immutable filtering, normalization, deduplication, cursor pagination, Promise.allSettled, and bounded concurrency.",
    },
    chapters: {
      "zh-CN": [
        "练习 1：成年用户筛选与不可变映射",
        "练习 2：原始事件清洗与字段归一化",
        "练习 3：按 ID 去重并保留最新事件",
        "练习 4：按类目分组汇总金额与峰值延迟",
        "练习 5：确定性游标分页器",
        "练习 6：Promise.allSettled 容错批量收集",
        "练习 7：有界并发任务调度器",
        "练习 8：组装端到端异步数据流水线",
      ],
      "en-US": [
        "Ex 1: Adult user filtering & immutable mapping",
        "Ex 2: Raw event cleaning & field normalization",
        "Ex 3: Deduplicate by ID keeping latest event",
        "Ex 4: Category aggregation & peak latency",
        "Ex 5: Deterministic cursor pagination",
        "Ex 6: Fault-tolerant Promise.allSettled batch",
        "Ex 7: Bounded concurrency scheduler",
        "Ex 8: Assemble end-to-end async pipeline",
      ],
    },
    snippet: `export async function runWithConcurrency<T, R>(
  items: readonly T[],
  limit: number,
  worker: (item: T) => Promise<R>
): Promise<R[]>`,
  },
  {
    id: "python",
    badge: "Python",
    exerciseCount: 6,
    runtime: "uv 0.11.6 + Python 3.12.13",
    runner: {
      "zh-CN": "pytest --json-report",
      "en-US": "pytest --json-report",
    },
    entryFile: "src/cleaner.py",
    accent: "teal",
    title: {
      "zh-CN": "Python 数据清洗与订单统计",
      "en-US": "Python Data Cleaning & Order Analytics",
    },
    summary: {
      "zh-CN": "通过 uv 隔离 Python 3.12 与 pytest 环境，依次完成无效订单过滤、dataclass CSV 解析、去重、Top-N 排行与异常检测。",
      "en-US": "Isolated Python 3.12 & pytest via uv: clean malformed orders, parse CSV with dataclasses, deduplicate, compute Top-N categories, and flag anomalies.",
    },
    chapters: {
      "zh-CN": [
        "练习 1：清洗无效订单并按类目汇总金额",
        "练习 2：使用 dataclass 解析与校验订单 CSV",
        "练习 3：按订单号去重并保留最新状态",
        "练习 4：计算消费额 Top-N 类目排行",
        "练习 5：检测异常金额与重复订单",
        "练习 6：组装端到端订单统计报表",
      ],
      "en-US": [
        "Ex 1: Filter invalid orders & sum by category",
        "Ex 2: Parse & validate CSV with dataclasses",
        "Ex 3: Deduplicate orders keeping latest state",
        "Ex 4: Compute Top-N spending categories",
        "Ex 5: Detect anomalous amounts & duplicates",
        "Ex 6: Build end-to-end order summary report",
      ],
    },
    snippet: `@dataclass(frozen=True)
class OrderRecord:
    order_id: str
    category: str
    amount_cents: int`,
  },
  {
    id: "java",
    badge: "Java 21",
    exerciseCount: 7,
    runtime: "Temurin JDK 21 + JUnit 5",
    runner: {
      "zh-CN": "javac + JUnit 5 Console",
      "en-US": "javac + JUnit 5 Console",
    },
    entryFile: "src/main/java/.../TokenBucket.java",
    accent: "pink",
    title: {
      "zh-CN": "Java 21 并发限流器实战",
      "en-US": "Java 21 Concurrent Rate Limiter",
    },
    summary: {
      "zh-CN": "使用 Temurin JDK 21 与 JUnit 5 逐步手写线程安全的令牌桶、固定窗口、滑动日志、租户隔离与虚拟线程压测器。",
      "en-US": "Build thread-safe rate limiters with Temurin JDK 21 & JUnit 5: token bucket, fixed window, sliding log, tenant isolation, and virtual-thread load runner.",
    },
    chapters: {
      "zh-CN": [
        "练习 1：线程安全的固定令牌桶",
        "练习 2：按时间均匀补充的令牌桶",
        "练习 3：固定时间窗口计数器",
        "练习 4：滑动时间窗口日志限流器",
        "练习 5：按租户 Key 隔离的并发限流器",
        "练习 6：Java 21 虚拟线程并发压测器",
        "练习 7：突发与持续双层组合限流器",
      ],
      "en-US": [
        "Ex 1: Thread-safe fixed token bucket",
        "Ex 2: Time-refilling token bucket",
        "Ex 3: Fixed time-window counter",
        "Ex 4: Sliding window log limiter",
        "Ex 5: Per-tenant concurrent rate limiter",
        "Ex 6: Java 21 virtual-thread load runner",
        "Ex 7: Composite burst & sustained limiter",
      ],
    },
    snippet: `public final class TokenBucket {
    private long tokens;
    public synchronized boolean tryAcquire() { ... }
}`,
  },
];

export const COPY = {
  "zh-CN": {
    nav: {
      tryIt: "互动试玩",
      markdown: "导入 Markdown",
      courses: "四门实战课",
      tutor: "本机 AI 助教",
      download: "下载 v0.1.0",
      github: "GitHub 仓库",
      downloadShort: "下载 v0.1.0",
      langSwitch: "EN",
      builtInLabel: "内置 4 套工程课 · 共 31 题",
      sidebarNoteTitle: "离线可用 · 免登录",
      sidebarNoteBody: "零账号、零订阅。文档、编辑器、终端与验收全程本地运行，AI 助教按需接入你本机 CLI。",
    },
    hero: {
      titleLine1: "别光看文档，",
      titleHighlight: "直接跑通",
      titleLine2Rest: "一门新语言。",
      subtitle:
        "AI 时代想快速上手 Go、TypeScript、Python 或 Java？别再在博客网页、IDE 和终端之间来回切窗口。把任意静态 Markdown 教程放进 CodeLab Studio，一窗读手册、写代码、跑真实测试验收。",
      ctaSecondary: "先改一行代码试试",
      downloadFor: {
        macos: "下载 macOS 通用版 v0.1.0",
        windows: "下载 Windows x64 版 v0.1.0",
        linux: "下载 Linux AppImage v0.1.0",
      },
      metaLine: "v0.1.0 公开预览 · macOS / Windows / Linux · 免费下载 · 离线可用",
      staticCardTag: "静态 README.md",
      staticCardLine1: "## 练习 1：让一个请求完成往返",
      staticCardLine2: "```bash  go run .  ```",
      liveCardHeader: "task-api/main.go · 交互工作台",
      liveTogglePass: "3/3 已通过",
      liveToggleFail: "2/3 点我验收",
      marginCalloutPass: "补上 GET 前缀，POST 405 立刻变绿！",
      marginCalloutFail: "点击右上角试跑，看路由补全后立刻变绿",
      steps: [
        {
          n: "1",
          title: "打开任意 Markdown",
          desc: "拖入单文件 .md、Git 仓库或 4 套内置工程课，自动拆出章节与可执行命令块。",
        },
        {
          n: "2",
          title: "同窗改代码跑终端",
          desc: "常驻服务与单次测试分轨运行；缺环境时隔离安装托管运行时，不碰系统 PATH。",
        },
        {
          n: "3",
          title: "真实测试框架验收",
          desc: "直接跑 go test、bun test、pytest、JUnit 5，输出清晰预期/实际 Diff。",
        },
      ],
    },
    playground: {
      heading: "点一下，看静态 Markdown 怎么变成活工作台",
      lead: "下面复刻了内置 Go 课程的「练习 1」。先切到静态原文感受日常切窗口的断点，再切回工作台亲手改一行代码，点击运行验收。",
      previewBadge: "浏览器轻量试玩 · 真实桌面端运行本机 go test",
      tabStatic: "静态 README.md 原文",
      tabLive: "交互工作台",
      staticPainLabel: "以前读博客 / GitHub 教程：",
      staticPainText: "复制命令 → 切终端 → 手建目录 → 切 IDE 改代码 → 切回浏览器核对预期状态码……一题切 20+ 次窗口。",
      transformBtn: "一键转为交互工作台",
      step1Title: "启动本地服务",
      step1Sub: "task-api · 常驻 daemon（保存源码后验收前自动热重启）",
      step2Title: "修改 task-api/main.go",
      step2Sub: "把路由 \"/healthz\" 改为 Go 1.22+ 方法前缀 \"GET /healthz\"，或点击查看参考 Diff。",
      step3Title: "运行无头验收",
      runBtn: "运行验收",
      runningBtn: "验收中...",
      diffBtn: "查看参考 Diff",
      closeDiffBtn: "收起 Diff",
      applyDiffBtn: "确认合并参考代码",
      resetBtn: "重置初始代码",
      stickyDoneBadge: "已验收 · 完成态已粘性保留",
      expectedLabel: "预期",
      actualLabel: "实际",
      diffBeforeTitle: "当前 Workspace · main.go",
      diffAfterTitle: "Reference Solution · 参考版本",
      hintAfterPass: "3/3 全部通过！在真实桌面端，你的代码改动会保存在你选定的本地 Workspace 目录，进度写入 .codelab/progress.json。",
      hintBeforePass: "当前第 2 项未通过：未限定 GET 前缀时 POST 也会返回 200。试着把右侧第 18 行改成 \"GET /healthz\" 再点「运行验收」。",
      checks: [
        { id: "v1", label: "GET /healthz 返回 200 {\"status\":\"ok\"}", ms: "4ms" },
        {
          id: "v2",
          label: "POST /healthz 返回 405 Method Not Allowed",
          ms: "3ms",
          expected: "405 Method Not Allowed",
          actualFail: "200 OK（路由未限定 GET 方法前缀）",
        },
        { id: "v3", label: "GET /missing 返回 404 Not Found", ms: "2ms" },
      ],
    },
    markdownSection: {
      heading: "你收藏的博客与 GitHub 仓库，不用改造直接打开",
      lead: "不只局限于内置课程。任何按 H2 分章节的标准 Markdown 教程，放进桌面端就能解析出章节导航、命令块与独立工作区。",
      tabWorkbench: "真实桌面端 · 双栏工作台",
      tabWelcome: "真实桌面端 · 欢迎首页",
      captionWorkbench: "截取自 CodeLab Studio 最新桌面界面：左侧三步引导流 + 右侧 Monaco 编辑器与常驻服务控制",
      captionWelcome: "截取自 CodeLab Studio 最新桌面界面：单一清晰主入口、4 套语言课程与最近 Workspace",
      calloutNote: "左侧读步骤，右侧写代码，同一窗口完成",
      pillars: [
        {
          icon: "folder" as const,
          title: "只读 Course 与可写 Workspace 彻底分离",
          body: "教程源永远只读，你手写的代码留在你自己选定的本地目录。一门课可以反复开多个 Workspace 从头重练；绑定非空目录时绝不覆盖已有文件。",
        },
        {
          icon: "shield" as const,
          title: "三层执行护栏，绝不替你盲目跑命令",
          body: "外部目录与 Git 仓库先以只读预览打开，显式授信后才能执行；路径读写受 cap-std 规范边界限制；rm -rf / sudo / 管道执行等高危 Shell 语法执行前必经 AST 检测弹窗确认。",
        },
        {
          icon: "code" as const,
          title: "和 VS Code / GoLand / Cursor 无缝并存",
          body: "工作区就是你硬盘上的真实文件夹。外部 IDE 修改未改动文件时自动静默重载；若两边同时有草稿，清晰提示你「加载磁盘版」或「保留草稿」。",
        },
      ],
    },
    coursesSection: {
      heading: "4 套内置工程课，缺环境就隔离装好，不污染系统",
      lead: "每套课程都附带真实 Starter 工程、可见测试文件与 Reference Solution Diff，覆盖从语法直觉到真实并发与数据流水线。",
      exercisesUnit: "题",
      runtimeLabel: "托管运行时",
      runnerLabel: "真实测试适配器",
      entryLabel: "入口文件",
      syllabusHeading: "完整练习目录",
      preflightTitle: "环境体检 & 托管运行时隔离机制",
      preflightBody:
        "启动前并行探测本机工具链；如果系统没装 Go / Bun / Python / JDK 21 或版本过低，一键按需下载并通过 SHA-256 校验解压到独立 runtimes/ 目录——绝不修改系统全局 PATH，学完想删就删。",
      runtimePills: [
        "Go 1.23.12",
        "Bun 1.3.13",
        "uv 0.11.6 + Python 3.12.13",
        "Temurin JDK 21 + JUnit 5",
      ],
    },
    tutorSection: {
      heading: "连上你本机的 AI 编程 Agent，学新语言也能引导思考",
      lead: "如果你本机已经装了 Claude Code、Gemini CLI、Codex CLI 或 OpenCode，CodeLab Studio 通过开放的 ACP（stdio JSON-RPC 2.0）协议直接复用——不用再折腾 API Key；没装任何 AI 也完全可离线使用。",
      agentsLabel: "内置本机 ACP 预设",
      contextTitle: "可检视、可删减的 Context Snapshot",
      contextSub: "每次提问附带的上下文完全透明，硬上限 6,000 tokens，自动过滤 .env / *secret* / *key* 敏感文件。点击任意胶囊可移除：",
      chips: [
        { id: "chapter", label: "练习 1 章节摘要", tokens: 320 },
        { id: "file", label: "task-api/main.go (L14-20)", tokens: 410 },
        { id: "err", label: "验收失败 Diff (v2: 405 vs 200)", tokens: 260 },
        { id: "term", label: "终端日志尾 (go run .)", tokens: 190 },
      ],
      restoreChips: "恢复全部上下文",
      filterNote: "已自动过滤 .env · *secret* · *key*",
      modeSocratic: "引导思考（默认）",
      modeDirect: "直接解答",
      socraticReply:
        "你注册的是 \"/healthz\"，在 Go 1.22+ 的 ServeMux 里不写方法名会匹配所有 HTTP 方法。先想想：怎么在模式字符串里只允许 GET，让 POST 自动触发 405？",
      directReply:
        "把第 18 行 mux.HandleFunc(\"/healthz\", healthz) 改为 mux.HandleFunc(\"GET /healthz\", healthz)。Go 1.22 起 ServeMux 原生支持「方法 + 路径」模式，其余方法会自动返回 405 Method Not Allowed。",
      permTitle: "ACP 权限审批 · guided_readonly",
      permDesc: "默认自动放行 Workspace 内非敏感只读；涉及写入文件、执行终端命令或越界读取时，必须经你逐次批准。",
      permRequest: "Claude Code 请求修改 task-api/main.go（第 18 行）",
      permAllow: "允许本次写入",
      permDeny: "拒绝",
      permAllowedState: "已放行本次写入（仅限当前文件，随时可切换为 strict_ask_all 逐项审批）",
      permDeniedState: "已拒绝写入，你可以继续自己手写或在参考 Diff 中对照修改。",
      permReset: "重演审批",
    },
    downloadSection: {
      heading: "下载 CodeLab Studio v0.1.0 公开预览版",
      lead: "所有桌面安装包均由 GitHub Actions 自动构建并直接发布在 GitHub Releases，附带 SHA-256 校验清单。",
      detectedTag: "推荐",
      downloadCta: "直接下载安装包",
      allReleasesLink: "前往 GitHub Releases 查看全部安装包与详细安装说明",
      checksumsLink: "查看 SHA256SUMS.txt",
      copyCmd: "复制校验命令",
      copiedCmd: "已复制",
      firstLaunchTitle: "公开预览版首次打开说明（诚实透明）",
      platforms: {
        macos: {
          name: "macOS（Apple Silicon & Intel 通用）",
          short: "macOS Universal",
          noteTitle: "macOS 尚未经过 Apple 公证",
          noteBody:
            "打开 DMG 将 CodeLab Studio 拖入「应用程序」文件夹。由于 v0.1.0 公开预览版采用 ad-hoc 签名、尚未完成 Apple 公证，首次打开时 macOS 可能会弹出 Gatekeeper 安全提示，请在「系统设置 → 隐私与安全性」中选择「仍要打开」。",
        },
        windows: {
          name: "Windows x64（NSIS 安装包）",
          short: "Windows x64",
          noteTitle: "Windows 安装包尚未代码签名",
          noteBody:
            "直接运行 .exe 安装程序。由于 v0.1.0 公开预览版尚未购买代码签名证书，Microsoft Defender SmartScreen 可能会显示安全提示，点击「更多信息 → 仍要运行」即可继续。",
        },
        linux: {
          name: "Linux x64（AppImage · Ubuntu 22.04 构建）",
          short: "Linux x64",
          noteTitle: "赋予可执行权限后直接运行",
          noteBody:
            "下载后执行 chmod +x CodeLab-Studio-0.1.0-linux-x64.AppImage 并运行。如新版 Ubuntu 提示缺少 libfuse2，可参阅 v0.1.0 GitHub Release 发行说明中的启动步骤。",
        },
      },
      authorTitle: "也在写技术博客或团队实战文档？",
      authorBody:
        "无需自建在线教学平台。沿用你的标准 Markdown，加上一份轻量 codelab.json 与 .codelab-checks.json 即可分发跨平台可验收课程包，工作台内置「创作课程包」与隔离的 Trial Workspace 试跑。",
      authorLink: "查看 Course Pack 目录规范",
    },
    footer: {
      tagline: "本地优先的桌面编程实践工作台 · 免登录 · 离线完整可用",
      docsZh: "简体中文文档",
      docsEn: "English README",
      releases: "v0.1.0 Releases",
      repo: "GitHub 仓库",
    },
  },
  "en-US": {
    nav: {
      tryIt: "Interactive Lab",
      markdown: "Any Markdown",
      courses: "4 Built-in Courses",
      tutor: "Local AI Tutor",
      download: "Download v0.1.0",
      github: "GitHub",
      downloadShort: "Download v0.1.0",
      langSwitch: "中文",
      builtInLabel: "4 Built-in Courses · 31 Exercises",
      sidebarNoteTitle: "Offline-complete · No login",
      sidebarNoteBody: "Zero accounts, zero subscriptions. Manuals, editor, terminals, and grading run completely locally; connect your local CLI agent only if you want.",
    },
    hero: {
      titleLine1: "Stop reading static docs.",
      titleHighlight: "Run your next",
      titleLine2Rest: " language.",
      subtitle:
        "Picking up Go, TypeScript, Python, or Java in the AI coding era? Stop bouncing between blog tabs, your IDE, and three terminals. Open any Markdown tutorial in CodeLab Studio to read, edit, run, and verify with real tests in one window.",
      ctaSecondary: "Try editing 1 line below",
      downloadFor: {
        macos: "Download macOS Universal v0.1.0",
        windows: "Download Windows x64 v0.1.0",
        linux: "Download Linux AppImage v0.1.0",
      },
      metaLine: "v0.1.0 Public Preview · macOS / Windows / Linux · Free download · Works offline",
      staticCardTag: "Static README.md",
      staticCardLine1: "## Exercise 1: One HTTP round-trip",
      staticCardLine2: "```bash  go run .  ```",
      liveCardHeader: "task-api/main.go · Live Workbench",
      liveTogglePass: "3/3 Passed",
      liveToggleFail: "2/3 Click to verify",
      marginCalloutPass: "Add GET prefix — POST 405 turns green!",
      marginCalloutFail: "Click top-right badge to flip 2/3 → 3/3",
      steps: [
        {
          n: "1",
          title: "Open any Markdown",
          desc: "Drop in a single .md file, a Git repo, or 4 built-in courses; chapters and Command Blocks parse automatically.",
        },
        {
          n: "2",
          title: "Edit & run in one window",
          desc: "Daemons and one-shot commands run on isolated tabs; missing runtimes install cleanly without touching your global PATH.",
        },
        {
          n: "3",
          title: "Verify with real test runners",
          desc: "Runs real go test, bun test, pytest, and JUnit 5 with structured Expected vs Actual diffs.",
        },
      ],
    },
    playground: {
      heading: "Flip a static Markdown tutorial into a live workbench",
      lead: "Below is a faithful browser preview of Exercise 1 from the built-in Go course. View the static README first, then switch to the live workbench, edit one line, and click Verify.",
      previewBadge: "Lightweight browser preview · Desktop runs real local go test",
      tabStatic: "Static README.md",
      tabLive: "Interactive Workbench",
      staticPainLabel: "The old static-tutorial loop:",
      staticPainText: "Copy command → jump to terminal → create folder → switch to IDE → jump back to browser to check expected status codes… 20+ switches per exercise.",
      transformBtn: "Turn into Interactive Workbench",
      step1Title: "Start local daemon",
      step1Sub: "task-api · Managed Daemon (restarts automatically when source edits land)",
      step2Title: "Edit task-api/main.go",
      step2Sub: "Prefix \"/healthz\" with Go 1.22+ \"GET /healthz\", or inspect the Reference Solution Diff.",
      step3Title: "Run headless verification",
      runBtn: "Run checks",
      runningBtn: "Verifying...",
      diffBtn: "View Reference Diff",
      closeDiffBtn: "Hide Diff",
      applyDiffBtn: "Apply Reference Solution",
      resetBtn: "Reset starter",
      stickyDoneBadge: "Passed · Sticky completion recorded",
      expectedLabel: "Expected",
      actualLabel: "Actual",
      diffBeforeTitle: "Your Workspace · main.go",
      diffAfterTitle: "Reference Solution",
      hintAfterPass: "All 3/3 checks passed! In the desktop app, edits stay in your local Workspace directory and progress persists atomically in .codelab/progress.json.",
      hintBeforePass: "Check 2 failed: without the GET prefix, POST also returns 200. Edit line 18 to \"GET /healthz\" and click Run checks.",
      checks: [
        { id: "v1", label: "GET /healthz returns 200 {\"status\":\"ok\"}", ms: "4ms" },
        {
          id: "v2",
          label: "POST /healthz returns 405 Method Not Allowed",
          ms: "3ms",
          expected: "405 Method Not Allowed",
          actualFail: "200 OK (route missing GET method prefix)",
        },
        { id: "v3", label: "GET /missing returns 404 Not Found", ms: "2ms" },
      ],
    },
    markdownSection: {
      heading: "Open your bookmarked blogs & GitHub tutorials with zero edits",
      lead: "Not locked to built-in content. Any standard H2-structured Markdown handbook opens directly as a navigable manual with executable Command Blocks.",
      tabWorkbench: "Real Desktop App · Two-Pane Workbench",
      tabWelcome: "Real Desktop App · Welcome Home",
      captionWorkbench: "Captured from the live CodeLab Studio desktop app: 3-step guided manual alongside Monaco editor and Managed Daemon controls",
      captionWelcome: "Captured from the live CodeLab Studio desktop app: focused single entry, 4 built-in Course Packs, and recent Workspaces",
      calloutNote: "Guided steps left, real editor right — one window",
      pillars: [
        {
          icon: "folder" as const,
          title: "Read-only Course, writable local Workspace",
          body: "Course sources stay strictly read-only while your edits live in any local folder you pick. Re-run the same course in fresh Workspaces; binding a non-empty folder never overwrites existing files.",
        },
        {
          icon: "shield" as const,
          title: "Three execution guardrails before any shell command runs",
          body: "External folders and Git repos open in read-only preview until trusted; cap-std enforces canonical path containment; tree-sitter-bash inspects shell ASTs to gate destructive patterns like rm -rf or sudo.",
        },
        {
          icon: "code" as const,
          title: "Works alongside VS Code, GoLand, or Cursor",
          body: "Your Workspace is a real folder on disk. Clean buffers reload automatically after external edits; unsaved drafts prompt clearly to Load Disk or Keep Draft.",
        },
      ],
    },
    coursesSection: {
      heading: "4 built-in engineering courses with isolated Managed Runtimes",
      lead: "Each Course Pack ships Starter files, visible tests, and side-by-side Reference Solutions so you build real instincts across Go, TypeScript, Python, and Java 21.",
      exercisesUnit: "ex",
      runtimeLabel: "Managed Runtime",
      runnerLabel: "Real Test Adapter",
      entryLabel: "Entry File",
      syllabusHeading: "Exercise Syllabus",
      preflightTitle: "Environment Preflight & Isolated Managed Runtimes",
      preflightBody:
        "Preflight checks local toolchains in parallel. If Go, Bun, Python, or JDK 21 is missing or outdated, install a SHA-256 verified Managed Runtime into an isolated runtimes/ folder without mutating your global PATH.",
      runtimePills: [
        "Go 1.23.12",
        "Bun 1.3.13",
        "uv 0.11.6 + Python 3.12.13",
        "Temurin JDK 21 + JUnit 5",
      ],
    },
    tutorSection: {
      heading: "Connect your local AI coding agent with Socratic guidance",
      lead: "Already using Claude Code, Gemini CLI, Codex CLI, or OpenCode locally? CodeLab Studio connects over ACP (Agent Client Protocol over stdio JSON-RPC 2.0) with no extra API key setup—and the entire workbench stays completely functional offline without AI.",
      agentsLabel: "Built-in local ACP presets",
      contextTitle: "Inspectable, removable Context Snapshot",
      contextSub: "Every turn attaches a transparent snapshot capped at 6,000 input tokens while filtering .env, *secret*, and *key* files. Click any chip to remove it:",
      chips: [
        { id: "chapter", label: "Ex 1 Chapter Summary", tokens: 320 },
        { id: "file", label: "task-api/main.go (L14-20)", tokens: 410 },
        { id: "err", label: "Failed Check Diff (v2: 405 vs 200)", tokens: 260 },
        { id: "term", label: "Terminal Tail (go run .)", tokens: 190 },
      ],
      restoreChips: "Restore all chips",
      filterNote: "Auto-filtered: .env · *secret* · *key*",
      modeSocratic: "Socratic Guidance (Default)",
      modeDirect: "Direct Answer",
      socraticReply:
        "You registered \"/healthz\" without an HTTP verb, so Go 1.22+'s ServeMux matches every method. How can you constrain that pattern to GET so POST automatically yields 405?",
      directReply:
        "Change line 18 from mux.HandleFunc(\"/healthz\", healthz) to mux.HandleFunc(\"GET /healthz\", healthz). Since Go 1.22, ServeMux supports method-prefixed routes and returns 405 Method Not Allowed for other methods.",
      permTitle: "ACP Permission Policy · guided_readonly",
      permDesc: "Allows non-sensitive reads inside the Workspace by default, while gating file writes, terminal commands, and out-of-workspace reads behind explicit approval cards.",
      permRequest: "Claude Code wants to edit task-api/main.go (line 18)",
      permAllow: "Allow write",
      permDeny: "Deny",
      permAllowedState: "Write approved for this request (switch to strict_ask_all in Settings anytime).",
      permDeniedState: "Write denied — keep editing yourself or inspect the Reference Solution diff.",
      permReset: "Replay prompt",
    },
    downloadSection: {
      heading: "Download CodeLab Studio v0.1.0 Public Preview",
      lead: "Built by GitHub Actions and distributed directly from GitHub Releases alongside SHA256SUMS.txt.",
      detectedTag: "Recommended",
      downloadCta: "Download installer",
      allReleasesLink: "View full release notes & installation details on GitHub Releases",
      checksumsLink: "SHA256SUMS.txt",
      copyCmd: "Copy checksum command",
      copiedCmd: "Copied",
      firstLaunchTitle: "Honest first-launch notes for Public Preview",
      platforms: {
        macos: {
          name: "macOS (Apple Silicon & Intel Universal)",
          short: "macOS Universal",
          noteTitle: "Ad-hoc signed · Not Apple notarized yet",
          noteBody:
            "Open the DMG and drag CodeLab Studio into Applications. Because v0.1.0 Public Preview is ad-hoc signed without Apple notarization, macOS may show a Gatekeeper prompt on first launch — allow it via System Settings → Privacy & Security.",
        },
        windows: {
          name: "Windows x64 (NSIS Installer)",
          short: "Windows x64",
          noteTitle: "Unsigned preview installer",
          noteBody:
            "Run the .exe installer. Because the v0.1.0 preview is unsigned, Microsoft Defender SmartScreen may display a prompt — select More info → Run anyway to continue.",
        },
        linux: {
          name: "Linux x64 (AppImage · Built on Ubuntu 22.04)",
          short: "Linux x64",
          noteTitle: "Mark executable and run",
          noteBody:
            "Run chmod +x CodeLab-Studio-0.1.0-linux-x64.AppImage and launch it. On newer Ubuntu releases requiring libfuse2, check the launch steps in the v0.1.0 GitHub Release notes.",
        },
      },
      authorTitle: "Writing technical blogs or team onboarding handbooks?",
      authorBody:
        "Skip hosting a proprietary learning site. Keep authoring in standard Markdown, add a lightweight codelab.json and .codelab-checks.json, and test drafts inside an isolated Trial Workspace.",
      authorLink: "Read the Course Pack format spec",
    },
    footer: {
      tagline: "Local-first desktop programming workbench · No login · Offline-complete",
      docsZh: "简体中文文档",
      docsEn: "English README",
      releases: "v0.1.0 Releases",
      repo: "GitHub Repository",
    },
  },
} as const;

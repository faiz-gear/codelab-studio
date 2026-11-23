# CodeLab Studio

CodeLab Studio 是一个开源、本地优先（Local-first）的 Tauri 2 桌面交互式编程实践工作台，面向 macOS、Windows 和 Linux。目标是把标准 Markdown 实践手册或本地与 Git 课程包，转化为同一窗口内的三步引导手册、本地代码工作区、多终端与四语言函数判题环境，并按需展开上下文感知 AI 助教。

产品设计以本地运行为前提，手册、编辑、终端和验收流程无需账号或云端服务。

## 当前实现

- **桌面外壳与持久化。** Tauri 2 和 React 外壳支持应用启动、单实例和全局配置。Rust 工作区存储具备原子写入和损坏恢复。`US-001` 已通过验收。
- **课程后端。** Rust 已能解析 Markdown 章节、三步流和命令块，加载本地课程包和 Git 来源。Go/GoFrame 练习 0 至 9 随应用分发。`US-002` 待真实桌面的预览、信任确认和工作区绑定验收。

### 构建与检查

安装 Node.js、pnpm、Rust stable 和 Tauri 对应平台的构建依赖后，在仓库根目录执行：

```sh
pnpm install --frozen-lockfile
pnpm run check:all
pnpm tauri build --no-bundle
```

macOS 上的发布模式可执行文件位于 `src-tauri/target/release/codelab-studio`。运行中的桌面外壳可切换语言、主题和强调色。

### 后续交付

- **四语言隔离托管运行时与环境体检。** 包含并行探测本地工具链、LSP 与 `GOPROXY`，以及将 Bun、Go 1.23、Eclipse Temurin JDK 21、`uv` 与 Python 3.12 隔离安装至 `~/.codelab/runtimes/`（`US-003`）。
- **本地 PTY 多终端与安全拦截。** 包含基于 `portable-pty` 的常驻服务与单次命令终端路由、端口占用捕获、子进程组清理、高危 Shell 命令确认弹窗与未信任课程包警告（`US-004`、`US-005`、`US-016`）。
- **工作区文件沙箱、Monaco 编辑器与可选 LSP 桥接。** 包含严格路径沙箱、500ms 内同步的文件监听、多标签编辑、外部修改冲突保护、参考代码强制 Diff 合并，以及 `gopls`、`tsserver`、`pyright`、`jdtls` 的按模块根启动与缺失降级（`US-006`、`US-014`、`US-015`）。
- **四语言真实判题与 HTTP 合同校验。** 包含独立进程组和超时熔断下的 `bun test`、`go test -json -race`、JUnit 5、`pytest --json-report` 执行，以及预期与实际结果 Diff 生成（`US-007`）。
- **BYOK 与 ACP 双协议 AI 助教。** 包含操作系统钥匙串存储的 BYOK 流式客户端、基于 JSON-RPC 2.0 over stdio 的 ACP 客户端与 `session/request_permission` 审批门禁、6,000 input tokens 截断上限的四维上下文装配与敏感文件屏蔽（`US-008`、`US-009`、`US-010`、`US-017`）。
- **Arc UI 生产界面与设置中心。** 包含正式组件接入、双主题与 8 组强调色抽屉、单入口首页、默认两栏三步流工作台、设置弹窗与 13 项本地隐私埋点（`US-011`、`US-013`、`US-018`）。

## 快速预览原型

浏览器原型展示产品设计。Go、Python 和 Java 判题、多终端、AI 与课程导入使用模拟数据。JS/TS 示例在浏览器内运行函数断言。

1. 用任意现代浏览器直接打开 [docs/design/prototype/index.html](docs/design/prototype/index.html)，无需安装依赖或启动本地服务。联网时页面加载 Geist、Inter 和 JetBrains Mono 字体，离线时自动回退至本地系统字体。
2. 在首页查看单一主入口卡片与 `2×2` 四语言课程列表，或点击 **导入 / 创作课程包** 体验导入解析与规则编辑弹窗。
3. 打开「Go 与 GoFrame 开发实践」练习 1，依次运行 `go run .` 常驻服务、执行初检查看失败 Diff，再点击 **问 AI** 触发 ACP 权限审批卡。随后通过 **查看参考提示与 Diff** 安全写入修复并复检全绿，并可在 **更多调试命令 (3)** 中点击 `rm -rf` 验证高危确认弹窗。
4. 切换至「TypeScript 数据流水线实战」编辑 `src/pipeline.ts` 中的 `getAdultNames` 后运行验收，验证浏览器内实时 JS 函数断言。点击顶栏 **外观** 测试浅深主题与 8 组强调色切换，按 `⌘/Ctrl + B` 切换专注编码态。

以下截图均采自当前交互原型 [docs/design/prototype/index.html](docs/design/prototype/index.html)。

| 首页单一主入口（浅色） | 默认两栏三步流工作台（浅色） |
| --- | --- |
| ![首页单一主入口与四语言课程卡片](docs/design/screenshots/01-welcome-light.png) | ![默认两栏三步流工作台](docs/design/screenshots/02-workbench-default-light.png) |
| **按需展开 AI 抽屉与 ACP 审批态** | **深色黑曜石主题与验收通过态** |
| ![按需展开 AI 抽屉与 ACP 权限审批卡](docs/design/screenshots/03-workbench-active-ai.png) | ![深色黑曜石主题验收通过态](docs/design/screenshots/04-workbench-dark.png) |

## 目标产品形态与用法

本节描述工程实现完成后的目标使用方式与核心约束，供后续研发对齐。完整规格见 [docs/prd/PRD.md](docs/prd/PRD.md)。

### 学习者主流程

1. 打开桌面应用首页，直接继续上次练习、选择内置四语言示范课程（含标杆课程《Go 与 GoFrame 开发实践手册：从第一性原理出发》练习 0 至 9），或导入本地 Markdown 文件、本地目录与 Git 课程包。
2. 绑定本地工作区目录并执行环境体检。缺少工具链时可一键隔离下载至 `~/.codelab/runtimes/`，不修改系统全局 `PATH`。
3. 在默认两栏工作台按编号三步流推进练习，先启动服务或运行测试，再修改本地代码，最后运行自动验收清单。项目文件树、次要调试命令、思考题与 AI 抽屉默认收起，按需展开。
4. 在手册正文、编辑器或终端输出划选 2 个以上字符，150ms 内浮现操作胶囊并可唤起 AI 助教。助教默认采用引导思考模式，输入框上方展示可移除的四维上下文芯片与实时 Token 预估。
5. 全部自动校验项通过后，应用将章节标记为完成，并原子写入 `<workspace>/.codelab/progress.json`。

### 课程包规范（目标格式）

普通标准 Markdown 教程无需修改即可直接导入。解析器将 `#` 识别为课程名，按 `##` 拆分有序章节与稳定 ID，并将 `bash`、`sh`、`zsh` 围栏代码块识别为可执行命令块（缺失扩展属性时自动推断 `cwd` 与常驻或单次运行模式）。

需要附带初始代码模板与自动验收规则时，课程包创作者可按 [docs/prd/PRD.md](docs/prd/PRD.md) 约定的目录格式分发本地目录或 Git 仓库。下述目录是目标外部课程包格式，并非当前仓库已有目录。

```text
my-course-pack/
├── codelab.json             # 课程元信息（名称、语言、简介、依赖、章节索引）
├── README.md                # 标准 Markdown 实践手册（保留纯阅读兼容性）
├── starter/                 # 初始单文件或多文件代码模板
└── .codelab-checks.json     # Schema v1.0 自动验收与函数判题规则
```

### 四语言运行时与判题目标矩阵

| 语言 | 隔离托管运行时（`~/.codelab/runtimes/`） | 单文件与多文件判题方式 | 可选 LSP | 超时熔断 |
| --- | --- | --- | --- | --- |
| JS/TS | 便携单文件 Bun（或系统 Node ≥ 20） | `bun test` 或内置 TS Runner | `tsserver` | 5s |
| Go | 官方绿色解压版 `go1.23` | `go test -json -race ./...` 与 PTY `go run .` | `gopls` | 10s |
| Java | 便携 Eclipse Temurin JDK 21 | `javac` 与 JUnit 5 Console Launcher | `jdtls` | 12s |
| Python | 单文件 `uv` 拉取独立 Python 3.12 | `pytest --json-report` | `pyright` | 8s |

### AI 助教协议与开发硬约束

后续工程实现必须满足以下来自 [docs/prd/PRD.md](docs/prd/PRD.md) 与 [docs/design/design-system.md](docs/design/design-system.md) 的基线要求。

- **BYOK 与 ACP 双协议。** BYOK 支持 OpenAI 兼容端点、DeepSeek、Anthropic、本地 Ollama 与自定义端点，API Key 仅保存在系统钥匙串（Keychain、Credential Manager、Secret Service），禁止写入工作区或日志。ACP 作为 JSON-RPC 2.0 over stdio 客户端连接本机 Claude Code、Gemini CLI、Codex CLI、OpenCode 或自定义 `--acp` 命令，引导模式默认仅放行工作区只读，写文件与执行命令须经对话流审批卡显式允许或拒绝。
- **上下文截断与隐私保护。** 上下文装配强制 6,000 input tokens 上限，永不截断用户选区与最新报错尾，并自动屏蔽 `.env`、`*secret*`、`*key*` 文件。本地埋点默认仅落盘 `<workspace>/.codelab/metrics.json`（统一 13 项事件），匿名统计默认关闭且永不上报源码、命令明文、文件路径、对话原文与密钥。
- **沙箱、终端与写入安全。** 所有文件读写限制在规范化后的 `workspaceRoot` 内，阻断 `../` 路径穿越与越界软链接。手册参考代码禁止静默覆盖用户文件，须通过左右 Diff 弹窗确认。常驻服务终端与单次命令终端严格隔离，高危命令（如 `rm -rf`、`sudo`、`mkfs`、`dd if=`、`chmod -R 777 /`、`curl | sh`）强制弹出默认焦点在「取消」的确认弹窗，外部课程包首次打开须确认信任。
- **进程回收、性能与无障碍。** 退出应用或切换工作区时完整回收所有派生 PTY 与 LSP 子进程树。桌面窗口最低支持 `1024×720`（最佳 `1440×900`），要求冷启动首屏 ≤ `2.0s`、1,000 行 Markdown 解析 ≤ `150ms`、文件树同步 ≤ `500ms`、划词浮层 ≤ `150ms`、常驻内存（不含 LSP）≤ `350 MB`。浅深双主题全色板须满足 WCAG 2.1 AA（≥ `4.5:1`），并完整支持键盘导航与 `prefers-reduced-motion`。

## 文档导航与仓库结构

下表列出产品需求、实施计划和设计参考。

| 查阅目的 | 文件路径 | 内容说明 |
| --- | --- | --- |
| 了解完整需求、边界与指标 | [docs/prd/PRD.md](docs/prd/PRD.md) | v3.1 定稿版产品需求文档，覆盖 M1 至 M6 规格、安全红线与里程碑 |
| 拆解开发任务与验收条件 | [docs/prd/prd-stories.json](docs/prd/prd-stories.json) | 18 条按优先级排列的原子用户故事（`US-001` 至 `US-018`） |
| 跟进剩余交付 | [U2 至 U14 多阶段实施计划](docs/architecture/u2-u14-multi-phase-plan.md) | PR 依赖、实机验证和性能门禁 |
| 对齐视觉体系与渐进披露规范 | [docs/design/design-system.md](docs/design/design-system.md) | Arc UI 纯中性灰阶、双主题对比度校准、界面断点与无障碍规则 |
| 复用三层 CSS 变量令牌 | [docs/design/tokens.css](docs/design/tokens.css) | `oklch` 色彩、字体、圆角、动效曲线与 8 组 `data-accent` 定义 |
| 接入 Tailwind 主题映射 | [docs/design/tailwind.config.js](docs/design/tailwind.config.js) | 将 `tokens.css` 映射为 Tailwind 配置的基准文件 |
| 复核设计评审与回归结果 | [docs/design/design-review.md](docs/design/design-review.md) | R1 至 R3 迭代记录、Headless Chrome 实机检查与分项评审得分 |
| 直接体验高保真交互原型 | [docs/design/prototype/index.html](docs/design/prototype/index.html) | 浏览器可打开的单文件交互原型 |
| 查看原型实机状态截图 | [docs/design/screenshots/](docs/design/screenshots/) | 4 张 `2880×1800` 原型截图（首页、工作台默认态、AI 与 ACP、深色验收通过） |

主要目录包括：

- `src/` 存放桌面前端，`src-tauri/` 存放 Rust 后端与集成测试。
- `schemas/` 存放版本化 JSON Schema，`scripts/` 存放工程检查与桌面冒烟脚本。
- `builtin-courses/` 存放随应用分发的课程包，`docs/` 存放需求、架构和设计文档。

## 研发里程碑（8 周开源 MVP）

后续工程排期与验收以 [docs/prd/PRD.md](docs/prd/PRD.md) 第 6 节及 [docs/prd/prd-stories.json](docs/prd/prd-stories.json) 为基准。

| 阶段 | 周期 | 对应用户故事 | 核心交付 |
| --- | --- | --- | --- |
| 阶段 1 核心骨架 | 第 1 至 3 周 | `US-001`、`US-002`、`US-004`、`US-005`、`US-006` | Tauri 2 桌面壳、工作区沙箱与 `.codelab/` 原子存储、Markdown 与课程包解析、PTY 多终端与安全拦截 |
| 阶段 2 运行时与编辑 | 第 4 至 5 周 | `US-003`、`US-007`、`US-014`、`US-015` | 四语言托管运行时与函数判题、实时文件树、Monaco 编辑器与多语言 LSP 降级桥接 |
| 阶段 3 AI 与 ACP | 第 6 周 | `US-008`、`US-009`、`US-010`、`US-017` | BYOK 钥匙串客户端、ACP Client 与权限审批卡、四维上下文装配、划词胶囊与 AI 抽屉 |
| 阶段 4 验收与发版 | 第 7 至 8 周 | `US-011`、`US-012`、`US-013`、`US-016`、`US-018` | Arc UI 渐进披露工作台联调、xterm.js 终端与判题 Diff 底栏、设置与隐私埋点、四语言端到端回归及三端开源发布 |

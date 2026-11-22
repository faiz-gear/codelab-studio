# PRD：CodeLab Studio — 本地交互式编程实践工作台

> 版本：**v3.1（正式整合定稿版）**
> 更新日期：2026-11-22
> 配套产物：[结构化任务拆解（18 条用户故事）](./prd-stories.json) · [Arc UI 设计系统](../design/design-system.md) · [交互原型](../design/prototype/index.html)

---

## 1. 产品定位与核心目标

**一句话定位**：开源、本地优先（Local-first）的 **Tauri 2 原生桌面编程实践工作台**，把任意标准 Markdown 实践手册或本地 / Git 课程包，转化为「编号引导手册 + 本地代码工作区 + 智能多终端与函数判题 + 划选上下文感知 AI / ACP 助教」一体化环境。

**核心边界**：
- **纯开源本地工具**：零登录、零订阅、零强制云端依赖；无网络 / 未配置 AI 时，文档、编码、多终端与自动化判题 100% 离线可用。
- **桌面端形态**：Tauri 2（Rust 后端 + Web 前端），支持 macOS / Windows / Linux，聚焦桌面窗口（最低 `1024×720`，最佳 `1440×900`）。
- **首期语言范围**：**JS/TS、Go、Java、Python** 四大语言，同时覆盖「单/多文件函数判题」与「工程实战手册双终端演练」两种形态。
- **标杆内置课程**：《Go 与 GoFrame 开发实践手册：从第一性原理出发》（练习 0～9）+ JS/TS、Python、Java 函数判题示范课程包。

### 量化成功指标（公测发布 90 天内）

| 维度 | 核心指标 | 目标值 |
|---|---|---|
| 效率提升 | 单练习窗口切换与手动复制粘贴次数（对比纯文档） | 下降 **≥ 80%** |
| 学习闭环 | 内置 Go/GoFrame 课程 14 日完课率（完成 ≥ 练习 7） | **≥ 45%** |
| 执行可靠 | 一键终端与判题调度成功率（非业务代码错误） | **≥ 98%** |
| 助教有效 | 划词 / 报错提问单次解决率（用户点赞或 3 分钟内未重复追问） | **≥ 70%** |
| 生态兼容 | 外部标准 Markdown 教程零改造可解析率 | **≥ 90%** |

---

## 2. 目标用户与核心旅程

| 用户画像 | 典型痛点 | 核心诉求 |
|---|---|---|
| **进阶自学者**（1～3 年开发者） | 读长篇实战手册要在文档/IDE/终端间切换 20～35 次；本地环境难配；卡壳排错要手工搬运上下文 | 开箱即用、入口一眼清晰、所见即所跑、报错即问、有明确验收证据 |
| **课程包创作者**（技术博主 / Tech Lead） | 教程跑在不同读者机器上环境千差万别，不想维护重型专有教学平台 | 继续写标准 Markdown + 轻量 `.codelab-checks.json`，即可分发可交互课程包 |

```text
打开应用（单一主入口）→ 继续上次练习 / 选择四语言课程 / 导入本地或 Git 课程包
  → 绑定本地工作区并自动环境体检（缺运行时可一键隔离托管安装）
  → 默认清爽两栏进入「① 运行/启动 → ② 修改代码 → ③ 自动验收」三步引导流
  → 遇卡点或失败 Diff → 划选任意片段唤起 AI 助教（BYOK 或 ACP 外部 Agent，默认引导模式）
  → 全绿验收通过 → 自动推进进度并落盘至 <workspace>/.codelab/progress.json
```

---

## 3. 信息架构与渐进披露原则（v3.1）

为消除「信息过载、入口不清、文字密集」的认知压力，产品界面强制遵循 **Arc UI 纯中性留白 + 渐进披露** 原则：

1. **首页单一入口**：顶部居中 Arc 氛围光晕卡片只呈现「上次学习进度 + 1 个主 CTA（继续练习）+ 1 个次 CTA（导入/创作课程包）」，下方为宽松 `2×2` 课程卡片（仅展示语言标签、进度、标题与一行短描述）及最近工作区；移除非行动导向的统计数字墙与冗余装饰。
2. **默认清爽两栏工作台**：进入工作台默认只呈现「左侧编号三步流手册（`① → ② → ③`）+ 右侧宽裕代码编辑器与紧凑终端」。
3. **次级能力按需展开**：项目文件树、次要调试命令、课后思考题与 AI 助教抽屉默认收起，点击入口图标、划选文本或触发报错时平滑展开；`⌘/Ctrl + B` 一键进入专注编码态。

---

## 4. 功能模块详述（M1 – M6）

### M1 · 交互式手册解析与课程包生态

| 能力 | 规格与边界 |
|---|---|
| **标准 Markdown 零改造解析** | `#` 为课程名，`##` 拆分稳定章节 ID；`bash/sh/zsh` 围栏块解析为可执行命令块（优先读扩展属性 `cwd` / `mode`，缺失时从上下文 `cd` 及 `go run` / `npm run dev` / `gf run` 自动推断）；识别验收与思考区块。文件上限 5 MB，支持外部改动热提示。 |
| **编号三步流与折叠收纳** | 每节展示核心目标（1 句话）→ `① 启动服务 / 运行测试` → `② 修改代码（含参考提示与 Diff 防覆盖）` → `③ 自动验收清单`；其余辅助命令与思考题折叠收纳于 `<details>`。 |
| **标准化课程包（导入 + 创作）** | 支持导入本地 `.md`、本地目录或 Git 仓库 URL；内置「创作课程包」工作台，支持在线编辑 Markdown 与 `.codelab-checks.json` 并一键试跑。 |

**标准化课程包目录与 `.codelab-checks.json` Schema**：

```text
my-course-pack/
├── codelab.json             # 元信息（名称、语言、简介、依赖、章节索引）
├── README.md                # 标准 Markdown 实践手册（兼容纯阅读）
├── starter/                 # 初始单/多文件代码模板
└── .codelab-checks.json     # 自动化验收与函数判题规则
```

```json
{
  "schemaVersion": "1.0",
  "courseId": "go-goframe-practice",
  "language": "go",
  "chapters": [
    {
      "chapterId": "ex-1",
      "requiresDaemon": { "label": "go run .", "port": 8080 },
      "checks": [
        {
          "id": "v1",
          "title": "GET /healthz 返回 200 {\"status\":\"ok\"}",
          "type": "http_assert",
          "request": { "method": "GET", "url": "http://127.0.0.1:8080/healthz" },
          "expect": { "status": 200, "bodyContains": "\"status\":\"ok\"" }
        },
        {
          "id": "v2",
          "title": "go test -race ./... 零竞态通过",
          "type": "command_test",
          "cwd": "task-api",
          "command": "go test -json -race ./...",
          "timeoutSec": 10,
          "expectExitCode": 0,
          "forbidOutput": ["WARNING: DATA RACE"]
        }
      ]
    }
  ]
}
```

---

### M2 · 本地学习工作区与代码产物台

- **严格沙箱隔离**：所有文件读写/新建/重命名校验规范化绝对路径必须位于 `workspaceRoot` 内，阻断 `../` 路径穿越与越界软链接；进度与日志存放在 `<workspace>/.codelab/`（原子写入，默认隐藏并建议加入 `.gitignore`）。
- **实时产物文件树与多标签编辑器**：500ms 内同步磁盘变更；支持多文件 Tab、未保存圆点、`⌘/Ctrl + S` 保存、双文件左右 Diff 对照、一键在外部 IDE（VS Code / Cursor / GoLand / IDEA）打开；超过 2 MB 或二进制文件拒绝文本加载并展示元信息。
- **外部修改冲突保护**：外部 IDE 修改文件时，若无本地草稿则静默刷新，若有未保存修改则顶部弹出「加载磁盘版 / 保留草稿」横幅。
- **参考代码安全写入**：手册参考片段禁止直接覆盖用户手写文件，强制弹出左右 Diff 对比弹窗确认后再合并。
- **可选多语言 LSP**：支持 `gopls`、`typescript-language-server`、`pyright`、`jdtls`；按活跃文件向上定位最近模块根（如同时维护 `task-api/` 与 `task-api-gf/`）；缺失或异常时平滑降级为语法高亮，不阻断编辑。

---

### M3 · 四语言托管运行时、智能多终端与函数判题

#### 四语言本地执行与判题引擎

| 语言 | 托管运行时（隔离于 `~/.codelab/runtimes/`） | 单/多文件函数判题 | 可选 LSP | 超时熔断 |
|---|---|---|---|---|
| **JS / TS** | 便携单文件 **Bun**（或系统 Node ≥ 20） | `bun test` / 内置 TS Runner | `tsserver` | 5s |
| **Go** | 官方绿色解压版 `go1.23` | `go test -json -race ./...` + PTY `go run .` | `gopls` | 10s |
| **Java** | 便携 **Eclipse Temurin JDK 21** | `javac` + JUnit 5 Console Launcher | `jdtls` | 12s |
| **Python** | 单文件 **`uv`** 拉取独立 Python 3.12 | `pytest --json-report` | `pyright` | 8s |

#### 智能多终端路由与安全红线
1. **常驻与单次命令严格隔离**：`daemon` 命令（如 `go run .`）独立启动命名服务 Tab（含绿色运行脉冲、`■ 停止 SIGINT`、`↻ 重启`）；`oneshot` 命令（如 `curl`、`go test`）优先复用空闲普通终端或新建 Tab，**绝不向运行中的服务终端误输入文本**。
2. **端口冲突感知**：捕获 `address already in use` 并在底栏提示「切到服务终端 / 问 AI」。
3. **高危 Shell 命令强拦截**：所有命令块禁止静默自动执行；命中 `rm -rf`、`sudo`、`mkfs`、`dd if=`、`chmod -R 777 /`、`curl | sh` 等模式强制弹出红色审查弹窗（明文展示目录与完整命令，默认焦点「取消」）。
4. **防卡死与清理**：单次判题运行在独立进程组 / Windows Job Object，超时自动 `SIGINT → SIGKILL`；退出应用或切换工作区时 100% 清理派生 PTY / LSP 子进程；UI 终端保留 5,000 行缓冲，AI 采集环形缓冲保留最近 200 行。

---

### M4 · 上下文感知 AI 助教（BYOK + ACP 双协议）

1. **跨三区域划词浮层**：在手册正文、代码编辑器或终端输出鼠标拖选 `≥ 2` 字符，150ms 内在选区附近浮现不遮挡选区的胶囊操作条：
   - 手册：`解释概念` · `提问`
   - 编辑器：`解释代码` · `查问题` · `提问`
   - 终端：`诊断报错` · `提问`
2. **双接入协议**：
   - **模式 A · 自定义 Provider（BYOK）**：支持 OpenAI 兼容、DeepSeek、Anthropic、本地 Ollama 及自定义端点；API Key 强制存入系统钥匙串（Keychain / Credential Manager / Secret Service），严禁写入工作区或日志。
   - **模式 B · ACP 外部 Agent**：遵循 **Agent Client Protocol（JSON-RPC 2.0 over stdio）**，CodeLab Studio 作为 ACP Client 拉起本机 Claude Code / Gemini CLI / Codex CLI / OpenCode / 自定义 `--acp` 命令；完整支持 `initialize`、`session/new`、`session/prompt`、`session/update`，并拦截 `session/request_permission`——**引导模式默认仅放行工作区只读，写文件与执行命令须用户在对话流审批卡显式允许或拒绝**。
3. **四维上下文透明可控与教学风格**：
   - 输入框上方展示可移除 **Context Chips**（当前章节、活跃文件片段、终端报错尾、验收状态）与实时 Token 预估；强制 **6,000 input tokens** 截断上限（永不截断用户选区与最新报错尾）；自动屏蔽 `.env` / `*secret*` / `*key*`。
   - 支持 **引导思考（苏格拉底，默认）** 与 **直接解答** 实时切换，支持流式输出与「停止生成」。

---

### M5 · 自动化验收与学习进度闭环

- 验收清单区分自动校验项（HTTP 合同断言 / 函数单测 / 退出码与竞态检测）与自查项；失败条目自动展开「预期 vs 实际」双列 Diff 并附「问 AI」直达按钮。
- 全部自动项通过后自动标记章节完成，推进顶部与首页进度，支持一键重置或导出本地学习记录。

---

### M6 · 环境体检、外观定制与本地隐私设置

- **环境体检（Preflight Check）**：并行探测四语言工具链、LSP 与 `GOPROXY`（单项超时 5s），缺失项支持一键下载隔离版至 `~/.codelab/runtimes/`。
- **Arc UI 外观定制抽屉**：支持浅色 Light（默认）/ 深色 Dark 双主题、8 组 Arc 强调色（Neutral/Violet/Blue/Green/Amber/Orange/Coral/Rose）、代码字号及一键复制主题 CSS。
- **隐私与本地埋点**：默认仅落盘 `<workspace>/.codelab/metrics.json`，Opt-in 匿名统计严格脱敏，永不上报源码、命令明文、文件路径、对话原文与 Key。统一 13 项本地事件：`app_launched`、`course_opened`、`preflight_executed`、`runtime_installed`、`cmd_block_clicked`、`cmd_risk_modal_action`、`cmd_finished`、`file_tree_open`、`selection_ai_clicked`、`ai_mode_switched`、`acp_permission_decision`、`verify_chapter_clicked`、`chapter_marked_done`。

---

## 5. 非功能需求与安全红线

| 维度 | 强制指标 |
|---|---|
| **性能** | 冷启动首屏 ≤ `2.0s`；1,000 行 Markdown 解析 ≤ `150ms`；文件树同步 ≤ `500ms`；划词浮层 ≤ `150ms`；高吞吐终端 ≥ `30 FPS`，常驻内存（不含 LSP 子进程）≤ `350 MB`。 |
| **安全** | 工作区路径沙箱；高危命令强确认；外部课程包首次打开信任确认；API Key 系统钥匙串加密；Markdown 渲染严格 XSS 清洗；ACP 写/执行权限审批隔离。 |
| **可靠性** | 进度 JSON 临时文件 + 原子重命名写入；退出 100% 回收子进程树；无网络完全离线可用。 |
| **无障碍** | 浅/深双主题全色板符合 **WCAG 2.1 AA（≥ 4.5:1）**；完整键盘导航（`⌘B`、`⌘S`、`Esc`）、`:focus-visible`、`aria-live`、`prefers-reduced-motion`。 |

---

## 6. 交付里程碑（8 周开源 MVP）

| 阶段 | 周期 | 核心里程碑 |
|---|---|---|
| 阶段 1 · 核心骨架 | 第 1～3 周 | Tauri 2 壳、工作区沙箱、Markdown/课程包解析器、PTY 多终端与安全拦截 |
| 阶段 2 · 运行时与编辑 | 第 4～5 周 | 四语言托管运行时与函数判题、实时文件树、Monaco + 多语言 LSP 降级桥接 |
| 阶段 3 · AI 与 ACP | 第 6 周 | 三区划词胶囊、四维上下文装配、BYOK 钥匙串客户端、ACP Client 与权限审批卡 |
| 阶段 4 · 验收与发版 | 第 7～8 周 | Arc UI 渐进披露工作台联调、Go/GoFrame + 四语言端到端回归、macOS/Win/Linux 开源发布 |

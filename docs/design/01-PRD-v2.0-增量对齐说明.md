# CodeLab Studio — PRD v2.0 增量对齐与规格补充

> 版本：v2.0（基于原 [02-产品需求文档-PRD.md](file:///Users/lyle/Work/sandbox/codelab-studio/docs/prd/02-产品需求文档-PRD.md) 与 [03-设计需求文档.md](file:///Users/lyle/Work/sandbox/codelab-studio/docs/prd/03-设计需求文档.md) 增量对齐）
> 更新日期：2026-11-21
> 同步补齐：[04-PRD量化评审报告.md](file:///Users/lyle/Work/sandbox/codelab-studio/docs/prd/04-PRD量化评审报告.md) 中指出的 P0 协议 Schema 与埋点口径统一项

---

## 1. v2.0 核心定位与增量变更摘要

CodeLab Studio 定位于**开源、本地优先（Local-first）的 Tauri 2 原生桌面编程实践工作台**，无云端账号依赖、无商业订阅门槛。

| 变更维度 | v1.0 原 PRD | v2.0 对齐后规格 |
|---|---|---|
| **产品外壳** | Tauri / Electron 或本地 Web 服务 | 确定为 **Tauri 2（Rust 后端 + Web 前端）原生桌面应用**，支持 macOS / Windows / Linux；聚焦桌面端窗口（≥1024px，1440×900 最佳） |
| **语言与练习形态** | Go & GoFrame 单语言标杆（重 PTY 双终端与 HTTP 验收） | 首期支持四大语言：**JS/TS、Go、Java、Python**；初期覆盖**单文件 / 多文件函数判题**与**实战手册多终端演练**双形态 |
| **本地运行时管理** | 仅检测本机 `go`/`curl`/`gopls`/`GOPROXY` | 升级为**四语言运行时管理中心**：自动探测系统 PATH 工具链；缺失时支持**一键下载隔离托管运行时**至 `~/.codelab/runtimes/`（Bun、官方 Go 压缩包、Temurin JDK、`uv` + Python），绝不污染全局环境 |
| **课程包生态** | 导入单个本地 `.md` + 内置 Go/GoFrame 手册 | 新增**标准化课程包机制**：支持导入**本地目录 / Git 仓库课程包** + 内置**创作者编写课程包工作台**（元数据配置、`.codelab-checks.json` 校验与一键预览试跑） |
| **AI 助教接入协议** | BYOK（OpenAI 兼容 / Anthropic / Ollama + 系统钥匙串） | 升级为**双协议接入**：① **自定义 Provider（BYOK）**；② **ACP（Agent Client Protocol，JSON-RPC 2.0 over stdio）**直连外部 Coding Agent（Claude Code、Gemini CLI、Codex CLI、OpenCode 或自定义 `--acp` 命令），工作台掌控权限审批 |

---

## 2. 四语言本地运行与判题架构

桌面端 Rust 后端提供统一的 `CodeRunner` 与 `RuntimeManager` 接口，按课程语言自动适配测试驱动与 LSP 语言服务：

| 语言 | 托管运行时来源（隔离于 `~/.codelab/runtimes/`） | 单/多文件函数判题引擎 | 可选 LSP 语言服务器 | 典型超时熔断 |
|---|---|---|---|---|
| **JS / TS** | 便携单文件 **Bun**（或探测本机 Node ≥ 20） | `bun test` / 内置断言 Runner（原生直跑 TS） | `typescript-language-server` | 5s |
| **Go** | 官方绿色解压版 `go1.23.<os>-<arch>` | 临时目录 `go test -json -race ./...` + PTY `go run .` | `gopls` | 10s |
| **Java** | 便携 **Eclipse Temurin JDK 21** | `javac` + 轻量 JUnit 5 Console Launcher JSON 输出 | `jdtls` | 12s |
| **Python** | 单文件 **`uv`** 自动拉取 `python-build-standalone` | `python -m pytest --json-report`（或 `unittest`） | `pyright` / `ruff` | 8s |

**本地执行安全与防卡死约束：**
1. 每次单次判题运行在独立进程组（Process Group / Windows Job Object），超时自动 `SIGINT → SIGKILL` 清理子进程树，UI 提供「**■ 停止运行**」按钮。
2. 所有文件读写严格限制在绑定工作区 `workspaceRoot` 内，阻断路径穿越；高危 Shell 命令（`rm -rf`、`sudo`、`curl | sh` 等）强制明文弹窗二次确认。

---

## 3. AI 助教双协议规格（BYOK + ACP）

| 接入模式 | 适用人群 | 协议与传输 | 安全与教学控制 |
|---|---|---|---|
| **模式 A：自定义 Provider（BYOK）** | 使用模型 API 或本地大模型的用户 | HTTPS Streaming（OpenAI-compatible / Anthropic）或本地 Ollama | API Key 强制存 OS Keychain；单次请求 6,000 input tokens 预算截断；输入框上方以可移除 Chip 透明展示四维上下文 |
| **模式 B：ACP 外部 Agent** | 已安装 Claude Code、Gemini CLI、Codex CLI、OpenCode 等终端 Agent 的开发者 | **Agent Client Protocol（JSON-RPC 2.0 over `stdio`）**，CodeLab Studio 作为 ACP Client 拉起本地 Agent 子进程 | 支持 `initialize` 能力协商、`session/new`、`session/prompt`、`session/update` 流式计划/回复；工作台拦截 `session/request_permission`：**引导模式下默认仅授予工作区只读权限**，写入文件或执行命令必须由用户在审批卡片中显式点击「允许本次 / 拒绝」 |

---

## 4. 标准化课程包与 `.codelab-checks.json` 协议（补齐评审 P0 项）

课程包可为本地目录或 Git 仓库，约定结构如下：

```text
my-course-pack/
├── codelab.json             # 课程包元信息（名称、语言、简介、前置依赖、章节索引）
├── README.md                # 标准 Markdown 实践手册（含可选围栏属性 cwd / mode / file）
├── starter/                 # 初始单/多文件模板（写入用户工作区）
└── .codelab-checks.json     # 自动化验收与函数判题规则（伴生文件，不破坏纯 Markdown 阅读）
```

### `.codelab-checks.json` 字段级 Schema 示例

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
          "id": "ex1-healthz-200",
          "title": "GET /healthz 返回 200 且 Body 含 ok",
          "type": "http_assert",
          "request": { "method": "GET", "url": "http://127.0.0.1:8080/healthz" },
          "expect": { "status": 200, "bodyContains": "\"status\":\"ok\"" }
        },
        {
          "id": "ex1-unit-test",
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

## 5. 统一本地埋点口径（对齐评审 P0 项，12 事件全集）

遵循本地优先原则，默认仅落盘 `.codelab/metrics.json`，仅用户显式 Opt-in 开启匿名统计后脱敏上报：

`app_launched` · `course_opened` · `preflight_executed` · `runtime_installed` · `cmd_block_clicked` · `cmd_risk_modal_action` · `cmd_finished` · `file_tree_open` · `selection_ai_clicked` · `ai_mode_switched` · `acp_permission_decision` · `verify_chapter_clicked` · `chapter_marked_done`

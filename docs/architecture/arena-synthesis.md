# CodeLab Studio 系统架构 Arena 综合决策记录

> 最终架构蓝图：[ARCHITECTURE.md](./ARCHITECTURE.md)
> 评审日期：2026-11-23
> 模式说明：本项目架构横跨 6 大模块与 18 条用户故事，按 `architect` 技能规范通过 **Arena** 流程并行生成 3 份独立候选方案，再由只读跨模型评委盲评打分、选定基准并从第一性原理嫁接与补齐盲区。

## 1. 评审阵容与模型家族说明

- **候选生成者（3 路并行，0 丢弃）：** `candidate-1`（`inherit`，父会话模型）、`candidate-2`（`pro`）、`candidate-3`（`flash`）。
- **只读盲评评委（Phase C）：** `pro`（按 `candidate-1` / `candidate-2` / `candidate-3` 路径标签盲评，不知晓各候选对应模型）。
- **模型家族披露：** Antigravity 平台配置的 `inherit`、`pro`、`flash` 同属 Gemini 模型家族（不同能力档位），已在评审结论与独立复核中显式计入该限制。

## 2. 评分矩阵（五项量化标准，每项 10 分，满分 50）

独立通读全部候选方案后的评分与只读盲评完全一致：

| 评审标准 | `candidate-1` | `candidate-2` | `candidate-3` |
|---|---:|---:|---:|
| 1. 单一状态所有权与清晰边界 | 9 | 8 | 6 |
| 2. 核心类型与 IPC / Schema 严谨度 | 9 | 7 | 5 |
| 3. 并发、子进程生命周期与故障恢复 | 8 | 8 | 6 |
| 4. 需求全覆盖与最小复杂度 | 9 | 8 | 6 |
| 5. 模块化结构与可验证交付序列 | 9 | 8 | 7 |
| **总分** | **44** | **39** | **30** |

## 3. 基准选型：`candidate-1`

三方高度收敛的核心共识形状包括：Rust 后端独占系统副作用、单一 `WorkspaceSession` 统一持有子进程生命周期、Rust 侧 `pulldown-cmark` 单源解析 Markdown、BYOK 与 ACP 双协议统一收口于 Rust 且密钥独占 OS Keychain、Tauri 2 IPC 按请求/流/广播分层、前端 Zustand 切片与命令式 `xterm.js`/`Monaco` 分流。

选择 **`candidate-1`** 作为基础架构，原因在于它在不变量强度和可维护性上最完整：

1. 通过 `WorkspaceSession` 配合单调递增 `SessionEpoch` 和 `SessionEventEnvelope<T>`，在结构上杜绝工作区切换时的陈旧命令与事件串扰，并显式禁用 `tauri-plugin-fs`、`tauri-plugin-shell`、`tauri-plugin-http`。
2. 完整定义品牌化核心类型（`CanonicalWorkspacePath`、`RelWorkspacePath`、`RiskConfirmationToken`、`AiStreamEvent`、精确 13 项 `MetricEvent` 枚举）及全部磁盘 JSON Schema，并保留 `schemars` 供 `US-012` 创作模式驱动 Monaco 校验。
3. 唯一主动对齐 `DESIGN.md`（v3.1 初稿）与 `docs/design/design-system.md` / `tokens.css`（v3.2 B 方案定稿）的布局尺寸差异，且给出覆盖 `US-001` 至 `US-018` 的 15 个带验收门禁交付单元（`U0` 至 `U14`）。

## 4. 方案嫁接（Grafts）

以下优势机制经从第一性原理重整后并入 [ARCHITECTURE.md](./ARCHITECTURE.md)：

| 来源候选 | 嫁接内容 | 解决的问题 |
|---|---|---|
| `candidate-2` | 用 `ts-rs` 单向生成 `generated.ts` 配合薄 `client.ts` 替代 `tauri-specta` v2 命令宏，同时保留 `schemars` | 规避 `tauri-specta` v2 宏与 Tauri 2 二进制 `Channel` 签名的演化耦合，保持单源派生与 CI 漂移校验 |
| `candidate-2` | 为 `RelWorkspacePath` 增加 `#[serde(try_from = "String")]` 反序列化守卫 | 非法路径字符、路径穿越段与 Windows 保留名在 IPC 反序列化入口即被阻断 |
| `candidate-2` | 在 `WorkspaceSandbox` 采用 `dunce::canonicalize` 替代裸 `std::fs::canonicalize` | 消除 Windows `\\?\` UNC 前缀导致 `go`、`javac`、`bun`、LSP `file://` URI 与外部 IDE 失败 |
| `candidate-2` | 为 `AtomicJsonStore` 增加 `.bak` 备份回退与启动 `.tmp.*` 清理，并将 `metrics.json` 写入收敛至单工作区 `mpsc` `MetricsWriter` | 提升崩溃恢复能力，消除高频 UI 埋点逐条 `sync_all()` 造成的磁盘抖动 |
| `candidate-2` | 在 `pulldown-cmark` 管线内对无害行内 HTML（`<br>`、`<kbd>`、`<details>` 等）走 `ammonia` 严格白名单后映射入 `MarkdownNode` AST；`AppError` 引入 `message_key`/`params`；增加 `pty_attach_stream` 快照重挂命令 | 在保持零 `dangerouslySetInnerHTML` 的同时守住外部教程 `>= 90%` 解析率，完善双语报错与组件重挂滚动恢复 |
| `candidate-3` | 在 `ContextChip` 上显式暴露 `redacted_secrets: u32` 计数 | 让学习者在对话输入框芯片栏同时可见已屏蔽敏感文件与内联脱敏计数 |

## 5. 被淘汰方案与原因

- **淘汰 `candidate-1` 中的 `tauri-specta` 全宏绑定、裸 `std::fs::canonicalize` 与同步逐条刷盘 `metrics.json`：** 已被上述来自 `candidate-2` 的更稳健机制替换。
- **淘汰 `candidate-2` 的无 `SessionEpoch` 事件总线、缺少 `tcgetpgrp`/JobObject 兜底的单一 OSC 133 空闲检测，以及错误标注的 `52px` 专注导轨：** 存在工作区切换竞态、自定义 Shell 误判空闲和设计规格漂移风险。
- **淘汰 `candidate-3` 的 `dispatch_command_block(..., risk_approved: bool)`、裸字符串路径、Windows `GetConsoleProcessList` 空闲检测及不完整断点（遗漏 `1361–1439px`）：** 存在前端绕过高危命令门禁、GUI 进程下 ConPTY 句柄失效及布局溢出缺陷。

## 6. 三方共同盲区的第一性原理补齐

盲评与复核共同识别出 8 处三份候选均未彻底解决的底层工程隐患，已在 [ARCHITECTURE.md](./ARCHITECTURE.md) 中统一重设计解决：

1. **PTY 二进制流反序列化膨胀与 UTF-8 截断乱码：** 废弃 `Channel<PtyStreamChunk { data: Vec<u8> }>`（JSON 整数数组膨胀 3 至 4 倍），改用 Tauri 2 原生二进制 `Channel<tauri::ipc::InvokeResponseBody>`（`Raw`）直喂 `xterm.write(Uint8Array)`。
2. **`portable-pty` 进程组与 ConPTY 真实 API 约束：** 修正不存在的 `portable-pty` `pre_exec` 假设；Unix PTY 使用 `openpty` 内置 `setsid` + `killpg`，Windows PTY 取得 PID 后 `AssignProcessToJobObject`，且空闲检测排除 `conhost.exe`/`OpenConsole.exe` 后检查仅余根 Shell PID。
3. **无 Bun 仅系统 Node `>= 20` 时的 TS 判题回退：** 明确内置纯 Rust `oxc_transformer` 将 `.ts` 剥离类型输出至 `.codelab/build-cache/ts/` 后调用 `node --test`，100% 离线无需额外 npm 安装。
4. **ACP 双向协议完整性与越权兜底：** 除拦截 `session/request_permission` 外，明确承接 Agent 反向调用 `fs/read_text_file`、`fs/write_text_file` 与 `terminal/*`，并在执行点二次强制 `WorkspaceSandbox`、敏感路径黑名单与已批准 `AcpDecision` 校验，同时独立采集 `stderr` 登录鉴权报错。
5. **AI 对话历史所有权与首屏前 `app_launched` 指标缓存：** 明确对话历史原子落盘于 `<workspace>/.codelab/chat.json` 支撑 `US-018` 清空动作；首页未绑定工作区前的埋点暂存 `AppState.PendingAppMetrics`，首次打开工作区时通过 `std::mem::take` 一次性冲刷。
6. **LSP `file://` URI 规范化、`jdtls` 工作目录隔离与脏缓冲同步：** 使用 `dunce::canonicalize` + `url::Url::from_file_path` 统一映射；将 `jdtls -data` 隔离至 `~/.codelab/cache/lsp/jdtls/<hash>/` 防止污染工作区；通过 100ms 防抖 `didChange` 同步未保存草稿。
7. **`starter/` 幂等注水与外部 IDE 原子重命名保存事件：** `workspace.json` 记录 `hydratedFiles` 防止误覆盖或复活学习者已删文件；将打开标签路径上的 `Create` 和 `RenameTo` 事件统一归一化为 `ContentChanged`，适配 VS Code、GoLand、Vim 的原子重命名保存。
8. **Git 课程包缓存限额与淘汰：** 明确单仓库 `30s` 超时与 `100 MB` 上限，以及 `~/.codelab/cache/git-packs/` 最多 20 包 / `500 MB` LRU 淘汰规则。

## 7. Phase F 自动化验证结果

通过确定性脚本核对 [ARCHITECTURE.md](./ARCHITECTURE.md) 与 [PRD.md](../prd/PRD.md)、[prd-stories.json](../prd/prd-stories.json) 及设计规范，全部检查均已通过：

- 18 条用户故事覆盖（`US-001` 至 `US-018`）：`18/18` 通过。
- 13 项统一本地埋点事件名称核对：`13/13` 通过。
- 34 项核心架构、安全与性能不变量核对：`34/34` 通过。
- `unslop`（无破折号连接）与文档标题层级（单一 H1、零跳级）及相对链接有效性核对：全部通过。

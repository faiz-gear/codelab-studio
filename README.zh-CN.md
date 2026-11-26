# CodeLab Studio

阅读[英文版 README](README.md)。

CodeLab Studio 是运行在 macOS、Windows 和 Linux 上的本地桌面编程实践工作台。它在一个窗口中打开 Markdown 实践手册或 Course Pack，并配套提供本地 Workspace、Monaco 代码编辑器、原生终端、无头 Exercise 验收与可选的本机 ACP 助教。

应用不需要账号、订阅或远端服务。在离线状态下，你仍然可以阅读手册、编辑代码、运行终端命令并完成 Exercise 验收。

## 核心能力

CodeLab Studio 将只读的 Course 内容与可写的本地代码目录分离。下文中的领域术语均以 [`GLOSSARY.md`](GLOSSARY.md) 为准。

- **分离只读 Course 与可写 Workspace。** Course 是只读的内容与规则源，Workspace 是绑定到某门 Course 的本地可写目录。同一门 Course 可以绑定多个 Workspace。首次绑定空目录时，CodeLab Studio 会把 Course 的 Starter 文件复制到该目录。Starter 文件包含初始代码与可见测试。绑定非空目录时，已有文件保持不变。
- **四种 Course 来源与内置创作。** 你可以打开内置 Course Pack、本地 Course Pack 目录、单文件 Markdown 或缓存后的 Git 仓库。你也可以在 **创作课程包** 中新建或编辑草稿并查看实时 Schema 诊断。点击 **试跑** 可在隔离的临时 Trial Workspace 中以学习者视角演练，不改动草稿源文件。
- **带冲突保护的编辑器与可选语言服务。** 编辑器基于 Monaco 并监听 Workspace 磁盘变化。当外部编辑器修改了未改动的文件时，CodeLab Studio 会自动重载该文件。如果当前存在未保存草稿，工作台会提示你 **加载磁盘版** 或 **保留草稿**。Reference Solution 是对应 Exercise 的目标代码快照。对比 Reference Solution 或 Starter 文件时会打开双列 Diff，只有你确认后才会写入磁盘。可选语言服务按当前打开文件最近的 Module Root 启动，也就是该文件所属的语言项目边界目录，支持 `gopls`、`typescript-language-server`、`pyright-langserver` 和 `jdtls`。语言服务未安装时，编辑器保持语法高亮。
- **原生终端与 Managed Daemon。** 交互终端在 Unix 上基于 `openpty`，在 Windows 上基于 ConPTY。CodeLab Studio 会把 `daemon` 类型的 Command Block 路由到具名服务终端，把 `oneshot` 类型的 Command Block 路由到空闲终端。Managed Daemon 是由工作台启动并追踪生命周期的常驻服务。验收运行前，如果章节声明了所需服务且尚未启动或源码已更新，工作台会自动启动或重启该 Managed Daemon。如果目标端口已被外部进程占用，工作台会报告 `DAEMON_PORT_CONFLICT`，不会终止外部进程。
- **无头 Exercise 验收。** 自动 Check 运行在 Unix 独立进程组或 Windows Job Object 中，不污染交互终端。支持的 Check 类型包括 `http_assert`、`command_test`、`test_suite` 和 `self_check`。语言适配器调用真实本地测试框架，包括 `go test -json -race`、`bun test` 或 Node、`pytest --json-report` 以及 `javac` 与 JUnit 5，并输出结构化的预期与实际 Diff。所有必选 Check 通过后，Exercise 标记为完成，且该完成态在后续修改中保持粘性，直到你显式重置进度。完整的 Check 规则与超时说明见 [`docs/verification.md`](docs/verification.md)。
- **隔离的 Managed Runtime。** **环境体检** 会并行探测本机工具链。当系统工具链缺失或版本过低时，CodeLab Studio 可以按需安装 Managed Runtime，你也可以在 **设置** 中开启 **优先托管版**。每个归档包均经过 SHA-256 校验并解压到隔离的 `runtimes/` 目录，不改动系统 `PATH`。支持的 Managed Runtime 包括 Bun `1.3.13`、Go `1.23.12`、Eclipse Temurin JDK `21.0.13.1+2` 与 JUnit `1.13.4`，以及 `uv` `0.11.6`、Python `3.12.13` 与 `pytest` `8.4.2`。
- **本机 ACP 助教。** **AI 助教** 通过 ACP 连接你本机已认证的编码 Agent CLI，ACP 即基于 stdio 的 JSON-RPC 2.0 Agent Client Protocol。内置预设包括 Claude Code 的 `claude --acp`、Gemini CLI 的 `gemini --acp`、Codex CLI 的 `codex --acp`、OpenCode 的 `opencode acp` 以及自定义命令。每个 Workspace 复用一个 Tutor Session。每次提问附带可逐项移除的 Context Snapshot，输入上限为 6,000 tokens，涵盖当前章节、活跃文件、近期终端报错和验收状态，并在装配前过滤 `.env`、`*secret*` 和 `*key*` 文件。助教支持苏格拉底式引导与直接解答两种风格。默认 Permission Policy 为 `guided_readonly`，自动放行 Workspace 内非敏感文件只读，对写入、命令执行和越界读取弹出审批卡。你可以在 **设置** 中切换为 `strict_ask_all`。v1 仅支持本机 ACP 助教，不包含外部 LLM API Key 配置，范围决策见 [ADR-0004](docs/adr/0004-v1-acp-only-ai-tutor.md)。
- **双语界面与外观。** 界面支持简体中文 `zh-CN` 和英文 `en-US`，默认使用简体中文。你可以切换浅色与深色主题，在 8 组强调色中选择，并将代码字号设置在 `11` 到 `20` px 之间。

## 内置 Course Pack

应用内置 4 套 Course Pack，共 31 个 Exercise。每套 Course Pack 都包含 Markdown 实践手册、Starter 文件、自动验收规则与 Reference Solution。

| Course ID | 语言 | Exercise 数量 | 目录 |
| --- | --- | --- | --- |
| `go-goframe-practice` | Go | 10 | [`builtin-courses/go-goframe-practice`](builtin-courses/go-goframe-practice) |
| `java-rate-limiter` | Java 21 | 7 | [`builtin-courses/java-rate-limiter`](builtin-courses/java-rate-limiter) |
| `python-data-clean` | Python | 6 | [`builtin-courses/python-data-clean`](builtin-courses/python-data-clean) |
| `ts-data-pipeline` | TypeScript | 8 | [`builtin-courses/ts-data-pipeline`](builtin-courses/ts-data-pipeline) |

运行以下命令可重新核对上述 Course ID、语言与 Exercise 数量：

```sh
node -e '
	const fs = require("fs");
	for (const dir of fs.readdirSync("builtin-courses").sort()) {
		const m = JSON.parse(fs.readFileSync(`builtin-courses/${dir}/codelab.json`, "utf8"));
		console.log(`${m.courseId}\t${m.language}\t${m.chapters.length}`);
	}
'
```

## 环境要求

从源码构建或运行测试前，先准备以下工具：

- Node.js 22 或更新版本，以及 `pnpm` 12。
- Rust stable 工具链，包含 `rustfmt` 和 `clippy`，支持 Rust edition 2024。
- Go 1.22 或更新版本，用于运行完整的 Rust 集成测试套件。
- 本机已安装的 ACP 代理 CLI，例如 `claude`、`gemini`、`codex` 或 `opencode`，用于启用 **AI 助教**。

在 Debian 或 Ubuntu 上构建前，先安装 Tauri 2 所需的系统库：

```sh
sudo apt-get update && sudo apt-get install -y --no-install-recommends \
	libwebkit2gtk-4.1-dev \
	libayatana-appindicator3-dev \
	librsvg2-dev \
	libssl-dev \
	patchelf
```

## 本地运行与验证

按以下步骤在开发模式下启动桌面应用：

1. 安装前端依赖：

```sh
pnpm install
```

2. 启动 Tauri 2 桌面窗口：

```sh
pnpm tauri dev
```

如果需要在本地测试时隔离应用数据，在启动前设置 `CODELAB_DATA_DIR`：

```sh
CODELAB_DATA_DIR=/tmp/codelab-studio-data pnpm tauri dev
```

使用以下命令打包或运行各类验证套件：

| 命令 | 用途 |
| --- | --- |
| `pnpm tauri build` | 为当前平台构建 Tauri 桌面安装包。 |
| `pnpm run check:all` | 依次运行 ESLint、Stylelint、`scripts/check-guards.mjs`、TypeScript 类型检查、`cargo fmt`、Vitest、Vite 构建、`cargo clippy` 与 `cargo test --workspace`。 |
| `pnpm test` | 运行 Vitest 单元测试与 `CoreClient` 边界测试。 |
| `pnpm run test:browser` | 运行 Playwright 浏览器边界测试，覆盖 Monaco、xterm.js、焦点与布局。 |
| `pnpm run check:rust` | 运行 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings` 与 `cargo test --workspace`。 |
| `pnpm run check:guards` | 运行架构纯净度、IPC 契约一致性、国际化与 CSS 令牌守卫检查。 |

首次运行 `pnpm run test:browser` 前，先安装 Playwright 的 Chromium 运行壳：

```sh
pnpm exec playwright install --with-deps chromium --only-shell
```

## Course Pack 目录格式

Course Pack 是包含标准 Markdown 手册以及可选元信息、Starter、验收规则和参考代码的目录。以下结构对应 `crates/codelab-core/src/course/pack.rs` 的解析契约：

- `codelab.json` 声明 `schemaVersion` `"1.0"`、`courseId`、`title`，以及可选的 `language`、`entryFile`、`starterDir`、`checksFile`、`dependencies` 和 `chapters`。
- `README.md` 或 `entryFile` 指定的路径存放 Markdown 实践手册。
- `starter/` 存放初始代码文件，仅在学习者绑定空目录时复制。
- `.codelab-checks.json` 按章节声明自动 Check 与可选的 `requiresDaemon` 常驻服务。
- `solutions/<chapterId>/` 存放可选的 Reference Solution 文件快照，用于双列 Diff 对照。

以下是一个最小化的 `codelab.json` 示例：

```json
{
	"schemaVersion": "1.0",
	"courseId": "example-go-lab",
	"title": "Example Go Lab",
	"language": "go",
	"entryFile": "README.md",
	"starterDir": "starter",
	"checksFile": ".codelab-checks.json",
	"chapters": [
		{
			"chapterId": "ex-1",
			"title": "Exercise 1: Return JSON",
			"kind": "exercise",
			"match": {
				"headingPrefix": "Exercise 1:"
			},
			"entryFile": "main.go"
		}
	]
}
```

没有 `codelab.json` 的单文件 Markdown 同样可以直接打开。CodeLab Studio 会按 H2 标题切分 Chapter，将 `bash`、`sh`、`zsh` 代码围栏识别为 Command Block，并使用 `ammonia` 清洗 HTML。

## 信任边界与本地存储

CodeLab Studio 不是操作系统或容器级进程沙箱。一旦你信任某门 Course 并运行命令或验收，派生进程会以你当前的本地用户权限运行。工作台在执行前提供三层应用级护栏，安全模型见 [ADR-0005](docs/adr/0005-trust-and-consent-guardrails.md)：

- **独立于 Workspace 的 Trust State。** 外部本地 Course 与 Git Course 在你显式授信前锁定为只读预览。信任记录保存在应用数据目录的 `trust.json` 中，Course Pack 无法自行伪造信任状态。
- **规范路径边界校验。** 所有文件读写都经过 `cap-std` 规范根目录校验，阻断 `..` 路径穿越与指向 Course Pack 或 Workspace 根目录之外的软链接。
- **结构化高危命令检查。** 任何 Command Block、验收命令或 ACP 命令执行前，都会经由 `tree-sitter-bash` 解析语法树。遇到无法解析的 Shell 语法，以及 `rm -rf`、`sudo`、`doas`、`mkfs`、`dd if=`、管道注入 Shell、`eval`、`source` 等破坏性命令时，必须由你显式确认后才会执行。

CodeLab Studio 在两处本地目录保存状态。所有 JSON 存储均采用原子写入，文件损坏时会将原文件隔离为 `.corrupt.*` 备份并恢复默认值：

- **应用数据目录。** CodeLab Studio 默认使用操作系统对应的 Tauri 应用数据目录，设置 `CODELAB_DATA_DIR` 后使用该目录。目录内保存 `settings.json`、`recents.json`、`trust.json`、`runtime-preferences.json` 和 `runtimes/`。
- **Workspace 元数据目录。** CodeLab Studio 将 Workspace 状态保存在 `<workspace>/.codelab/`，你可以把该目录加入 `.gitignore`。目录内保存 `workspace.json`、`progress.json`、`metrics.json`、`tutor-history.json` 和 `command-cwds.json`。

## 架构与仓库结构

`crates/codelab-core` 是唯一的领域真源，不依赖 Tauri，设计决策见 [ADR-0002](docs/adr/0002-rust-core-as-domain-authority.md)。`src-tauri` 仅把 `AppCommand`、`codelab://core-event` 事件流、原生文件选择器和退出清理钩子转发给 `codelab_core::App`。`src` 中的 React 前端只通过 `CoreClient` 与后端通信。

仓库将应用代码、内置课程、守卫脚本和文档组织在六个主要目录中：

| 路径 | 职责 |
| --- | --- |
| [`crates/codelab-core`](crates/codelab-core) | 脱离 Tauri 的 Rust 领域核心，负责课程解析、工作区、托管运行时、PTY 执行、无头验收、LSP、ACP 助教与持久化。 |
| [`src-tauri`](src-tauri) | 薄 Tauri 2 桌面外壳与 IPC 事件桥接层。 |
| [`src`](src) | 基于 React 19、TypeScript、Monaco、xterm.js 和 Tailwind CSS 的桌面界面。 |
| [`builtin-courses`](builtin-courses) | 四套内置 Course Pack，包含 Starter、验收规则和 Reference Solution。 |
| [`scripts`](scripts) | 位于 [`scripts/check-guards.mjs`](scripts/check-guards.mjs) 的仓库守卫脚本。 |
| [`docs`](docs) | 验收参考文档、9 份 ADR、视觉设计规范与历史 PRD。 |

运行以下命令可列出仓库中全部已跟踪的顶层目录：

```sh
git ls-tree -d --name-only HEAD
```

## 相关文档

下表列出领域词汇、架构决策、验收规则与编码规范。当早期产品文档与 ADR 存在冲突时，以 ADR 为准。

| 文档 | 内容范围 |
| --- | --- |
| [`GLOSSARY.md`](GLOSSARY.md) | 规范领域术语与避免使用的近义词。 |
| [`docs/adr/`](docs/adr/) | 9 份架构决策记录，编号 `0001` 至 `0009`。运行 `ls -1 docs/adr/*.md` 可列出全部文件。 |
| [`docs/verification.md`](docs/verification.md) | Check 配置结构、语言测试适配器、超时与 Managed Daemon 规则。 |
| [`CODING_STANDARDS.md`](CODING_STANDARDS.md) | 架构边界、崩溃安全持久化、错误本地化与界面工程规范。 |
| [`DESIGN.md`](DESIGN.md) | Arc UI 色彩令牌、字号排版、对比度要求与布局规范。 |
| [`PRODUCT.md`](PRODUCT.md) | 早期产品简报，仅作背景参考。 |

## 许可证

本仓库尚未添加 `LICENSE` 文件。

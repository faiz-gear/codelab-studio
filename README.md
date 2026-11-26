# CodeLab Studio

Read this document in [Simplified Chinese](README.zh-CN.md).

CodeLab Studio is a local desktop workbench for hands-on code practice on macOS, Windows, and Linux. It opens a Markdown manual or a Course Pack in one window. Beside the manual, it provides a local Workspace, a Monaco code editor, native terminals, headless Exercise verification, and an optional local ACP tutor.

CodeLab Studio requires no account, no subscription, and no remote server. You can read manuals, edit code, run terminal commands, and verify Exercises offline.

## Core capabilities

CodeLab Studio separates read-only Course content from your writable code directory. Each domain term below follows [`GLOSSARY.md`](GLOSSARY.md).

- **Separate Course and Workspace.** A Course is a read-only content source, and a Workspace is a writable local directory bound to one Course. One Course can bind to multiple Workspaces. When you bind an empty directory, CodeLab Studio copies the Course Starter files into that directory. Those Starter files contain initial source code and visible tests. Current files in a non-empty directory stay untouched.
- **Four Course sources and built-in authoring.** You can open built-in Course Packs, local Course Pack directories, single Markdown files, or cached Git repositories. You can also create or edit a Course Pack in **Author Course Pack** with live schema diagnostics. Select **Start trial** to test a draft in a Trial Workspace, an isolated temporary Workspace that leaves draft source files untouched.
- **Editor with conflict protection and optional language servers.** The editor uses Monaco and watches disk changes in your Workspace. If an external editor modifies a clean file, CodeLab Studio reloads that file automatically. With unsaved edits open, CodeLab Studio asks you to load the disk version or keep your draft. A Reference Solution is the target code snapshot for an Exercise. When you compare a Reference Solution or a Starter file, CodeLab Studio opens a side-by-side diff and writes to disk only after you confirm. Optional language servers start at the nearest Module Root, the language project directory for the open file. Supported servers are `gopls`, `typescript-language-server`, `pyright-langserver`, and `jdtls`. When a server is absent, the editor keeps syntax highlighting.
- **Native terminals and Managed Daemons.** Interactive terminals run through Unix `openpty` and Windows ConPTY. CodeLab Studio routes `daemon` Command Blocks to named service tabs and routes `oneshot` Command Blocks to idle terminals. A Managed Daemon is a long-lived service started by CodeLab Studio. Before verification runs, CodeLab Studio starts any required Managed Daemon or restarts it after source edits. If an external process holds the port, CodeLab Studio reports `DAEMON_PORT_CONFLICT` and does not stop that process.
- **Headless Exercise verification.** Automated Checks run in an isolated process group on Unix or a Job Object on Windows. Supported Check types are `http_assert`, `command_test`, `test_suite`, and `self_check`. Language adapters run `go test -json -race`, `bun test` or Node, `pytest --json-report`, and `javac` with JUnit 5, then report structured expected and actual diffs. When all required Checks pass, CodeLab Studio marks the Exercise complete. That completion stays sticky across later edits until you reset progress. See [`docs/verification.md`](docs/verification.md) for the Check schema and timeouts.
- **Managed Runtimes.** **Preflight** checks local toolchains in parallel. When a system toolchain is absent or too old, CodeLab Studio can install a Managed Runtime on demand. You can also enable **Prefer Managed Runtime** in **Settings**. CodeLab Studio verifies each archive by SHA-256 and extracts it into an isolated `runtimes/` directory. Your system `PATH` stays unchanged. Supported Managed Runtimes are Bun `1.3.13`, Go `1.23.12`, Eclipse Temurin JDK `21.0.13.1+2` with JUnit `1.13.4`, and `uv` `0.11.6` with Python `3.12.13` and `pytest` `8.4.2`.
- **Local ACP AI tutor.** The **AI Tutor** connects to a local agent over ACP, the Agent Client Protocol over stdio JSON-RPC 2.0. Built-in presets are Claude Code with `claude --acp`, Gemini CLI with `gemini --acp`, Codex CLI with `codex --acp`, OpenCode with `opencode acp`, and a custom command. Each Workspace reuses one Tutor Session. Each prompt attaches a removable Context Snapshot capped at 6,000 input tokens across the current Chapter, the active file, recent terminal errors, and Check Results. CodeLab Studio filters `.env`, `*secret*`, and `*key*` files before prompt assembly. You can switch between **Socratic Guidance** and **Direct Answer** styles. The default `guided_readonly` Permission Policy allows non-sensitive reads inside the Workspace and asks approval for writes, commands, and external reads. You can switch to `strict_ask_all` in **Settings**. Version 1 supports local ACP agents only and omits external LLM API keys. See [ADR-0004](docs/adr/0004-v1-acp-only-ai-tutor.md) for that scope decision.
- **Bilingual interface and appearance.** The interface supports Simplified Chinese as `zh-CN` and English as `en-US`. Simplified Chinese is the default locale. You can switch between Light and Dark themes, choose from 8 accent presets, and set code font sizes from `11` to `20` px.

## Built-in Course Packs

The binary embeds 4 Course Packs with 31 Exercises in total. Each Course Pack ships a Markdown manual, Starter files, automated Checks, and Reference Solutions.

| Course ID | Language | Exercises | Directory |
| --- | --- | --- | --- |
| `go-goframe-practice` | Go | 10 | [`builtin-courses/go-goframe-practice`](builtin-courses/go-goframe-practice) |
| `java-rate-limiter` | Java 21 | 7 | [`builtin-courses/java-rate-limiter`](builtin-courses/java-rate-limiter) |
| `python-data-clean` | Python | 6 | [`builtin-courses/python-data-clean`](builtin-courses/python-data-clean) |
| `ts-data-pipeline` | TypeScript | 8 | [`builtin-courses/ts-data-pipeline`](builtin-courses/ts-data-pipeline) |

Run this command to regenerate the Course Pack IDs, languages, and Exercise counts:

```sh
node -e '
	const fs = require("fs");
	for (const dir of fs.readdirSync("builtin-courses").sort()) {
		const m = JSON.parse(fs.readFileSync(`builtin-courses/${dir}/codelab.json`, "utf8"));
		console.log(`${m.courseId}\t${m.language}\t${m.chapters.length}`);
	}
'
```

## Prerequisites

Install these tools before you build or test CodeLab Studio from source:

- Node.js 22 or newer and `pnpm` 12.
- The Rust stable toolchain with `rustfmt` and `clippy` for Rust edition 2024.
- Go 1.22 or newer for the full Rust integration test suite.
- A local ACP agent CLI, such as `claude`, `gemini`, `codex`, or `opencode`, for the **AI Tutor**.

On Debian or Ubuntu, install the Tauri 2 system libraries before you build:

```sh
sudo apt-get update && sudo apt-get install -y --no-install-recommends \
	libwebkit2gtk-4.1-dev \
	libayatana-appindicator3-dev \
	librsvg2-dev \
	libssl-dev \
	patchelf
```

## Run and verify locally

Follow these steps to start the desktop app in development mode:

1. Install frontend dependencies:

```sh
pnpm install
```

2. Launch the Tauri 2 desktop window:

```sh
pnpm tauri dev
```

To isolate app data for a local run, set `CODELAB_DATA_DIR` before you launch:

```sh
CODELAB_DATA_DIR=/tmp/codelab-studio-data pnpm tauri dev
```

Use these commands to build the desktop bundle and run verification suites:

| Command | Purpose |
| --- | --- |
| `pnpm tauri build` | Build the Tauri desktop bundle for the current platform. |
| `pnpm run check:all` | Run ESLint, Stylelint, `scripts/check-guards.mjs`, TypeScript typecheck, `cargo fmt`, Vitest, Vite build, `cargo clippy`, and `cargo test --workspace`. |
| `pnpm test` | Run Vitest unit and `CoreClient` seam tests. |
| `pnpm run test:browser` | Run Playwright browser seam tests for Monaco, xterm.js, focus, and layout. |
| `pnpm run check:rust` | Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. |
| `pnpm run check:guards` | Run architecture, IPC contract parity, i18n, and CSS token guards. |

Before you run `pnpm run test:browser` for the first time, install the Playwright Chromium shell:

```sh
pnpm exec playwright install --with-deps chromium --only-shell
```

## Course Pack format

A Course Pack is a directory that pairs a standard Markdown manual with optional metadata, Starter files, Checks, and Reference Solutions. The layout below matches `crates/codelab-core/src/course/pack.rs`:

- `codelab.json` declares `schemaVersion` `"1.0"`, `courseId`, `title`, optional `language`, `entryFile`, `starterDir`, `checksFile`, `dependencies`, and `chapters`.
- `README.md`, or the path set in `entryFile`, holds the Markdown practice manual.
- `starter/` holds initial Workspace files copied only when a learner binds an empty directory.
- `.codelab-checks.json` declares automated Checks and optional `requiresDaemon` services per Chapter.
- `solutions/<chapterId>/` holds optional Reference Solution file snapshots for side-by-side diffs.

Here is a minimal `codelab.json` manifest:

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

A single Markdown file also opens without `codelab.json`. CodeLab Studio splits Chapters at H2 headings, turns `bash`, `sh`, and `zsh` fenced blocks into Command Blocks, and sanitizes HTML with `ammonia`.

## Trust boundaries and local storage

CodeLab Studio is not an OS or container sandbox. Once you trust a Course and run commands or Checks, spawned processes run with your local user permissions. Three application guardrails apply before execution. See [ADR-0005](docs/adr/0005-trust-and-consent-guardrails.md) for the security model:

- **Trust State outside Workspaces.** External local Courses and Git Courses open in read-only preview until you trust them. Trust records persist in `trust.json` inside the app data directory so a Course Pack cannot forge its own trust.
- **Canonical path containment.** Every file read and write goes through `cap-std` canonical root checks that block `..` traversal and symlinks outside the Course Pack or Workspace root.
- **Structured command inspection.** Before any Command Block, Check command, or ACP command runs, `tree-sitter-bash` parses the shell AST. CodeLab Studio asks you to confirm unparseable shell syntax and destructive commands such as `rm -rf`, `sudo`, `doas`, `mkfs`, `dd if=`, piped shells, `eval`, and `source`.

CodeLab Studio stores state in two local locations. Every JSON store writes atomically and quarantines corrupt files under `.corrupt.*` backups on recovery:

- **App data directory.** CodeLab Studio uses the OS Tauri app data directory by default, or the path in `CODELAB_DATA_DIR` when set. That directory holds `settings.json`, `recents.json`, `trust.json`, `runtime-preferences.json`, and `runtimes/`.
- **Workspace metadata directory.** CodeLab Studio stores Workspace state in `<workspace>/.codelab/`, and you can add that directory to `.gitignore`. That directory holds `workspace.json`, `progress.json`, `metrics.json`, `tutor-history.json`, and `command-cwds.json`.

## Architecture and repository layout

`crates/codelab-core` is the single domain authority and has no dependency on Tauri. See [ADR-0002](docs/adr/0002-rust-core-as-domain-authority.md). `src-tauri` forwards `AppCommand` calls, `codelab://core-event` events, native dialogs, and shutdown hooks directly to `codelab_core::App`. The React frontend in `src` communicates with the backend only through `CoreClient`.

The repository organizes application code, built-in Course Packs, verification guards, and documentation into six main directories:

| Path | Role |
| --- | --- |
| [`crates/codelab-core`](crates/codelab-core) | Tauri-free Rust domain core for Courses, Workspaces, Managed Runtimes, PTY execution, verification, LSP, ACP tutor, and storage. |
| [`src-tauri`](src-tauri) | Thin Tauri 2 desktop shell and IPC event bridge. |
| [`src`](src) | React 19, TypeScript, Monaco, xterm.js, and Tailwind CSS desktop interface. |
| [`builtin-courses`](builtin-courses) | Four embedded Course Packs with Starter files, Checks, and Reference Solutions. |
| [`scripts`](scripts) | Repository guard runner in [`scripts/check-guards.mjs`](scripts/check-guards.mjs). |
| [`docs`](docs) | Verification reference, 9 ADRs, design specification, and PRD archive. |

Run this command to list every top-level tracked directory in the repository:

```sh
git ls-tree -d --name-only HEAD
```

## Documentation

The documents below define the domain vocabulary, architecture decisions, verification rules, and engineering standards. When earlier product briefs differ from the ADRs, follow the ADRs.

| Document | Scope |
| --- | --- |
| [`GLOSSARY.md`](GLOSSARY.md) | Canonical domain terms and avoided synonyms. |
| [`docs/adr/`](docs/adr/) | Nine Architecture Decision Records, `0001` through `0009`. Run `ls -1 docs/adr/*.md` to list them. |
| [`docs/verification.md`](docs/verification.md) | Check schema, language test adapters, timeouts, and Managed Daemon rules. |
| [`CODING_STANDARDS.md`](CODING_STANDARDS.md) | Architecture seams, crash-safe persistence, error localization, and UI craft rules. |
| [`DESIGN.md`](DESIGN.md) | Arc UI color tokens, typography, contrast targets, and layout rules. |
| [`PRODUCT.md`](PRODUCT.md) | Early product brief kept for background context. |

## License

No `LICENSE` file exists in this repository yet.

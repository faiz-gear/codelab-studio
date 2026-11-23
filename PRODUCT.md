# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

The desktop shell uses Tauri 2 with Rust, React, TypeScript, Vite, and Tailwind CSS. The target platforms are macOS, Windows, and Linux. The shell opens at `1440×900` by default and supports a minimum of `1024×720`. Monaco Editor, xterm.js, and `portable-pty` are planned for the workbench.

## Users

- **Primary — Advancing self-learners (1–3 years development experience):** Working through long-form hands-on programming manuals and tutorials. Today they switch windows and copy-paste 20–35 times per exercise across docs, IDE, and terminal, lose momentum on local environment setup, and have to assemble debugging context manually when stuck. They need clear entry, run-in-place execution, contextual error help, and unambiguous pass/fail verification evidence.
- **Secondary — Course-pack authors (tech bloggers / Tech Leads):** Distributing practical tutorials to readers across heterogeneous machines without building or operating a proprietary hosted learning platform. They need to author in standard Markdown plus a lightweight `.codelab-checks.json` specification.

## Product Purpose

CodeLab Studio turns any standard Markdown practice handbook or local/Git course pack into a unified, local-first desktop programming workbench combining:

1. A numbered 3-step guided manual (`① Start service / run tests → ② Edit code → ③ Automated verification`),
2. A sandboxed local code workspace with live disk sync and safe-write Diff protection,
3. Isolated managed multi-language runtimes with daemon/oneshot multi-terminal routing and automated function/HTTP grading, and
4. A cross-pane, selection-aware AI tutor supporting both BYOK LLM providers and local ACP coding agents.

**90-day public-beta success targets** (targets from `docs/prd/PRD.md`, not measured results):

- Per-exercise window switches and manual copy/paste vs. plain documentation: **≥ 80% reduction**.
- 14-day completion rate on the built-in Go/GoFrame handbook (completing ≥ Exercise 7): **≥ 45%**.
- One-click terminal and grading dispatch reliability (excluding learner code errors): **≥ 98%**.
- Single-turn resolution rate on selection/error AI queries: **≥ 70%**.
- Zero-modification parse rate on external standard Markdown tutorials: **≥ 90%**.

## Positioning

- **Strictly local-first and open-source:** Zero login, zero subscription, zero mandatory cloud dependency; docs, editing, multi-terminal execution, and automated grading remain 100% functional offline and without AI configured.
- **Zero-lock-in Markdown & portable Course Packs:** Standard Markdown tutorials work out of the box (`#` course title, `##` stable chapter IDs, `bash/sh/zsh` executable command inference) and upgrade incrementally with `codelab.json`, `starter/`, and `.codelab-checks.json` (schema v1.0) while remaining readable as ordinary Markdown.
- **Managed 4-language runtimes + real multi-terminal engineering labs:** Isolates JS/TS, Go, Java, and Python runtimes under `~/.codelab/runtimes/` without polluting global `PATH`, while cleanly separating long-lived server daemons from one-shot test/curl terminals—covering both function-level grading and real multi-service backend exercises.
- **Dual-protocol, consent-first AI tutoring (BYOK + ACP):** Pairs OS Keychain-backed BYOK LLM providers (including local Ollama) with **Agent Client Protocol (ACP, JSON-RPC 2.0 over stdio)** to connect local coding agents (Claude Code, Gemini CLI, Codex CLI, OpenCode, or custom `--acp` binaries) under explicit workspace read/write/exec permission approval, defaulting to Socratic guidance.

## Operating Context

- **Environment:** Desktop engineering workstation (macOS, Windows, Linux) frequently used alongside external IDEs (VS Code, Cursor, GoLand, IntelliJ IDEA) editing the same local workspace folder on disk.
- **Core learner workflow:** Open app → continue previous exercise, pick a 4-language course, or import/author a local or Git course pack → bind a local workspace and run environment preflight (installing missing runtimes into `~/.codelab/runtimes/` on demand) → work through the numbered 3-step loop (`① Start/Test → ② Edit with safe-write Diff → ③ Automated checks`) → highlight any manual text, code, or terminal output (`≥ 2` chars) to summon contextual AI help → pass all automated checks to advance progress atomically in `<workspace>/.codelab/progress.json`.
- **Canonical on-disk structures:**
  - Course Pack: `codelab.json`, `README.md`, `starter/`, `.codelab-checks.json`.
  - Workspace metadata (hidden, recommended for `.gitignore`): `<workspace>/.codelab/workspace.json`, `progress.json`, `metrics.json`.
  - Managed runtimes: `~/.codelab/runtimes/`.

## Capabilities and Constraints

- **v1 Languages, runtimes, grading, and optional LSPs:**
  - **JS / TS:** Portable Bun (or system Node ≥ 20), `bun test` / built-in TS runner, optional `tsserver`, 5s timeout.
  - **Go:** Portable `go1.23`, `go test -json -race ./...` + PTY `go run .`, optional `gopls`, 10s timeout.
  - **Java:** Portable Eclipse Temurin JDK 21, `javac` + JUnit 5 Console Launcher, optional `jdtls`, 12s timeout.
  - **Python:** Single-binary `uv` managing Python 3.12, `pytest --json-report`, optional `pyright`, 8s timeout.
  - Optional LSPs resolve nearest module root per active file (supporting multi-module repos such as `task-api/` and `task-api-gf/`) and degrade gracefully to syntax highlighting when absent or unhealthy.
- **Localization:** Bilingual UI supporting **Simplified Chinese (`zh-CN`)** and **English (`en-US`)**.
- **Workspace & editor constraints:**
  - Strict canonical-path sandboxing inside `workspaceRoot` (blocking `../` traversal and out-of-root symlinks).
  - Disk-to-file-tree sync ≤ 500ms; reject text loading for binary files or files > 2 MB; Markdown handbook limit 5 MB with strict XSS sanitization.
  - External IDE conflict protection: silently reload unmodified buffers; prompt `Load Disk` vs. `Keep Draft` when local edits exist.
  - Manual reference snippets never silently overwrite learner files—always confirm through side-by-side Diff merge.
- **Terminal & grading constraints:**
  - Strict routing isolation: `daemon` commands spawn named service tabs (live pulse, SIGINT stop, restart); `oneshot` commands target idle terminals only and never inject text into running service tabs.
  - Detect port conflicts (`address already in use`) with recovery actions; retain 5,000 lines of UI scrollback and a 200-line ring buffer for AI error context.
  - Isolated process-group / Windows Job Object execution with `SIGINT → SIGKILL` timeout enforcement and 100% PTY/LSP child cleanup on workspace switch or exit.
- **AI, ACP & privacy constraints:**
  - Cross-pane selection trigger (`≥ 2` chars, ≤ 150ms): Manual (`Explain concept`, `Ask`), Editor (`Explain code`, `Find issue`, `Ask`), Terminal (`Diagnose error`, `Ask`).
  - Transparent, removable 4D Context Chips (chapter, active file snippet, terminal error tail, check state) with a hard **6,000 input token** ceiling that never truncates user selection or the latest error tail; automatic filtering of `.env`, `*secret*`, and `*key*` files.
  - BYOK API keys stored exclusively in OS Keychain / Credential Manager / Secret Service, never written to workspace files or logs.
  - ACP defaults to Guided Read-only: workspace writes and command execution require explicit user approval per request via in-thread approval cards.
  - Socratic guidance (default) and Direct answer mode toggle with streaming output and stop control.
  - Local-only metrics persisted to `<workspace>/.codelab/metrics.json` (13 unified events); any anonymized telemetry is strictly opt-in and never transmits source code, commands, file paths, chat transcripts, or keys.
- **Security guardrails:**
  - No silent automatic execution of command blocks.
  - Destructive shell patterns (`rm -rf`, `sudo`, `mkfs`, `dd if=`, `chmod -R 777 /`, `curl | sh`) require explicit confirmation showing full working directory and command with default focus on Cancel.
  - External and Git course packs require first-open trust confirmation.
- **Performance & reliability budgets:**
  - Cold start ≤ 2.0s; 1,000-line Markdown parse ≤ 150ms; file tree sync ≤ 500ms; selection pill ≤ 150ms; high-throughput terminal ≥ 30 FPS; resident memory (excluding LSP children) ≤ 350 MB.
  - Atomic temp-file + rename persistence with corruption recovery for `.codelab/*.json`.
- **Explicitly undecided product facts:**
  - **Open-source license:** Not yet selected (no `LICENSE` file in repository).
  - **Distribution & release channels:** Packaging, code-signing, auto-update mechanism, and release channels (e.g., GitHub Releases, Homebrew) are not yet decided.
  - **Bilingual defaults:** Default first-launch locale detection and language-switcher placement/persistence details between `zh-CN` and `en-US` are not yet specified.

## Brand Commitments

- **Product name:** CodeLab Studio.
- **Product stance & voice:** Calm, focused, transparent, and developer-respecting; prioritizes clear entry and progressive disclosure over dense dashboards or decorative statistics.
- **Existing repository design artifacts:** A v3.1 design-system specification and interactive HTML prototype already exist under `docs/design/` (`design-system.md`, `tokens.css`, `tailwind.config.js`, `design-review.md`, `prototype/index.html`, and `screenshots/`). Recorded here as existing project context without prescribing visual details in `PRODUCT.md`.

## Evidence on Hand

- **Current implementation and evidence.**
  - `README.md` — Project overview and documentation index.
  - `docs/prd/PRD.md` — Confirmed v3.1 Product Requirements Document.
  - `docs/prd/prd-stories.json` — 18 structured engineering user stories (`US-001`–`US-018`).
  - `docs/design/design-system.md`, `docs/design/tokens.css`, `docs/design/tailwind.config.js`, `docs/design/design-review.md`, `docs/design/prototype/index.html`, and `docs/design/screenshots/01-welcome-light.png` through `04-workbench-dark.png` — Existing v3.1 design specification, interactive prototype, and browser prototype screenshots.
  - `src/` and `src-tauri/` contain the desktop shell, local persistence, and course backend. `builtin-courses/go-goframe-practice/` contains the Go/GoFrame handbook with Exercises 0–9.
- **Pending work.**
  - The desktop course preview, trust confirmation, and workspace binding journeys remain pending. JS/TS, Python, and Java demo course packs are planned.
  - No `LICENSE` file exists yet.
  - No customer testimonials, adoption counts, press quotes, or real user telemetry exist yet (the PRD 90-day figures are targets, not achieved metrics).

## Product Principles

1. **Local-first and offline-complete:** Core reading, coding, multi-terminal execution, and automated grading must function 100% locally without accounts, subscriptions, or network access; AI enhances rather than gates the workbench.
2. **Zero-lock-in open content:** Standard Markdown handbooks work immediately without proprietary lock-in and upgrade cleanly into portable Course Packs that stay useful anywhere.
3. **Progressive disclosure over clutter:** Keep the primary entry and 3-step learning flow immediately legible, revealing secondary workspaces (file tree, auxiliary commands, reflections, AI tutor) only when needed.
4. **Explicit consent at every boundary:** Never silently overwrite learner code, silently execute destructive commands, expose local secrets to AI prompts, or allow external agents to write or run commands without explicit approval.
5. **Verifiable progress over guesswork:** Ground learning feedback in deterministic automated checks, side-by-side expected-vs-actual diffs, and inspectable, removable AI context chips.

## Accessibility & Inclusion

- **Contrast:** WCAG 2.1 AA (≥ 4.5:1) across both Light and Dark themes.
- **Keyboard & assistive support:** Full keyboard navigation and shortcuts (`⌘/Ctrl + B`, `⌘/Ctrl + S`, `Esc`), visible `:focus-visible` indicators, skip-navigation links, `aria-live="polite"` status announcements, and `prefers-reduced-motion` support.
- **Language inclusion:** Bilingual `zh-CN` and `en-US` UI support.

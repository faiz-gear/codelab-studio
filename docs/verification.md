# Exercise verification

Course Packs declare Checks in `.codelab-checks.json` (`schemaVersion: "1.0"`). Each chapter may declare `requiresDaemon: { label, port, serviceKey }`; its matching daemon Command Block supplies the command and Workspace directory. Verification starts that Managed Daemon when needed, restarts it after source saves, and waits up to 30 seconds for its loopback port. An external listener produces `DAEMON_PORT_CONFLICT` and is never terminated.

Supported Checks:

- `http_assert`: `request` accepts `url`, `method` (default `GET`), string `headers`, and a string `body`. `expect` accepts `status` (default `200`), exact string `headers`, `bodyContains`, and `bodyEquals`. Requests do not follow redirects or use the host's HTTP proxy.
- `command_test`: requires `command`; optionally declares `cwd`, `expectExitCode` (default `0`), and `forbidOutput` (literal substrings).
- `test_suite` runs visible Workspace tests through the framework adapter for the loaded Course language. Go is supported and defaults to `go test -json -race ./...`. Custom commands must emit the same framework report. To pass, a suite must produce a complete report with at least one passing case and no failing cases. Every entry in `requireTests` must match a passing case. Use a package-qualified name such as `package/path/TestName` when a short name is ambiguous. If a package path and a subtest path produce the same joined name, use `package/path::TestName/Subtest` to specify both exactly. A missing adapter produces an error for that Check and the other Checks can still run.
- `self_check`: a learner checkbox; optional unless `required: true`.

Automatic Checks accept `timeoutSec` (1–600). Defaults are 10 seconds for HTTP, 120 for Go/Java commands, and 30 for other commands. Captured command/HTTP output is limited to 2 MiB. Command output is captured independently of interactive PTYs. A timed-out Check retains its timeout status even if its report is incomplete. Timeout and Workspace teardown interrupt and then kill the owned process group (Unix) or Job Object (Windows).

`AppCommand::RunChecks` performs the Trust Gate, checks every command (including daemon startup) for high-risk shell patterns, and applies revision-guarded editor drafts before launching a background task. `ProgressUpdated` events expose running and final Check Results. `GetProgress` retrieves the same snapshot. `SetSelfCheck`, `CompleteExercise` (only when no automatic Checks exist), `ResetProgress`, and `ExportLearningRecord` are Workspace-scoped operations.

The Workspace's atomic `.codelab/progress.json` stores `completionState`, `lastCheckResults`, and `selfChecks` separately per stable Chapter id. All automatic Checks plus required Self-Checks must pass to complete an Exercise. Completion stays sticky until an explicit reset. Reset preserves learner code; export returns a JSON learning record for a local download. Interrupted running results are recovered as failures on reopen, preserving any earlier completion.

Verification is covered through the `App` and `CoreClient` seams in `verification_seam.rs` and `verification.seam.test.tsx`, with a browser check in `verification.browser.ts`.

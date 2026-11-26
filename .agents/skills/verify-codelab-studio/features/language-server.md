# Check Module Root language support in Monaco

Use the restored built-in Go Workspace. Go and a compatible `gopls` must be visible on the launcher's PATH to exercise the ready state. Record the Go LSP row in [Preflight](runtime-preflight.md).

1. Open **文件** → **task-api** → **main.go**. Capture the Go LSP status in Monaco's status bar. Click the status to confirm it opens Settings on **四语言运行时**, then close Settings.
2. With LSP ready, hover over `http.NewServeMux` and capture documentation. On macOS, focus the identifier and use `⌘K`, then `⌘I` to open hover from the keyboard. Replace `NewServeMux()` with `NewSer` and trigger completion with `⌘I` on macOS or `Ctrl+Space` on Linux and Windows. Replace it with `MissingVerifySymbol()` and capture the diagnostic. Undo the edits and confirm the status returns to saved. Switch to `task-api-gf/main.go` and capture hover on the local `configure` call when the second module is ready.
3. If the status instead reports a missing or degraded server, capture it and the Preflight tool status. Confirm Monaco still accepts an edit and Undo. Record the exact tool or setup required to reach ready.

[Editor integration](../../../../src/workspace/monacoLanguage.ts) · [Module Root resolution](../../../../crates/codelab-core/src/lsp/mod.rs)

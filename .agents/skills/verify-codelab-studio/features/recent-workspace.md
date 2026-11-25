# Resume the same Workspace after restarting

The welcome page remembers the last bound Workspace across a full desktop app restart.

## Sub-features

- Locate the previous Workspace on the welcome page.
- Open its Starter source and observe the persisted edit.

## How to get to it (user POV)

Use the matching row under **最近的 Workspace** (**Recent Workspaces**). The primary **继续上次 Workspace** (**Resume Workspace**) button also opens the latest Workspace.

## Driving it with the desktop window

- Start the second app process with the same data directory after saving the edit in the [Workspace journey](workspace-save.md). Confirm the row lists the original temporary Workspace path. Capture `07-recent.png`.
- Click that row and check the Workspace path in the workbench. Open **文件** (**Files**) → **task-api** → **main.go**.
- Confirm Monaco shows the exact `VERIFY_LINE` on line 1 and the status bar shows `task-api/main.go`. Capture `08-reopened.png` before making any further edits.

## Gotchas

- Match the full Workspace path in the row. Course titles can be shared by multiple Workspaces.
- File tabs may start closed. Reopen the file from **Files** before checking the editor.

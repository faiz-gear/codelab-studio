# Run and stop course commands

## Managed Daemon

Use a bound scratch Workspace in the built-in Go / GoFrame course with Go ready in Preflight. Check that port 8080 is free before starting the exercise service.

1. In Exercise 1 under **启动 / 测试**, find the `go run .` block with working directory `task-api` and click its **运行命令** button. Exercise 1 also has a root-directory command with the same button label. Select **终端** and the **Managed Daemon** tab if needed. Capture the listening message.
2. Check that `curl -fsS http://127.0.0.1:8080/healthz` returns `{"status":"ok"}`. Click **停止（中断）** in the terminal and confirm the listener is gone. Record both checks.

## Command confirmation

1. After returning to the disposable [Go draft](authored-trial.md), replace `go run main.go` in the first command block with `sh -c 'printf desktop-guard-ok'`. Click **试跑** to create a fresh Trial Workspace, then click **运行命令**. Capture **确认高危命令** with the exact command and Trial Workspace path.
2. Click **取消** and confirm there are zero terminal tabs. Run the same block again and click **仍要运行**. Capture the terminal marker and exit code 0, then return to the draft to close the Trial Workspace.

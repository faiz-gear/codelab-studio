# Run and stop course commands

## Managed Daemon

Use a bound scratch Workspace in the built-in Go / GoFrame course with Go ready in Preflight. Check that port 8080 is free before starting the exercise service.

1. In Exercise 1, click **运行命令** on the `go run .` block with working directory `task-api`. Capture the Managed Daemon terminal showing the listening message.
2. Check that `curl -fsS http://127.0.0.1:8080/healthz` returns `{"status":"ok"}`. Click **停止（中断）** in the terminal and confirm the listener is gone. Record both checks.

## Command confirmation

1. In a disposable [Go draft](authored-trial.md), replace `go run main.go` in the first command block with `sh -c 'printf desktop-guard-ok'`. Click **试跑** and then **运行命令**. Capture **确认高危命令** with the exact command and Trial Workspace path.
2. Click **取消** and confirm the terminal stays closed. Run the same block again and click **仍要运行**. Capture the terminal marker and successful exit.

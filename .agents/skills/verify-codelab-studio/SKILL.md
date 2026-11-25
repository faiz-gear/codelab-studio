---
name: verify-codelab-studio
description: Verify CodeLab Studio through its real Tauri desktop window. Use for desktop acceptance checks of courses, Workspace editing and resume, Reference Solutions, command execution, runtime setup, Git import, and authored Trial Workspaces.
---

# Verify CodeLab Studio

Run the [feature map](features/README.md) from the repo root. Use the Tauri window and the operating system's folder picker for UI actions. The [Playwright browser fixture](../../../src/__tests__/browser/README.md) runs Vite on port 1421 with `FakeCoreClient`. Its selectors and keyboard actions can guide desktop interaction. Desktop evidence comes from the native window and the Workspace on disk.

## Launch

Use an unlocked macOS graphical session with pnpm, Rust and the Tauri build prerequisites. Install missing frontend dependencies with `pnpm install --frozen-lockfile`. Use a desktop driver with window capture and input access, or operate the window directly. On macOS, an automated driver needs Accessibility access for interaction and Screen Recording access for capture. Keep the driver focused on the window owned by this run.

Check the macOS console lock before starting a launch. If this prints the lock flag, record it and report the run as blocked until the console is unlocked.

```sh
ioreg -n Root -d1 | grep -oE '\{[^{}]*"kCGSSessionOnConsoleKey"=Yes[^{}]*\}' |
  grep -oF '"CGSSessionScreenIsLocked"=Yes'
```

Before launching, check `lsof -nP -iTCP:1420 -sTCP:LISTEN`. Proceed when there is no listener. The configured dev URL is fixed at `http://localhost:1420/`, so use the port for one verification run at a time.

From the repo root, run these commands in a shell that you can reuse for the second launch. The launch command runs in the foreground.

```sh
mkdir -p test-results
VERIFY_EVIDENCE=$(mktemp -d "$PWD/test-results/verify-codelab-studio.XXXXXX") &&
VERIFY_TMP=$(mktemp -d /tmp/codelab-studio-verify.XXXXXX) &&
mkdir "$VERIFY_TMP/data" "$VERIFY_TMP/workspace" &&
test -z "$(ls -A "$VERIFY_TMP/workspace")" &&
printf 'checkout=%s\nstarted=%s\nscratch=%s\nevidence=%s\n' \
  "$(git rev-parse HEAD)" "$(date -u +%FT%TZ)" "$VERIFY_TMP" "$VERIFY_EVIDENCE" \
  > "$VERIFY_EVIDENCE/run.txt" &&
cat "$VERIFY_EVIDENCE/run.txt"
```

```sh
launch_verify() {
  CODELAB_DATA_DIR="$VERIFY_TMP/data" pnpm tauri dev --no-watch \
    > "$VERIFY_EVIDENCE/launch-$1.log" 2>&1
}
launch_verify 1
```

Tauri starts Vite through `beforeDevCommand` and builds the native shell. The launch log shows Vite and the `Running` line for the checkout's `codelab-studio` binary. Wait for its **CodeLab Studio** window to show the welcome page and built-in course button. On the second launch, expect the welcome page and a recent Workspace with the temporary path. [Tauri config](../../../src-tauri/tauri.conf.json) · [Tauri entry point](../../../src-tauri/src/lib.rs)

## Doctor

Run from a second shell if the window is missing or before driving either launch. Use the scratch and evidence paths printed by setup in that shell. Use the app PID reported by your desktop driver, or find it from the binary path printed in the launch log. Record the output in `doctor-1.log` or `doctor-2.log`.

```sh
ps -p "$VERIFY_PID" -o pid=,ppid=,pgid=,command=
ps eww -p "$VERIFY_PID" | grep -oF -- "CODELAB_DATA_DIR=$VERIFY_TMP/data"
lsof -nP -iTCP:1420 -sTCP:LISTEN
curl -fsS -o /dev/null -w 'HTTP %{http_code}\n' http://localhost:1420/
```

Confirm the app executable is the one in the launch log. Use the listener PID reported by `lsof` in `ps -p PID -o pid=,ppid=,pgid=,command=` and match it to the same launch process tree or group. Record both PIDs for teardown. Confirm the expected welcome controls in the native window before reporting readiness. Keep the launch log and report the last observed step if boot, desktop input or capture is blocked.

If input or capture fails while the app and listener checks pass, repeat the [console lock check](#launch) and record its result in the Doctor log.

## Drive and evidence

For a full pass, start with the [save and restart journey](features/workspace-save.md), then follow the remaining recipes in the [feature map](features/README.md). Save screenshots, file assertions, process details and launch logs to the per-run directory printed as `evidence`. Record the chosen entry point, visible outcome, and each failed or blocked assertion in `run.txt`. Each screenshot must show this run's app window or the native picker it opened. For macOS window capture, use `screencapture -x -l "$VERIFY_WINDOW_ID" "$VERIFY_EVIDENCE/01-welcome.png"`. For the picker, use its window ID or capture the app window with the attached picker visible.

## Cleanup

After each launch, quit the verified app through the OS (on macOS, `⌘Q` with its window active). Wait for the launch command to return. If it remains active, use Ctrl+C in its launch terminal. Check that the recorded app and Vite PIDs are gone and that this run no longer owns port 1420. For a surviving process, verify its recorded PID, executable and launch ownership before sending `kill -TERM PID`.

Keep the scratch directory across the two launches. After the last launch or a blocked attempt, remove the scratch directory recorded by setup. Git import can leave read-only cache directories, so restore owner write access inside that scratch cache before removal. Verify the evidence separately.

```sh
case "${VERIFY_TMP:-}" in
  /tmp/codelab-studio-verify.??????)
    if [ -d "$VERIFY_TMP" ] && [ ! -L "$VERIFY_TMP" ] &&
      grep -qxF -- "scratch=$VERIFY_TMP" "$VERIFY_EVIDENCE/run.txt"
    then
      if [ -d "$VERIFY_TMP/data/git-courses" ]; then
        find "$VERIFY_TMP/data/git-courses" -type d -exec chmod u+w {} +
      fi
      find "$VERIFY_TMP" -depth -delete
    fi
    ;;
  *) printf 'Unexpected scratch path: %s\n' "${VERIFY_TMP:-}" >&2 ;;
esac
test ! -e "$VERIFY_TMP" &&
test -r "$VERIFY_EVIDENCE/run.txt" &&
test -r "$VERIFY_EVIDENCE/launch-1.log" &&
ls -l "$VERIFY_EVIDENCE"
```

Open the retained screenshots and assertion log (or the blocker diagnostics) after cleanup. Use [`$maintain-verification-skill`](../maintain-verification-skill/SKILL.md) to audit and refresh this skill and its feature map.

---
name: verify-codelab-studio
description: Verify CodeLab Studio through its real Tauri desktop window. Use for desktop acceptance or smoke checks of the built-in Go / GoFrame course, native Workspace selection, Monaco saves, and reopening a Workspace after restart.
---

# Verify CodeLab Studio

Run the [feature map](features/README.md) from the repo root. Use the Tauri window and the operating system's folder picker for UI actions. The [Playwright browser fixture](../../../src/__tests__/browser/README.md) runs Vite on port 1421 with `FakeCoreClient`. Its selectors and keyboard actions can guide desktop interaction. Desktop evidence comes from the native window and the Workspace on disk.

## Launch

Use a macOS graphical session with pnpm, Rust and the Tauri build prerequisites. Install missing frontend dependencies with `pnpm install --frozen-lockfile`. Use a desktop driver with window capture and input access, or operate the window directly. On macOS, an automated driver needs Accessibility access for interaction and Screen Recording access for capture. Keep the driver focused on the window owned by this run.

Check `lsof -nP -iTCP:1420 -sTCP:LISTEN` first. Proceed when there is no listener. The configured dev URL is fixed at `http://localhost:1420/`; use the port for one verification run at a time.

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
ps eww -p "$VERIFY_PID" | rg -oF -- "CODELAB_DATA_DIR=$VERIFY_TMP/data"
lsof -nP -iTCP:1420 -sTCP:LISTEN
curl -fsS -o /dev/null -w 'HTTP %{http_code}\n' http://localhost:1420/
```

Confirm the app executable is the one in the launch log. Use the listener PID reported by `lsof` in `ps -p PID -o pid=,ppid=,pgid=,command=` and match it to the same launch process tree or group. Record both PIDs for teardown. Confirm the expected welcome controls in the native window before reporting readiness. Keep the launch log and report the last observed step if boot, desktop input or capture is blocked.

## Drive and evidence

Follow the [desktop journey](features/workspace-save.md) in order. Save the screenshots, file assertions, process details and launch logs to the per-run directory printed as `evidence`. Record the chosen entry point, the visible UI outcome, and each failed or blocked assertion in `run.txt`. Each screenshot must show this run's app window or the native picker it opened. Where screenshots use macOS window IDs, capture with `screencapture -x -l "$VERIFY_WINDOW_ID" "$VERIFY_EVIDENCE/01-welcome.png"`; use the picker window ID or select the picker interactively for the folder-selection screenshot.

## Cleanup

After each launch, quit the verified app through the OS (on macOS, `⌘Q` with its window active). Wait for the launch command to return. If it remains active, use Ctrl+C in its launch terminal. Check that the recorded app and Vite PIDs are gone and that this run no longer owns port 1420. For a surviving process, verify its recorded PID, executable and launch ownership before sending `kill -TERM PID`.

Keep the scratch directory across the two launches. After the last launch or a blocked attempt, remove only the scratch directory created by setup and check the evidence separately.

```sh
case "${VERIFY_TMP:-}" in
  /tmp/codelab-studio-verify.??????) rm -rf -- "$VERIFY_TMP" ;;
  *) printf 'Unexpected scratch path: %s\n' "${VERIFY_TMP:-}" >&2 ;;
esac
test ! -e "$VERIFY_TMP" &&
test -r "$VERIFY_EVIDENCE/run.txt" &&
test -r "$VERIFY_EVIDENCE/launch-1.log" &&
ls -l "$VERIFY_EVIDENCE"
```

Open the retained screenshots and assertion log (or the blocker diagnostics) after cleanup. Refresh the map when the user path changes with [PStack's `/maintain-verification-skill`](https://github.com/cursor/plugins/blob/main/pstack/skills/maintain-verification-skill/SKILL.md).

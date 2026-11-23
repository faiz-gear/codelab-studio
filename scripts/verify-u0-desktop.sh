#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BINARY="${REPO_ROOT}/src-tauri/target/release/codelab-studio"
APP_SUPPORT_DIR="${HOME}/Library/Application Support/studio.codelab.desktop"
MAX_RSS_KB=153600
MAX_MEDIAN_STARTUP_MS=2000

SKIP_BUILD=0
for arg in "$@"; do
  case "$arg" in
    --skip-build)
      SKIP_BUILD=1
      ;;
    *)
      echo "Usage: $0 [--skip-build]" >&2
      exit 2
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "FAIL: scripts/verify-u0-desktop.sh requires macOS (Darwin)." >&2
  exit 1
fi

if [[ -d "${HOME}/.cargo/bin" && ":${PATH}:" != *":${HOME}/.cargo/bin:"* ]]; then
  export PATH="${HOME}/.cargo/bin:${PATH}"
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codelab-u0-desktop.XXXXXX")"
CURRENT_PID=""
SECOND_PID=""

is_pid_running() {
  local pid="$1"
  local stat
  stat="$(ps -o stat= -p "$pid" 2>/dev/null | tr -d '[:space:]' || true)"
  [[ -n "$stat" && "$stat" != Z* ]]
}

count_codelab_procs() {
  local pids
  pids="$(pgrep -x codelab-studio 2>/dev/null || true)"
  if [[ -z "$pids" ]]; then
    echo 0
  else
    printf '%s\n' "$pids" | wc -l | tr -d '[:space:]'
  fi
}

terminate_pid() {
  local pid="$1"
  if [[ -z "$pid" ]]; then
    return 0
  fi
  if is_pid_running "$pid"; then
    kill -TERM "$pid" 2>/dev/null || true
    for _ in $(seq 1 50); do
      if ! is_pid_running "$pid"; then
        break
      fi
      sleep 0.1
    done
    if is_pid_running "$pid"; then
      kill -KILL "$pid" 2>/dev/null || true
    fi
  fi
  wait "$pid" 2>/dev/null || true
}

cleanup() {
  terminate_pid "$SECOND_PID"
  SECOND_PID=""
  terminate_pid "$CURRENT_PID"
  CURRENT_PID=""
  if [[ -n "$TMP_DIR" && -d "$TMP_DIR" ]]; then
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT INT TERM

if (( $(count_codelab_procs) > 0 )); then
  echo "FAIL: codelab-studio is already running before smoke verification." >&2
  exit 1
fi

cd "$REPO_ROOT"

if (( SKIP_BUILD == 0 )); then
  echo "Building release desktop binary (pnpm tauri build --no-bundle)..."
  pnpm tauri build --no-bundle
fi

if [[ ! -x "$BINARY" ]]; then
  echo "FAIL: release binary not found or not executable: ${BINARY}" >&2
  exit 1
fi

echo "Clearing persisted window state at ${APP_SUPPORT_DIR}..."
rm -rf "$APP_SUPPORT_DIR"

STARTUP_TIMES=()
RSS_SAMPLES=()

for run in 1 2 3; do
  LOG_FILE="${TMP_DIR}/run-${run}.log"
  : > "$LOG_FILE"

  echo "Run ${run}/3: cold-starting ${BINARY}..."
  CODELAB_STARTUP_PROBE=1 "$BINARY" >"$LOG_FILE" 2>&1 &
  CURRENT_PID=$!

  READY=0
  for _ in $(seq 1 50); do
    if grep -q "^CODELAB_BOOTSTRAP_READY " "$LOG_FILE" && grep -q "^CODELAB_WINDOW_INITIAL " "$LOG_FILE"; then
      READY=1
      break
    fi
    if ! is_pid_running "$CURRENT_PID"; then
      echo "FAIL [run ${run}]: process exited before emitting CODELAB_BOOTSTRAP_READY" >&2
      cat "$LOG_FILE" >&2
      exit 1
    fi
    sleep 0.1
  done

  if (( READY == 0 )); then
    echo "FAIL [run ${run}]: timed out after 5s waiting for CODELAB_BOOTSTRAP_READY" >&2
    cat "$LOG_FILE" >&2
    exit 1
  fi

  WIN_LINE="$(grep -m 1 "^CODELAB_WINDOW_INITIAL " "$LOG_FILE")"
  WIN_JSON="${WIN_LINE#CODELAB_WINDOW_INITIAL }"
  node -e '
    const data = JSON.parse(process.argv[1]);
    const ok =
      typeof data.width === "number" && data.width >= 1024 &&
      typeof data.height === "number" && data.height >= 720 &&
      data.minWidth === 1024 &&
      data.minHeight === 720;
    if (!ok) {
      console.error("Invalid CODELAB_WINDOW_INITIAL:", data);
      process.exit(1);
    }
  ' "$WIN_JSON"

  BOOT_LINE="$(grep -m 1 "^CODELAB_BOOTSTRAP_READY " "$LOG_FILE")"
  BOOT_JSON="${BOOT_LINE#CODELAB_BOOTSTRAP_READY }"
  STARTUP_MS="$(node -e '
    const data = JSON.parse(process.argv[1]);
    const validLocale = data.locale === "zh-CN" || data.locale === "en-US";
    const validMs =
      typeof data.startupMs === "number" &&
      Number.isFinite(data.startupMs) &&
      data.startupMs >= 0;
    if (data.epoch !== 0 || !validLocale || !validMs) {
      console.error("Invalid CODELAB_BOOTSTRAP_READY:", data);
      process.exit(1);
    }
    console.log(data.startupMs);
  ' "$BOOT_JSON")"
  STARTUP_TIMES+=("$STARTUP_MS")

  RSS_KB="$(ps -o rss= -p "$CURRENT_PID" | tr -d '[:space:]')"
  if [[ ! "$RSS_KB" =~ ^[0-9]+$ ]]; then
    echo "FAIL [run ${run}]: could not read RSS for PID ${CURRENT_PID}" >&2
    exit 1
  fi
  if (( RSS_KB > MAX_RSS_KB )); then
    echo "FAIL [run ${run}]: RSS ${RSS_KB} KB exceeds limit ${MAX_RSS_KB} KB (150 MB)" >&2
    exit 1
  fi
  RSS_SAMPLES+=("$RSS_KB")

  if (( run == 1 )); then
    SECOND_LOG="${TMP_DIR}/second-instance.log"
    "$BINARY" >"$SECOND_LOG" 2>&1 &
    SECOND_PID=$!
    sleep 1

    if is_pid_running "$SECOND_PID"; then
      echo "FAIL [run 1]: second instance (PID ${SECOND_PID}) did not exit within 1s" >&2
      cat "$SECOND_LOG" >&2
      exit 1
    fi
    wait "$SECOND_PID" 2>/dev/null || true
    SECOND_PID=""

    LIVE_COUNT="$(count_codelab_procs)"
    if (( LIVE_COUNT != 1 )); then
      echo "FAIL [run 1]: expected 1 live codelab-studio instance after single-instance check, found ${LIVE_COUNT}" >&2
      exit 1
    fi
    echo "PASS [run 1]: single-instance guard kept exactly 1 live process."
  fi

  MAIN_PID="$CURRENT_PID"
  kill -TERM "$MAIN_PID"
  EXITED=0
  for _ in $(seq 1 50); do
    if ! is_pid_running "$MAIN_PID"; then
      EXITED=1
      break
    fi
    sleep 0.1
  done
  if (( EXITED == 0 )); then
    echo "FAIL [run ${run}]: PID ${MAIN_PID} did not exit cleanly after SIGTERM" >&2
    exit 1
  fi
  wait "$MAIN_PID" 2>/dev/null || true
  CURRENT_PID=""

  REMAINING_PROCS="$(count_codelab_procs)"
  if (( REMAINING_PROCS != 0 )); then
    echo "FAIL [run ${run}]: expected 0 codelab-studio processes after exit, found ${REMAINING_PROCS}" >&2
    exit 1
  fi

  LEFTOVER_CHILDREN="$(pgrep -P "$MAIN_PID" 2>/dev/null || true)"
  if [[ -n "$LEFTOVER_CHILDREN" ]]; then
    echo "FAIL [run ${run}]: leftover child processes for PID ${MAIN_PID}: ${LEFTOVER_CHILDREN}" >&2
    exit 1
  fi

  echo "PASS [run ${run}/3]: startupMs=${STARTUP_MS}ms, rss=${RSS_KB}KB, window=${WIN_JSON}, bootstrap=${BOOT_JSON}"
done

MEDIAN_MS="$(node -e '
  const maxMedian = Number(process.argv[1]);
  const values = process.argv.slice(2).map(Number).sort((a, b) => a - b);
  if (!Number.isFinite(maxMedian) || values.length !== 3 || values.some((n) => !Number.isFinite(n))) {
    console.error("Invalid startupMs samples:", process.argv.slice(1));
    process.exit(1);
  }
  const median = values[1];
  console.log(median);
  if (median > maxMedian) {
    console.error(`Median startupMs ${median} exceeds ${maxMedian}ms`);
    process.exit(1);
  }
' "$MAX_MEDIAN_STARTUP_MS" "${STARTUP_TIMES[@]}")"

echo "PASS: U0 desktop smoke check succeeded. startupMs=[${STARTUP_TIMES[*]}] median=${MEDIAN_MS}ms (<= ${MAX_MEDIAN_STARTUP_MS}ms), rssKB=[${RSS_SAMPLES[*]}] (<= ${MAX_RSS_KB}KB)."

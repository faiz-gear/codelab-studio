#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
run_id="${1:-p01-course-parse-$(date +%Y%m%d-%H%M%S)}"
verify=.agents/skills/verify/scripts/codelab-verify
evidence=".audit/verify/$run_id"
if [[ -e "$evidence" ]]; then
  echo "Choose an unused run ID: $run_id" >&2
  exit 2
fi
mkdir -p "$evidence"
shasum -a 256 src-tauri/tests/fixtures/course_preview.md builtin-courses/go-goframe-practice/README.md > "$evidence/sources.sha256"
trap '"$verify" cleanup --run-id "$run_id"' EXIT
"$verify" launch --run-id "$run_id"
"$verify" doctor --run-id "$run_id" --out "$evidence/doctor.json"
pid="$(cat "${TMPDIR:-/tmp}/codelab-verify-$run_id/pids/desktop.pid")"
swift scripts/verify-course-preview.swift "$pid" "$PWD/src-tauri/tests/fixtures/course_preview.md" \
  'Preview fixture' "$evidence/fixture"
swift scripts/verify-course-preview.swift "$pid" "$PWD/builtin-courses/go-goframe-practice/README.md" \
  'Go 与 GoFrame 开发实践手册：从第一性原理出发' "$evidence/go-handbook"
"$verify" inspect-codelab --run-id "$run_id"
shasum -a 256 -c "$evidence/sources.sha256"

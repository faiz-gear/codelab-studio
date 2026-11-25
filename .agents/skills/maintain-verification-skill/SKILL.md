---
name: maintain-verification-skill
description: Audit CodeLab Studio's desktop verification skill and feature map against the source and the running Tauri app.
license: MIT
---

# Maintain the desktop verification skill

Adapted from [Lauren Tan's PStack skill](https://github.com/cursor/plugins/blob/6714489fa233263f00e419f6cfc4759b07c81056/pstack/skills/maintain-verification-skill/SKILL.md). See [LICENSE](LICENSE).

Run from the repo root. Limit maintenance edits to the verification skill's directory, including its feature map and scripts. Report product regressions with reproduction steps and evidence. [Verification skill](../verify-codelab-studio/SKILL.md) · [Feature map](../verify-codelab-studio/features/README.md)

## Maintenance pass

1. **Check the index.** Compare the feature map with the Markdown files beside it. Correct missing, duplicate, or dead entries.
2. **Read the source.** Dispatch one read-only subagent per feature file in parallel, in batches if needed. Ask each for the user path, source citations, possible drift, and one desktop recipe. The coordinator owns all edits and desktop interaction. If subagents are unavailable, review each feature directly.
3. **Plan desktop coverage.** Check the cited code for each suspected drift and scan recent changes for unmapped user-facing features with a concrete source entry point. Combine overlapping recipes into a single sequential desktop run where possible. The [Workspace save journey](../verify-codelab-studio/features/workspace-save.md) covers opening the built-in course, saving in Monaco, and resuming the same Workspace after restart. Cover every mapped feature and any new entry before declaring the pass complete.
4. **Drive the app.** Use the target skill's Launch, Doctor, evidence, and Cleanup instructions. Run Doctor before driving each launch and again after any failed action. If the window is stuck despite a passing Doctor check, return to a known UI state or relaunch. If Doctor fails because the skill is stale, correct the instructions and retry once. Record each feature's observed result and evidence path. Record a feature as `verified-unreachable` only after attempting its route and identifying the prerequisite that prevents access.
5. **Correct and recheck.** Update the map for confirmed changes to user paths, prerequisites, or expected results. Fix verification harness gaps that prevent exercising working features. Keep any added script executable and document its invocation in the target skill. Repeat affected desktop steps to prove corrections. After a failed attempt, clean up processes and temporary state that the next attempt can do without. Preserve app data and Workspace files across the documented restart. After the last drive, finish teardown and confirm that the recorded evidence is still readable.
6. **Report.** Re-read the diff and report feature coverage, any unreachable prerequisites, confirmed drift, product regressions, and evidence paths. Use `clean` when coverage is complete and the skill needs no corrections, `changed` when coverage is complete and the corrections are proven, or `blocked` when coverage or proof is incomplete. For a blocked pass, include the last completed action and the observed blocker. Keep run notes with the local evidence. Leave corrections in the working tree by default. If the user requested publication, a `changed` pass can produce one PR containing the proven corrections.

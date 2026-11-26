# Check Settings and Workspace-local data

Use the isolated app data directory and a bound scratch Workspace with at least one completed [Exercise Check](exercise-checks.md).

1. Open **设置** in the top bar and confirm **四语言运行时** starts selected. Use the arrow keys, Home and End to select the four tabs. Capture the focused and selected tabs. Press Escape and confirm focus returns to **设置**.
2. In **编辑器与桌面**, change the code font size, theme, or language. Capture the result in Monaco or the top bar. Read the changed preference in `$VERIFY_TMP/data/settings.json` and confirm it survives reopening Settings. Restore the original preference before continuing.
3. In **隐私与本地数据**, click **刷新统计** and capture the counters. Compare **点击运行验收** or **展开文件树** with the corresponding count in `.codelab/metrics.json` in the active Workspace. Capture counts and event names in the run evidence.
4. Record a checksum for the Exercise entry file. Click **重置当前练习进度**, cancel once, and confirm the completed badge is still present. Repeat and click **确认重置进度**. Confirm the badge clears and the entry-file checksum matches. Use the [Git recipe](git-course-pack.md) to exercise Trust State from this tab.
5. After a live [Tutor exchange](acp-tutor.md), click **一键清空对话历史** in that Workspace. Reopen Tutor and confirm the previous prompt and reply are gone.

[Settings tabs](../../../../src/components/RuntimeSettingsDialog.tsx) · [Local data controls](../../../../src/components/PrivacySettingsPanel.tsx)

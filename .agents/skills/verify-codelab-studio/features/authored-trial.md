# Create and try an authored Course Pack

Use an empty scratch directory such as `$VERIFY_TMP/draft`, with an absolute path separate from the app data and learner Workspace. Have Go ready in Preflight to run the generated command.

1. From the welcome page, click **创作新 Course Pack**. Name the course, select Go and enter the scratch path. Click **创建草稿** and capture the source editor and manual preview.
2. Append a marker to the `Exercise 1` heading before `{#ex-1}`. Confirm the preview shows the marker and the draft on disk still has the original heading. Wait for **试跑** to become available and click it.
3. Capture the **Trial Workspace** badge and its temporary path. Independently confirm the draft `README.md` now has the marker. Click **运行命令** on the generated `go run main.go` block and capture `Hello` and the successful exit. This is the generated Starter output.
4. Click **返回草稿**. Confirm the source editor shows the saved heading and the recorded Trial Workspace directory has been removed.

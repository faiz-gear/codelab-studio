# Run Exercise Checks and inspect learning progress

Use the [built-in courses](builtin-course.md) with separate empty Workspaces. Check the runtime prerequisites in [Preflight](runtime-preflight.md). The Go sample uses port 8080. TypeScript uses Bun or Node, Python needs pytest and pytest-json-report, and Java needs JDK 21 and JUnit 5. Use the Managed Python and Java runtimes if the host lacks these tools.

1. After the [manual daemon run](command-execution.md), confirm port 8080 is free. In Go Exercise 1, click **运行验收** under **验收清单**. Capture the **验收结果** panel showing the GET, POST and missing-route checks passing and the exercise **已完成**. Switch to **终端** and stop the daemon. Record that port 8080 is free again.
2. In Exercise 1 of each other built-in course, bind its scratch Workspace and click **运行验收**. Capture a failed Starter case, its duration, and **预期** / **实际** values. Open **原始输出** to confirm the runner report. Use the TypeScript failure for the [Tutor recipe](acp-tutor.md) before applying its solution.
3. Use **查看参考提示与 Diff** and **应用 Reference Solution** in each course. Rerun Checks and capture the passing cases and **已完成**. Confirm the corresponding `.codelab/progress.json` chapter entry records completion and results on disk.
4. In the TypeScript Workspace, use **对比 Starter 恢复** to restore the Exercise 1 entry file and run Checks again. Capture the failed rerun with completion still visible. Complete the other desktop routes before attempting export.
5. Use **导出学习记录** and open `codelab-learning-record.json` in Downloads. Confirm `courseId` is `ts-data-pipeline` and `progress.chapters["ex-1"]` matches `data.chapters["ex-1"]` in the Workspace's `.codelab/progress.json`. macOS may ask for Downloads access and pause the app until the prompt is answered. Capture the prompt and record the permission as the prerequisite if export cannot proceed.

[Check declarations and progress format](../../../../docs/verification.md)

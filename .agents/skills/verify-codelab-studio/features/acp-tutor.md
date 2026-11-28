# Check ACP Tutor and Context Snapshot

Use a trusted scratch Workspace. Use an installed, authenticated ACP-compatible agent to check a live response. For a local protocol check, copy the `FAKE_ACP_AGENT_JS` fixture from the Core seam test to a scratch `.cjs` file and set the ACP launch command to `node /absolute/path/to/fixture.cjs`. Keep prompts and selections within the disposable course material. [ACP fixture](../../../../crates/codelab-core/tests/tutor_seam.rs)

The built-in **Claude Code** preset launches `claude-agent-acp` (`@agentclientprotocol/claude-agent-acp`) and **Codex CLI** launches `codex-acp` (`@agentclientprotocol/codex-acp`), reusing each provider's local CLI login (`claude auth login` / `codex login`). Check logins with `claude auth status` or `codex login status`. If an adapter is not installed globally, first verify the missing-adapter guidance in Settings, then install the adapter into the run's scratch directory for a live check. [Claude Agent ACP](https://github.com/agentclientprotocol/claude-agent-acp) · [Codex ACP](https://github.com/agentclientprotocol/codex-acp#authentication)

```sh
npm install --prefix "$VERIFY_TMP/adapter" --cache "$VERIFY_TMP/npm-cache" \
  --no-audit --no-fund @agentclientprotocol/claude-agent-acp@0.113.0
printf '%s\n' "$VERIFY_TMP/adapter/node_modules/.bin/claude-agent-acp"
```

Paste the printed path into **启动命令** (or add its `.bin` directory to `PATH` before launch to verify the default preset command directly). Capture the successful handshake and the completed response. The handshake checks the protocol connection (and `auth/status` when advertised by the adapter). Sending a prompt also checks the agent's session and provider authentication.

1. Open **设置** → **ACP 助教**. Select the local agent preset or enter its launch command. Click **测试 ACP 握手** and capture the returned agent name or the exact prerequisite error. Check the default **引导只读** permission policy.
2. In Go Exercise 1, open the manual and `task-api/main.go`. Use output from the [daemon run](command-execution.md) for the terminal region. Select at least two characters in the manual, Monaco, and terminal in turn. Capture each selection pill, choose **提问**, and confirm the quote names the correct region. Close Tutor between selections.
3. Open **课后思考题** and use **问 AI · 聊聊思考题**. In the [TypeScript Starter failure](exercise-checks.md), use **问 AI** on the failed check row and **问 AI 分析验收结果** in the results panel. Capture each source quote.
4. With the Exercise 1 file open and results recorded, use **新建终端** and enter `echo 'error: address already in use (verification fixture)'`. Use **问 AI** in the recovery strip and capture the port-conflict quote. Preview the **活跃文件** and **终端报错** chips. Capture the file path, fixture line, and `/ 6000 tok` estimate. Remove a populated chip, confirm the estimate drops, and restore the default context.
5. With a successful handshake, send a short question from the scratch course and capture the streamed reply. If handshake fails, record the unavailable agent prerequisite for this step. The selection and context steps can run before an agent starts.

[Workbench Ask AI routes](../../../../src/components/CourseWorkbench.tsx) · [Context Snapshot](../../../../crates/codelab-core/src/tutor/context.rs)

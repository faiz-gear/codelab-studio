---
status: accepted
---

# ACP 默认自动放行工作区非敏感只读并采用 Workspace 级会话复用

现有 PRD（M4）与交互原型（`docs/design/prototype/index.html`）在 ACP 权限体验上存在直接冲突：PRD 要求引导模式默认放行工作区只读、仅审批写与执行，而原型连只读查看 `main.go` 都弹出了审批卡；此外 ACP 是长驻有状态进程，频繁随切章节重启会丢失跨节讨论并拖慢响应。我们正式裁决以 PRD 原则为准：默认 `Permission Policy` 为 `guided_readonly`（自动放行工作区规范路径内且命中非敏感规则的文件读取，对工作区外读取、所有写入与命令执行强制弹窗审批，并提供可选 `strict_ask_all` 全审批模式）；同时以单个 Workspace 为边界复用 ACP 会话，在每轮 `session/prompt` 前由工作台显式注入用户可控的 `Context Snapshot` 与教学风格指令。

## Considered Options

- **默认 `guided_readonly` + Workspace 级会话复用（已采纳）**：默认排查报错流畅无打断，高风险操作零妥协，且支持递增式课程跨章节追问。
- **默认全审批 + 按 Chapter 重建会话**：与原型演示一致，但日常排查报错点击负担过高，且切换章节即清空 Agent 语境与冷启动子进程。

## Consequences

- **有意覆盖交互原型中的只读审批演示行为**：研发实现以本 ADR 及 `guided_readonly` 为默认策略，不再将「只读必弹窗」作为默认验收标准。
- ACP 经审批修改工作区文件时，若编辑器无未保存草稿则静默刷新并提供 Diff 入口；若存在未保存草稿，则走外部修改冲突横幅保护（加载磁盘版 / 保留草稿），绝不覆盖手写改动。

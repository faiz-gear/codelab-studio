---
status: accepted
---

# v1 AI 助教仅实现 ACP 协议并推迟 BYOK

PRD v3.1（M4、US-008）原计划在首期同时交付 BYOK（多家 LLM HTTP 客户端 + OS Keychain 密钥管理）与 ACP 本地编码 Agent。为聚焦核心产品差异化并避免在 8 周 MVP 中维护多厂商 LLM SDK、代理兼容与三端系统钥匙串，我们决定 **v1 AI 助教仅实现 ACP（JSON-RPC 2.0 over stdio）协议**，移除 v1 的 BYOK 界面与密钥存储，同时在 Rust Core 中保留薄 `TutorBackend` Trait 边界以便二期扩展。

## Considered Options

- **v1 纯 ACP + 保留 Trait 扩展点（已采纳）**：直接复用学习者本机已认证的 Claude Code / Gemini CLI / Codex CLI / OpenCode 等 Agent，免去密钥存储与计费配置负担。
- **v1 同时交付 BYOK 与 ACP**：按 PRD v3.1 同步开发 OpenAI/Anthropic/Ollama SSE 客户端与三端 Keychain 封装。

## Consequences

- **有意偏离 PRD v3.1 / US-008**：任何后续开发或评审均不得在 v1 补回 BYOK 表单或 Keychain 依赖，直至二期重新立项。
- 离线无外部 ACP CLI 的用户仍可 100% 使用文档、编辑器、多终端与自动验收（Local-first 承诺不受影响），但使用 AI 助教的前提是本机已安装至少一个支持 ACP 的 CLI Agent。

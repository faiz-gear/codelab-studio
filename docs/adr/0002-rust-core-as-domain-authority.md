---
status: accepted
---

# Rust Core 作为唯一领域真源

CodeLab Studio 基于 Tauri 2 构建，同时包含复杂的文档解析、工作区路径沙箱、四语言运行时管理、PTY 路由、验收编排、进度原子持久化及 ACP 权限审批。我们决定将全部领域模型、安全边界与核心用例收敛在独立于 Tauri 胶水层的纯 **Rust Core** 中作为唯一权威真源，Web 前端（React/TypeScript）仅负责视图渲染与临时 UI 交互态，通过强类型 IPC Command 与事件流通信。

## Considered Options

- **Rust Core 唯一真源（已采纳）**：解析、上下文预算截断、秘密文件过滤、进度状态机与 ACP 协议全部在 Rust 侧实现并可脱离 GUI 独立单测。
- **前后端双写拆分**：在 TypeScript 侧做 Markdown 解析、上下文装配与进度逻辑，Rust 侧仅做文件系统与子进程系统调用代理。

## Consequences

- 安全红线（路径规范化、敏感文件屏蔽、高危命令拦截、ACP 权限闸门）全在单一可信边界内执行，消除前后端模型漂移。
- 领域核心可在无桌面窗口环境下运行快速单元与集成测试；前端开发需遵循后端下发的契约事件而非在浏览器端旁路修改磁盘或状态。

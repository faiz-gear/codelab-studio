---
status: accepted
---

# 分离只读 Course 与可写 Workspace（1:0..N）

CodeLab Studio 同时支持内置课程、单文件 Markdown 教程、本地目录与 Git 课程包，若将课程源目录直接当作学习者的代码工作区，会导致上游更新覆盖用户代码、无法重复开练以及单文件 Markdown 生命周期割裂。我们决定将 **Course（只读内容与规则定义源）** 与 **Workspace（用户本地可写代码目录，承载 `starter/` 复制产物与 `.codelab/` 局部状态）** 拆分为两个独立领域聚合，呈 `1:0..N` 关系。

## Considered Options

- **分离实例化（已采纳）**：Course 缓存或只读引用，首次打开空目录时复制 `starter/`，绑定已有非空目录时不覆盖原有文件；上游更新仅刷新手册与验收规则。
- **原地合二为一**：直接把克隆下来的 Git 仓库或本地课程目录当作工作区原地修改。

## Consequences

- 同一门 Course 可实例化多个独立 Workspace 重练，重装或刷新课程源不会破坏用户手写代码。
- 课程创作（Authoring）需通过独立「试跑 Workspace」将作者源文件与学习者试跑产物隔离。

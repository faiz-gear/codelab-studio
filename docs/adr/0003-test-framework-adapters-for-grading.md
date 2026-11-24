---
status: accepted
---

# 四语言判题统一走真实测试框架适配器

PRD 要求同时覆盖工程双终端实战与 JS/TS、Go、Java、Python 四语言函数判题，而交互原型曾用浏览器内 `new Function` 模拟函数断言。我们决定不在应用内自研「在 JSON 中声明输入输出并动态生成胶水测试代码」的 LeetCode 式判题器，而是将所有四语言函数与工程验收统一建模为基于真实本地测试框架的结构化输出适配（`bun test`、`go test -json -race`、`JUnit 5`、`pytest --json-report`）加 HTTP 合同断言，由各语言适配器归约为统一的 `CheckResult`。

## Considered Options

- **真实测试框架适配器（已采纳）**：课程包内编写标准工程单测，适配器解析结构化报告产出 `Expected vs Actual` Diff。
- **内置 JSON 动态函数求值器**：新增 `function_eval` 规则由工作台自动生成并执行四语言包装测试代码。

## Consequences

- 课程包测试文件完全零平台锁定，学习者在 VS Code / GoLand / IDEA 中也能原生运行同一套测试。
- 课程包创作者需随 `starter/` 提供对应语言的真实测试代码，实例化后在 Workspace 内对学习者完全可见；若误改测试文件，可通过与 `starter/` 的 Diff 一键恢复。

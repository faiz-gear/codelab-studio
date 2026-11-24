---
status: accepted
---

# 分离 Check Result 实时态与 Exercise 粘性完成态

递增式工程实践课程（如内置 Go/GoFrame 练习 0～9）让多个连续 Exercise 共享同一份演进中的代码库，后续章节的重构极易改变早期章节的接口或行为。若将「某次重跑检查失败」直接等同于撤销该章节已完成状态，会导致学习进度随正常推进反复倒退。我们决定将单次验收产出的实时 **Check Result** 与 **Exercise 完成态** 彻底分离：自动 Check 全通过（或无自动 Check 时由用户手动勾选，Self-Check 默认不阻塞）即晋升为 `completed` 并保持粘性，后续重跑失败只如实展示最新运行结果，除非用户显式重置进度，绝不自动回退历史完成态。

## Considered Options

- **实时结果与粘性完成态分离（已采纳）**：历史达成记录稳定保留，同时最新运行反馈保持完全诚实。
- **实时联动状态**：每次验收结果直接覆写章节完成态，失败即回退为未完成。

## Consequences

- `progress.json` 必须分别持久化章节的 `completionState` 与各 Check 的 `lastCheckResult`；未来工程实现不得把「已完成章节出现红叉」误判为 bug 而去回退完成态。
- 课程包若确实希望某条 Self-Check 阻塞完成，需在规则中显式声明 `required: true`。

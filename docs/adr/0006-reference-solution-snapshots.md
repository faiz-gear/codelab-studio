---
status: accepted
---

# Reference Solution 采用按章节文件快照而非行替换补丁

PRD 与原型把「查看参考提示与 Diff → 确认后安全写入」定为核心交互，但 PRD 的课程包目录未规定参考代码的存放方式，原型则使用了脆弱的单行字符串替换（`oldLine/newLine`）。一旦学习者改动了缩进、注释或相邻代码，行替换就会失效。我们决定在标准 Course Pack 中通过可选的 `solutions/<chapterId>/...` 目录提供按章节组织的目标参考文件快照，并在无该目录的纯 Markdown 教程中降级解析带 `file=<path>` 标注的代码围栏块，统一以「工作区当前文件 vs 参考版本」双列 Diff 交由用户确认合并。

## Considered Options

- **章节目录快照 + Markdown `file=` 降级（已采纳）**：天然支持多文件修改，与学习者任意中间代码态都能生成准确 Diff。
- **Unified `.patch` 或 JSON 行替换规则**：对上下文行号和空白极其敏感，学习者提前修改或课程微调时极易 apply 失败。

## Consequences

- 标准 Course Pack 目录契约在 `codelab.json`、`README.md`、`starter/`、`.codelab-checks.json` 之外新增可选顶层目录 `solutions/<chapterId>/`。
- 课程包创作者只需维护可正常编译和通过单测的参考源文件，无需手写脆弱的补丁描述。

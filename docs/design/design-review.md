# CodeLab Studio — 设计评审与实机验证报告

> 评审对象：[design-system.md](./design-system.md) · [tokens.css](./tokens.css) · [tailwind.config.js](./tailwind.config.js) · [prototype/index.html](./prototype/index.html)
> 实机截图：[首页](./screenshots/01-welcome-light.png) · [默认两栏工作台](./screenshots/02-workbench-default-light.png) · [按需展开 AI 与 ACP](./screenshots/03-workbench-active-ai.png) · [深色模式验收通过](./screenshots/04-workbench-dark.png)

---

## B 方案定稿验证（v3.2）

用户选定 B「均衡紧凑」。正式原型已同步首页卡片密度、三步手册间距、360px AI 抽屉与 BYOK / ACP 直接切换，移除实验控制条。旧版截图和下文 R1–R3 记录保留为历史证据。

Google Chrome 157.0.8103.80 无头模式，设备缩放比 1，共生成 16 张正式原型截图。45 项浏览器检查通过；捕获状态无 JavaScript 错误及页面横向溢出。AI 展开时，两种尺寸、两种主题下的编辑区均为 560px。检查包含协议与设置同步、审批允许/拒绝、上下文删除、Esc 与关闭按钮、焦点恢复、隐藏抽屉不可聚焦、分栏拖拽宽度限制，以及 1024px 覆盖抽屉。具体结果见 [verification.json](./screenshots/b-balanced/verification.json)。

| 尺寸与主题 | 首页 | 工作台 | BYOK | ACP |
|---|---|---|---|---|
| 1440x900 浅色 | [home](./screenshots/b-balanced/home-1440x900-light.png) | [work](./screenshots/b-balanced/work-1440x900-light.png) | [ai](./screenshots/b-balanced/ai-1440x900-light.png) | [acp](./screenshots/b-balanced/acp-1440x900-light.png) |
| 1440x900 深色 | [home](./screenshots/b-balanced/home-1440x900-dark.png) | [work](./screenshots/b-balanced/work-1440x900-dark.png) | [ai](./screenshots/b-balanced/ai-1440x900-dark.png) | [acp](./screenshots/b-balanced/acp-1440x900-dark.png) |
| 1280x800 浅色 | [home](./screenshots/b-balanced/home-1280x800-light.png) | [work](./screenshots/b-balanced/work-1280x800-light.png) | [ai](./screenshots/b-balanced/ai-1280x800-light.png) | [acp](./screenshots/b-balanced/acp-1280x800-light.png) |
| 1280x800 深色 | [home](./screenshots/b-balanced/home-1280x800-dark.png) | [work](./screenshots/b-balanced/work-1280x800-dark.png) | [ai](./screenshots/b-balanced/ai-1280x800-dark.png) | [acp](./screenshots/b-balanced/acp-1280x800-dark.png) |

AI 与终端依然是交互模拟，以上检查不表示已接通实际模型或 ACP 进程。

---

## 1. 迭代演进与核心痛点闭环

| 轮次 | 发现问题 | 修复与验证结论 |
|---|---|---|
| **R1 · Arc UI 风格对齐** | 旧版深蓝灰风格偏重、缺主题定制；原始 Arc `--text-muted` (59%) 与 `--tone-amber` (55%) 在浅色底接近 AA 边界 | 全面切换零色偏 `oklch` 中性灰 + 粉彩语义族 + 外观定制侧滑抽屉；校准 `--text-muted` 至 `52%`（`5.42:1`）、`--tone-amber` 至 `50%`（`6.16:1`），双主题 100% 通过 WCAG 2.1 AA |
| **R2 · 视觉细节巡检** | Headless Chrome 实机发现窄栏 `cwd` 胶囊折行、右下角多条 Toast 遮挡 AI 输入框、设置弹窗双列标签错位 | 锁定 `.cmd-head` 单行省略；Toast 迁至顶部居中胶囊并限显 2 条；统一基线对齐，二次截图确认零折行零遮挡 |
| **R3 · 认知减负重构（v3.1）** | 用户反馈首屏内容过多、入口不明确、文字密集给人压力 | 首页收敛为居中 Arc 光晕单一主入口 + 宽松 `2×2` 卡片；工作台改为默认两栏 + 编号三步流（`①→②→③`），文件树/辅助命令/思考题/AI 抽屉全部改为渐进披露，认知负荷显著降低且零功能缺失 |

---

## 2. 自动化回归与实机检查结果

- **脚本语法与运行时**：`node --check` 零语法错误；Headless Chrome（`1440×900` 与 `1280×800`）全程 `PROTO_ERRORS: []`。
- **核心业务闭环实测**：
  - Go 练习 1：启动常驻服务 `go run .` → 运行验收初检 `2/3` 展开 Diff → 唤起 AI 与 ACP 权限审批卡 → 安全写入参考修改 → 复检 `3/3` 全绿通过。
  - JS/TS 练习：编辑器内真实执行 `getAdultNames` 函数断言 `3/3` 通过。
  - 安全与扩展：`rm -rf` 触发高危确认弹窗；Java 托管运行时一键安装生效；外观抽屉 8 组强调色与浅/深主题无闪烁切换。

---

## 3. 综合评审得分

| 维度 | 得分（/10） | 说明 |
|---|---:|---|
| 入口清晰度与视觉呼吸感 | **9.7** | 单一居中主入口 + 默认两栏三步引导流，零压迫感 |
| Arc UI 风格还原度 | **9.7** | 纯中性灰阶、Geist Medium、柔和光晕 Hero、外观侧滑抽屉 |
| 对比度与无障碍（WCAG 2.1 AA） | **9.8** | 浅/深双主题全令牌实测 ≥ 4.95:1，完整键盘与减弱动效支持 |
| PRD 业务闭环完整度 | **9.8** | 四语言判题、双终端路由、课程包导入/创作、BYOK + ACP 全闭环 |
| **综合评分** | **9.75 / 10** | **设计定稿，可直接进入 Tauri 2 工程研发** |

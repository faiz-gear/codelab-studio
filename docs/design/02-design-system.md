# CodeLab Studio — 设计系统与组件规范 v3.0（Arc UI 主题风格版）

> 令牌源文件：[tokens.css](./tokens.css) · Tailwind 映射：[tailwind.config.js](./tailwind.config.js)
> 视觉语言参考：**Arc UI**（`https://uiarc.dev/docs/introduction` · `/docs/theming` · `/docs/motion`）
> 功能基线：完整保留 Tauri 2 本地桌面工作台、JS/TS · Go · Java · Python 四语言托管运行时与函数判题、课程包生态，以及 BYOK + ACP 双协议 AI 助教。

---

## 1. 设计哲学：Arc UI 美学与极客工作台的融合

1. **纯中性画廊阶梯（Pure Achromatic Neutrals）**：摒弃带蓝灰偏色的传统暗色面板，全面采用 `oklch(L% 0 0)` 零色相偏移中性灰阶。浅色模式为纯净白底画廊（`oklch(100% 0 0)` → `oklch(97.8% 0 0)`），深色模式为独立调校的暖黑曜石灰（`oklch(19% 0 0)` → `oklch(21.5% 0 0)` → `oklch(24% 0 0)`），让代码语法高亮与验收语义色成为唯一视觉焦点。
2. **默认克制单色 + 8 组可切换强调色（`data-accent`）**：默认主按钮与激活控件遵循 Arc UI 经典的单色反差哲学（浅色深墨黑底白字 `19.67:1` / 深色柔象牙白底深字 `16.44:1`），同时在顶栏提供 Arc 标志性的 **「外观（Appearance）」侧滑定制抽屉**，支持 8 组强调色（`neutral / violet / blue / green / amber / orange / coral / rose`）、浅/深主题切换与一键复制主题 CSS。
3. **冷却粉彩语义家族（Cool Pastel Tones）**：状态色与四语言标识统一收敛于 Arc 粉彩色族（`--tone-teal / sky / violet / rose / amber / green`），徽章全部通过 `color-mix(in oklab, var(--badge-tone) var(--badge-tint), var(--surface))` 动态调和，告别高饱和霓虹刺眼感。
4. **轻字重编辑感排版（Geist Medium + Inter）**：标题使用 **Geist Medium（`500`）** 搭配 `-0.03em` 紧密字距与 `1.12` 行高，正文与控件使用 **Inter** 搭配 `-0.01em` 字距，代码与数字使用 **JetBrains Mono + `tabular-nums`**，呈现出版物级的清晰与秩序。
5. **临界阻尼弹簧动效（Zero-Overshoot Motion）**：遵循 Arc Motion 规范，使用 `--ease-standard (cubic-bezier(.22,1,.36,1))`、`--ease-drawer (cubic-bezier(.32,.72,0,1))` 与临界阻尼 `linear()` 弹簧曲线，只动画化 `transform` 与 `opacity`，完整响应 `prefers-reduced-motion`。

---

## 2. 色彩与表面令牌体系（WCAG 2.1 AA 实测校准）

### 2.1 中性表面与文本层级

| 语义令牌 | 浅色模式（默认） | 深色模式 | 实测对比度（对主面） | 职责边界 |
|---|---|---|---|---|
| `--background` | `oklch(100% 0 0)` (`#FFFFFF`) | `oklch(19% 0 0)` (`#141414`) | — | 应用底布、顶栏玻璃基底 |
| `--surface` | `oklch(100% 0 0)` (`#FFFFFF`) | `oklch(21.5% 0 0)` (`#191919`) | — | 手册区、编辑器、终端、卡片、AI 抽屉 |
| `--surface-raised` | `oklch(100% 0 0)` (`#FFFFFF`) | `oklch(24% 0 0)` (`#202020`) | — | 弹窗、Popover、划词浮层、命令块头部 |
| `--surface-muted` | `oklch(97.8% 0 0)` (`#F8F8F8`) | `oklch(26% 0 0)` (`#242424`) | — | 静音填充、分段轨道、行内代码、输入槽 |
| `--sidebar-fill` | `color-mix(foreground 2.2%, bg)` | `oklch(17.8% 0 0)` | — | 文件树与侧边栏极淡底色 |
| `--border` / `--border-subtle` | `oklch(92.5% 0 0)` / `96%` | `oklch(29% 0 0)` / `24.5%` | — | 1px 精密分割线与卡片内部细线 |
| `--border-strong` | `oklch(81% 0 0)` | `oklch(40% 0 0)` | — | 悬停描边、输入框边界、浮层外框 |
| `--foreground` | `oklch(15% 0 0)` (`#0B0B0B`) | `oklch(96% 0 0)` (`#F2F2F2`) | **19.67:1** / **15.59:1** | 主标题、主按钮基底、核心正文 |
| `--text-secondary` | `oklch(46% 0 0)` (`#585858`) | `oklch(77% 0 0)` (`#B4B4B4`) | **7.13:1** / **8.45:1** | 手册正文、副标题、说明描述 |
| `--text-muted` | `oklch(52% 0 0)` (`#696969`) | `oklch(66% 0 0)` (`#929292`) | **5.42:1** / **5.60:1** | 路径、时间戳、占位提示（较原版 `59%` 调深至 AA+） |

### 2.2 粉彩语义色与四语言色标

| 语义角色 | 令牌 | 浅色值（对比度） | 深色值（对比度） | 使用场景 |
|---|---|---|---|---|
| **Success / 通过** | `--success` (`--tone-green`) | `oklch(50% .12 162)` (**5.57:1**) | `oklch(79% .14 162)` (**9.60:1**) | 验收通过、Exit 0、常驻服务运行脉冲、LSP 就绪 |
| **Info / Go** | `--info` (`--tone-sky`) | `oklch(51% .11 240)` (**5.67:1**) | `oklch(79% .095 236)` (**9.21:1**) | Go 语言徽章、命令运行信息提示 |
| **AI / ACP** | `--ai` (`--tone-violet`) | `oklch(51% .16 290)` (**6.13:1**) | `oklch(74% .125 290)` (**7.34:1**) | AI 助教、ACP 徽章、上下文 Chip、思考题（配 `--arc-gradient-soft` 柔光底） |
| **Warning / JS·TS** | `--warning` (`--tone-amber`) | `oklch(50% .12 68)` (**6.16:1**) | `oklch(83% .12 78)` (**10.25:1**) | JS/TS 标签、端口冲突、外部修改冲突、ACP 权限审批 |
| **Danger / Java** | `--danger` (`--tone-rose`) | `oklch(53% .18 10)` (**5.83:1**) | `oklch(71% .155 8)` (**6.32:1**) | Java 标签、断言失败、Exit ≠ 0、高危命令拦截 |
| **Teal / Python** | `--tone-teal` | `oklch(50% .095 182)` (**5.66:1**) | `oklch(79% .13 182)` (**9.58:1**) | Python 课程标签、隔离运行环境标识 |

### 2.3 Arc 标志性光感渐变（Hero & AI Wash）
- **欢迎页 Hero 氛围渐变（`--arc-hero-bg`）**：浅色模式为柔和玫瑰粉（`#e9babb`）+ 雾蓝（`#afc7f6`）+ 暖沙（`#e8d9ad`）三点径向光晕叠于 `#f8f7f5`；深色模式收敛为低饱和暗梅/靛青/青铜叠加 `#1d1e22`，营造 Arc 官网极具辨识度的温润科技气质。
- **AI 柔光洗底（`--arc-gradient-soft`）**：极浅紫-暖杏线性过渡，用于思考题卡片与 AI 引用卡片，保持工作台视觉安静。

---

## 3. 排版、圆角、阴影与动效规范

| 维度 | 规范明细 |
|---|---|
| **字体栈** | 展示标题：`Geist`（`500` 中等字重，`-0.03em` 字距，`1.12` 行高）；正文与 UI：`Inter` + `PingFang SC / Noto Sans SC`（`-0.01em` 字距）；代码/终端/数据：`JetBrains Mono` + `tabular-nums` |
| **圆角阶梯** | 胶囊/主 CTA `9999px` · 英雄区大面 `--radius-surface: 24px` · 弹窗/卡片/命令块 `--radius-panel: 18px`（工作台内卡片 `14px`） · 控件/输入框 `--radius-control: 10px` · 小标签 `6px` |
| **阴影克制原则** | 静态面板仅以 `1px solid var(--border)` 区分；悬停卡片轻抬 `--shadow-raised`；仅浮层（划词条、Popover、外观抽屉、弹窗）允许使用 `--shadow-floating` |
| **动效曲线** | 常规交互 `--duration-fast: 150ms` + `--ease-standard`；抽屉滑出 `240ms` + `--ease-drawer (0.32, 0.72, 0, 1)`；开启 `prefers-reduced-motion` 自动归零 |

---

## 4. 核心业务组件（Arc UI 化升级清单）

1. **桌面顶栏（`48px` 半透明磨砂玻璃）**：集成 macOS 静默窗口点（Hover 显色）、极简几何 Brand 标志、胶囊课程切换器、细条进度胶囊、环境体检 Popover、Arc 分段布局控制器（`标准 | 辅导 | 专注`）、AI 助教胶囊、`外观` 定制入口、主题快切与设置入口。
2. **外观定制侧滑抽屉（Arc `Customize` Drawer，新增）**：点击顶栏「外观」滑出，背景毛玻璃虚化（`backdrop-filter: blur(8px)`），内置 8 组圆形强调色板（带选中对勾）、一键「复制主题 CSS」、浅/深主题分段器与代码字号预设。
3. **欢迎页 / 课程库**：顶部 `24px` 大圆角 Arc 氛围光晕 Hero 卡（内嵌双终端与 AI/ACP 实时演示预览）、四格极简 Metric 卡、胶囊语言筛选器、悬停轻抬的 Arc 课程卡片网格、表格风格的最近本地工作区列表。
4. **交互式手册与命令块（Arc `Code block` 风格）**：`14px` 圆角、`--surface-raised` 头部展示可编辑 `cwd` 胶囊与粉彩模式标签（`常驻服务` / `单次命令` / `高危演练`），左侧状态色条平滑切换 `Idle / Running / Success / Error`；验收卡与思考题卡保留完整功能，视觉全面去重框、轻盈化。
5. **文件树 + 多标签编辑器 + 双模底栏**：文件树采用 `--sidebar-fill` 极淡底色与圆角行高亮；底栏采用 Arc 胶囊 Tab 隔离常驻服务进程与单次终端，并提供结构化函数判题 Diff 视图。
6. **划词浮层与 AI / ACP 抽屉**：划词条升级为悬浮胶囊（`--shadow-floating`）；AI 抽屉采用 Arc 对话气泡体系，完整支持 BYOK/ACP 协议切换、苏格拉底/直接模式、可移除四维 Context Chips 与 ACP `session/request_permission` 工具权限审批卡。

---

## 5. 桌面端断点与无障碍

| 窗口宽度 | 布局行为 |
|---|---|
| `≥ 1440px` | 标准三栏并列（手册 36% / 工作区 64%，AI 展开时 30% / 44% / 26%），分隔条拖拽与双击复原 |
| `1280–1439px` | 展开 AI 抽屉时自动折叠产物文件树，顶栏隐藏次要装饰徽章与进度条轨道，保障零换行与充足编辑区 |
| `< 1280px` | AI 抽屉转为右侧毛玻璃覆盖浮层（`--shadow-floating`）；`⌘B` 一键折叠手册为 `46px` 专注条 |
| 键盘与无障碍 | 跳过导航链接、`:focus-visible` 双环聚焦态、`aria-live="polite"` 播报验收与终端结果、`⌘B / ⌘S / Esc` 快捷键、全色板 WCAG 2.1 AA+ |

# CodeLab Studio

开源、本地优先（Local-first）的 **Tauri 2 桌面交互式编程实践工作台**——将任意标准 Markdown 实践手册或本地 / Git 课程包，转化为「编号引导手册 + 本地代码工作区 + 智能多终端与四语言函数判题 + 划选上下文感知 AI / ACP 助教」一体化实战环境。

- **支持语言**：JS/TS、Go、Java、Python（内置托管隔离运行时与多文件函数判题）
- **AI 助教**：支持自定义 LLM Provider（BYOK，钥匙串加密）与 **ACP（Agent Client Protocol）** 连接本地 Coding Agent
- **视觉体系**：采用 **Arc UI** 纯中性极简风格（浅/深双主题 × 8 组强调色 × 渐进披露工作台）

## 项目文档与产物导航

```text
codelab-studio/
├── README.md
└── docs/
    ├── prd/
    │   ├── PRD.md                               # 完整产品需求文档（v3.1 定稿版）
    │   └── prd-stories.json                     # 结构化任务拆解（18 条原子用户故事）
    └── design/
        ├── design-system.md                     # Arc UI 设计系统与渐进披露规范
        ├── tokens.css                           # 三层 oklch 色彩 / 字体 / 动效令牌
        ├── tailwind.config.js                   # Tailwind 映射配置
        ├── design-review.md                     # 设计评审与实机回归报告
        ├── prototype/
        │   └── index.html                       # 单文件高保真交互原型（浏览器直接打开）
        └── screenshots/
            ├── 01-welcome-light.png             # 首页（单一主入口）
            ├── 02-workbench-default-light.png   # 默认两栏三步流工作台
            ├── 03-workbench-active-ai.png       # 按需展开 AI 与 ACP 审批态
            └── 04-workbench-dark.png            # 深色黑曜石验收通过态
```

## 快速预览原型

直接用浏览器打开 [docs/design/prototype/index.html](docs/design/prototype/index.html) 即可体验完整交互流程（外观 8 色切换、四语言课程与真实 JS 判题、双终端路由、安全写入 Diff、高危命令拦截、BYOK / ACP 助教配置）。

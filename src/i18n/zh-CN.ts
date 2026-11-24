import type {
  AccentPreset,
  CheckKind,
  CourseLanguage,
  ErrorCode,
  StoreKind,
} from "../core/contracts";

export interface CoursePlaceholderMeta {
  id: string;
  language: CourseLanguage;
  totalExercises: number;
}

export const COURSE_PLACEHOLDER_META: readonly CoursePlaceholderMeta[] = [
  { id: "go-goframe-practice", language: "Go", totalExercises: 10 },
  { id: "ts-data-pipeline", language: "JS/TS", totalExercises: 8 },
  { id: "python-data-clean", language: "Python", totalExercises: 6 },
  { id: "java-rate-limiter", language: "Java", totalExercises: 7 },
] as const;

export interface CoursePlaceholderText {
  title: string;
  description: string;
}

export interface Messages {
  skipToMain: string;
  brandTitle: string;
  topbar: {
    homeTooltip: string;
    courseMenuLabel: string;
    courseMenuEmpty: string;
    builtinGroupLabel: string;
    importedGroupLabel: string;
    importLocalAction: string;
    backToHomeAction: string;
    progressLabel: string;
    focusModeLabel: string;
    focusModeTooltip: string;
    envStatusLabel: string;
    envStatusTooltip: string;
    appearance: string;
    appearanceTooltip: string;
    settingsLabel: string;
    settingsTooltip: string;
  };
  welcome: {
    ariaLabel: string;
    heroTitle: string;
    heroSubtitle: string;
    primaryCtaContinue: string;
    primaryCtaStart: string;
    secondaryCtaImport: string;
    resumeSummaryPrefix: string;
    localFirstSummary: string;
    footerDesktop: string;
    footerLanguages: string;
    footerTutor: string;
    coursesHeading: string;
    filterGroupLabel: string;
    filterAll: string;
    recentHeading: string;
    newCoursePack: string;
    recentEmptyTitle: string;
    recentEmptyHint: string;
    placeholderNotice: string;
    courseTexts: Record<string, CoursePlaceholderText>;
  };
  importModal: {
    dialogLabel: string;
    title: string;
    subtitle: string;
    close: string;
    modeGroupLabel: string;
    modeMarkdown: string;
    modePack: string;
    pathLabelMarkdown: string;
    pathLabelPack: string;
    pathPlaceholderMarkdown: string;
    pathPlaceholderPack: string;
    browseFile: string;
    browseDir: string;
    hintMarkdown: string;
    hintPack: string;
    cancel: string;
    submit: string;
    submitting: string;
    pathRequired: string;
  };
  workbench: {
    ariaLabel: string;
    manualAriaLabel: string;
    stageAriaLabel: string;
    focusRailExpand: string;
    focusRailLabel: string;
    outlineToggle: string;
    outlineTitle: string;
    outlineClose: string;
    outlineSummary: string;
    prevChapter: string;
    nextChapter: string;
    chapterKindGuide: string;
    chapterKindExercise: string;
    exerciseCounter: string;
    guideHeaderBadge: string;
    splitterTitle: string;
    splitterAriaLabel: string;
    reloadTitle: string;
    reloadDetail: string;
    reloadAction: string;
    reloadDismiss: string;
    reloadedToast: string;
    step1Title: string;
    step2Title: string;
    step3Title: string;
    targetFileLabel: string;
    requiresDaemonLabel: string;
    secondaryCommandsTitle: string;
    reflectionsTitle: string;
    linearFallbackNotice: string;
    commandModeDaemon: string;
    commandModeOneshot: string;
    cwdWorkspaceRoot: string;
    copyCommand: string;
    copiedCommandToast: string;
    copyFailedToast: string;
    readOnlyPreviewBadge: string;
    builtinBadge: string;
    localMarkdownBadge: string;
    localPackBadge: string;
    stageTitle: string;
    stageGuideHint: string;
    stageExerciseHint: string;
    stageCommandsHeading: string;
    stageNoCommands: string;
    stageChecksHeading: string;
    stageNoChecks: string;
    stageMetadataHeading: string;
    stageChapterIdLabel: string;
    stageSourceLabel: string;
    stageNextMilestoneNote: string;
    checkKinds: Record<CheckKind, string>;
  };
  appearance: {
    dialogLabel: string;
    title: string;
    close: string;
    accentLabel: string;
    accentGroupLabel: string;
    copyConfig: string;
    copiedToast: string;
    copyFailedToast: string;
    themeLabel: string;
    themeGroupLabel: string;
    themeLight: string;
    themeDark: string;
    codeSizeLabel: string;
    codeSizeGroupLabel: string;
    sizeCompact: string;
    sizeStandard: string;
    sizeLarge: string;
    codePreviewLabel: string;
    localeLabel: string;
    localeGroupLabel: string;
    accents: Record<AccentPreset, string>;
  };
  stores: Record<StoreKind, string>;
  errors: Record<ErrorCode, string>;
  relativeTime: {
    justNow: string;
    minutesAgo: string;
    hoursAgo: string;
    daysAgo: string;
  };
}

export const zhCN: Messages = {
  skipToMain: "跳到主内容",
  brandTitle: "CodeLab Studio",
  topbar: {
    homeTooltip: "返回课程主页",
    courseMenuLabel: "切换或导入 Course",
    courseMenuEmpty: "选择 Course",
    builtinGroupLabel: "内置 Course Pack",
    importedGroupLabel: "已导入本地 Course",
    importLocalAction: "导入本地 Markdown / Course Pack…",
    backToHomeAction: "返回课程主页",
    progressLabel: "实战 Exercise 进度",
    focusModeLabel: "专注模式",
    focusModeTooltip: "折叠 / 展开左侧手册（⌘/Ctrl + B）",
    envStatusLabel: "环境待检测",
    envStatusTooltip: "本机环境与四语言 Managed Runtime 状态",
    appearance: "外观",
    appearanceTooltip: "外观定制：双主题、8 组强调色、代码字号与界面语言",
    settingsLabel: "设置",
    settingsTooltip: "设置（ACP 助教与运行时）",
  },
  welcome: {
    ariaLabel: "课程主页",
    heroTitle: "开始今天的本地实战。",
    heroSubtitle:
      "打开本地实践手册或 Course Pack，在同一窗口里读文档、写代码、跑终端与自动验收。",
    primaryCtaContinue: "继续上次 Workspace：{title}",
    primaryCtaStart: "开始内置 Go / GoFrame 实战",
    secondaryCtaImport: "导入本地 Markdown / Course Pack",
    resumeSummaryPrefix: "最近进度 · {title}",
    localFirstSummary: "本地优先 · 零登录、离线完整可用",
    footerDesktop: "开源本地桌面端",
    footerLanguages: "支持 Go / JS·TS / Python / Java",
    footerTutor: "支持本机 ACP 助教",
    coursesHeading: "选择一门 Course 开始",
    filterGroupLabel: "按语言筛选",
    filterAll: "全部",
    recentHeading: "最近的 Workspace",
    newCoursePack: "+ 创作新 Course Pack",
    recentEmptyTitle: "暂无最近打开的 Workspace",
    recentEmptyHint:
      "从上方选择内置 Course 或导入本地 Course Pack 后，你的 Workspace 记录将显示在这里。",
    placeholderNotice: "「{target}」将在下一里程碑接入完整工作台流程。",
    courseTexts: {
      "go-goframe-practice": {
        title: "Go 与 GoFrame 开发实践",
        description: "从标准库 net/http 到并发测试与 GoFrame 工程迁移。",
      },
      "ts-data-pipeline": {
        title: "TypeScript 数据流水线实战",
        description: "多文件函数判题：筛选过滤、去重聚合与并发限流。",
      },
      "python-data-clean": {
        title: "Python 数据清洗与订单统计",
        description: "通过 uv 隔离环境运行多模块 pytest 自动化验收。",
      },
      "java-rate-limiter": {
        title: "Java 21 并发限流器实战",
        description: "Temurin JDK 21 与 JUnit 5 多线程安全验收。",
      },
    },
  },
  importModal: {
    dialogLabel: "导入本地 Course",
    title: "导入本地 Course",
    subtitle: "零改造解析本地单文件 Markdown（≤ 5 MB），或打开包含 codelab.json 的本地 Course Pack 目录。",
    close: "关闭导入窗口",
    modeGroupLabel: "选择导入方式",
    modeMarkdown: "单文件 Markdown",
    modePack: "本地 Course Pack 目录",
    pathLabelMarkdown: "本地 Markdown 文件路径",
    pathLabelPack: "本地 Course Pack 目录路径",
    pathPlaceholderMarkdown: "/path/to/handbook.md",
    pathPlaceholderPack: "/path/to/my-course-pack",
    browseFile: "浏览 .md 文件",
    browseDir: "浏览目录",
    hintMarkdown: "自动按 H2 切分 Chapter、推断 bash/sh/zsh Command Block，并严格清洗 HTML。",
    hintPack: "读取 codelab.json、入口 Markdown 与可选 .codelab-checks.json 验收清单。",
    cancel: "取消",
    submit: "打开只读手册",
    submitting: "解析中…",
    pathRequired: "请先输入或选择一个有效的本地绝对路径。",
  },
  workbench: {
    ariaLabel: "只读手册工作台",
    manualAriaLabel: "Course 实践手册",
    stageAriaLabel: "工作台右侧预览区",
    focusRailExpand: "展开手册（⌘/Ctrl + B）",
    focusRailLabel: "手册",
    outlineToggle: "章节大纲",
    outlineTitle: "章节大纲",
    outlineClose: "关闭章节大纲",
    outlineSummary: "共 {chapters} 节 · {exercises} 个实战 Exercise",
    prevChapter: "上一节",
    nextChapter: "下一节",
    chapterKindGuide: "概念导读",
    chapterKindExercise: "实战 Exercise",
    exerciseCounter: "练习 {current} / {total}",
    guideHeaderBadge: "导读",
    splitterTitle: "拖拽调整手册宽度，双击复原默认分栏",
    splitterAriaLabel: "调整手册与右侧工作区宽度",
    reloadTitle: "本地 Markdown 源文件已被外部修改",
    reloadDetail: "已检测到变更：{file}。点击刷新以同步最新手册内容并保留当前章节定位。",
    reloadAction: "立即刷新",
    reloadDismiss: "忽略",
    reloadedToast: "已刷新手册内容（{title}）",
    step1Title: "启动 / 测试",
    step2Title: "修改代码",
    step3Title: "验收清单",
    targetFileLabel: "目标文件",
    requiresDaemonLabel: "需运行常驻服务：{label}（端口 {port}）",
    secondaryCommandsTitle: "次要调试命令（{count}）",
    reflectionsTitle: "课后思考题",
    linearFallbackNotice: "纯标准 Markdown 教程 · 已按原文顺序呈现并提取可执行 Command Block",
    commandModeDaemon: "常驻 daemon",
    commandModeOneshot: "单次 oneshot",
    cwdWorkspaceRoot: "工作区根目录",
    copyCommand: "复制命令",
    copiedCommandToast: "已复制命令到剪贴板",
    copyFailedToast: "无法访问剪贴板，请手动复制。",
    readOnlyPreviewBadge: "只读预览",
    builtinBadge: "内置 Course Pack",
    localMarkdownBadge: "本地 Markdown",
    localPackBadge: "本地 Course Pack",
    stageTitle: "只读手册工作台",
    stageGuideHint: "当前为概念导读 Chapter（guide），不计入 Exercise 完成进度。",
    stageExerciseHint: "当前为实战 Exercise，已解析出以下结构化工作区上下文。",
    stageCommandsHeading: "已解析 Command Block",
    stageNoCommands: "本节未包含可执行 bash/sh/zsh 命令块。",
    stageChecksHeading: "验收条件概览",
    stageNoChecks: "本节暂无结构化 Check 声明。",
    stageMetadataHeading: "Chapter 元信息",
    stageChapterIdLabel: "稳定 chapterId",
    stageSourceLabel: "Course 来源",
    stageNextMilestoneNote: "本地 Workspace 绑定、多标签编辑器与 PTY 智能多终端将在后续里程碑接入右侧工作区。",
    checkKinds: {
      httpAssert: "HTTP 断言",
      commandTest: "命令测试",
      testSuite: "测试套件",
      selfCheck: "自查 Self-Check",
    },
  },
  appearance: {
    dialogLabel: "外观定制",
    title: "外观定制",
    close: "关闭外观抽屉",
    accentLabel: "强调色",
    accentGroupLabel: "选择强调色",
    copyConfig: "复制主题配置",
    copiedToast: "已复制主题配置（{theme} · {accent}）",
    copyFailedToast: "无法访问剪贴板，请手动复制配置。",
    themeLabel: "主题模式",
    themeGroupLabel: "切换主题模式",
    themeLight: "浅色 Light",
    themeDark: "深色 Dark",
    codeSizeLabel: "代码字号",
    codeSizeGroupLabel: "选择代码字号",
    sizeCompact: "紧凑 12.5",
    sizeStandard: "标准 13.5",
    sizeLarge: "大字 15",
    codePreviewLabel: "代码预览",
    localeLabel: "界面语言",
    localeGroupLabel: "切换界面语言",
    accents: {
      neutral: "中性灰 Neutral",
      violet: "紫罗兰 Violet",
      blue: "钴蓝 Blue",
      green: "翠绿 Green",
      amber: "琥珀 Amber",
      orange: "暖橙 Orange",
      coral: "珊瑚 Coral",
      rose: "玫红 Rose",
    },
  },
  stores: {
    settings: "全局偏好（settings）",
    recents: "最近 Workspace（recents）",
    trust: "信任列表（trust）",
  },
  errors: {
    INVALID_CODE_FONT_SIZE: "代码字号必须在 {min}px 至 {max}px 之间。",
    INVALID_WORKSPACE_PATH: "Workspace 路径必须为有效的本地绝对路径。",
    INVALID_COURSE_METADATA: "Course 标识与标题不能为空。",
    INVALID_PROGRESS_COUNT: "已完成 Exercise 数（{completed}）不能超过总数（{total}）。",
    INVALID_TRUST_SOURCE: "Course 来源标识无效，请提供有效的绝对路径或 Git 仓库地址。",
    COURSE_NOT_FOUND: "未找到指定的 Course：{source}",
    COURSE_PATH_INVALID: "Course 路径无效或超出课程目录边界：{path}",
    COURSE_MARKDOWN_TOO_LARGE: "Markdown 文件超过 {maxMb} MB 上限，已拒绝加载。",
    COURSE_MARKDOWN_INVALID: "无法读取该 Markdown 文件，请确认其为有效的 UTF-8 文本。",
    COURSE_MANIFEST_INVALID:
      "Course Pack 清单（codelab.json）缺失或格式无效，请检查必填字段与版本号。",
    COURSE_CHECKS_INVALID:
      "验收规则文件（.codelab-checks.json）格式无效，请检查字段与检查类型。",
    NO_ACTIVE_COURSE: "当前没有已打开的 Course 可供刷新。",
    STORAGE_READ_FAILED: "读取本地数据（{store}）失败，请检查目录权限后重试。",
    STORAGE_WRITE_FAILED: "保存本地数据（{store}）失败，请检查磁盘空间与写权限。",
    STORAGE_CORRUPTED: "检测到 {store} 配置文件损坏，已自动备份并恢复默认配置。",
    SHUTDOWN_FAILED: "退出回收部分后台资源时发生异常：{failures}",
    IPC_UNAVAILABLE: "桌面运行时连接不可用，请在 CodeLab Studio 桌面应用中打开。",
    UNEXPECTED_ERROR: "操作未能完成，请重试。",
  },
  relativeTime: {
    justNow: "刚刚",
    minutesAgo: "{n} 分钟前",
    hoursAgo: "{n} 小时前",
    daysAgo: "{n} 天前",
  },
};

import type {
  AccentPreset,
  AcpAgentPreset,
  CheckKind,
  ContextDimension,
  ContextQuoteSource,
  CourseLanguage,
  ErrorCode,
  MetricEventName,
  PermissionPolicy,
  StoreKind,
  HostTool,
  LanguageStatus,
  OptionalLsp,
  ProbeProblem,
  ProbeStatus,
  TeachingStyle,
  TutorPermissionKind,
  TutorPermissionStatus,
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
  verification: {
    run: string; running: string; results: string; empty: string; expected: string; actual: string;
    raw: string; complete: string; completed: string; reset: string; resetTitle: string; resetDetail: string; export: string;
    required: string; status: Record<"running" | "passed" | "failed" | "timedOut" | "skipped", string>;
  };
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
    tutorLabel: string;
    tutorTooltip: string;
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
  coursePack: {
    gitMode: string;
    gitUrl: string;
    gitHint: string;
    gitRefresh: string;
    gitBadge: string;
    gitLoading: string;
    gitTrustTitle: string;
    gitUrlRequired: string;
    authorMode: string;
    courseName: string;
    language: string;
    draftDirectory: string;
    draftHint: string;
    referenceSolution: string;
    createDraft: string;
    openDraft: string;
    draftTitle: string;
    sourceFile: string;
    sourceEditor: string;
    diagnostics: string;
    previewUnavailable: string;
    trial: string;
    trialTitle: string;
    returnDraft: string;
    trialHint: string;
    draftConflict: string;
    diagnosticReasons: Record<string, string>;
  };
  comparison: {
    referenceTrigger: string;
    replaceFile: string;
    starterTrigger: string;
    referenceTitle: string;
    starterTitle: string;
    currentFile: string;
    starter: string;
    chooseFile: string;
    selectFile: string;
    referenceEmpty: string;
    starterEmpty: string;
    referenceApply: string;
    starterApply: string;
    stale: string;
    draft: string;
  };
  execution: {
    run: string;
    changeCwd: string;
    cwd: string;
    cwdHint: string;
    resetCwd: string;
    confirmTitle: string;
    confirmDetail: string;
    confirmAction: string;
    terminal: string;
    terminals: string;
    newTerminal: string;
    clear: string;
    maximize: string;
    restore: string;
    close: string;
    stop: string;
    restart: string;
    daemon: string;
    running: string;
    exited: string;
    port: string;
    portConflict: string;
    showService: string;
    askAI: string;
    tutorUnavailable: string;
    copyErrors: string;
  };
  workspace: {
    languageStatus: Record<LanguageStatus, string>;
    choose: string;
    change: string;
    resume: string;
    chooseHint: string;
    trust: string;
    revokeTrust: string;
    trustTitle: string;
    previewHint: string;
    trustDetail: string;
    files: string;
    tabs: string;
    openFile: string;
    openChapterFile: string;
    loading: string;
    save: string;
    saved: string;
    unsaved: string;
    closeTab: string;
    editFile: string;
    changedOnDisk: string;
    deletedOnDisk: string;
    loadDisk: string;
    keepDraft: string;
    binary: string;
    tooLarge: string;
    missing: string;
    bytes: string;
    compare: string;
    pickComparison: string;
    comparisonName: string;
    comparisonUnavailable: string;
    endCompare: string;
    newFile: string;
    rename: string;
    filePath: string;
    emptyFolder: string;
    openIde: string;
    chooseIde: string;
    cancel: string;
    discard: string;
    switchTitle: string;
    saveAndSwitch: string;
    closeTitle: string;
    saveAndClose: string;
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
    terminalSplitterTitle: string;
    terminalSplitterAriaLabel: string;
    treeSplitterTitle: string;
    treeSplitterAriaLabel: string;
    tutorSplitterTitle: string;
    tutorSplitterAriaLabel: string;
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
    checkKinds: Record<CheckKind, string>;
  };
  runtime: {
    settingsTab: string;
    refresh: string;
    readyCount: string;
    installingCount: string;
    manage: string;
    close: string;
    install: string;
    installFor: string;
    retry: string;
    retryFor: string;
    preferManaged: string;
    preferFor: string;
    system: string;
    managed: string;
    installDir: string;
    optionalLsp: string;
    progressLabel: string;
    preparing: string;
    downloading: string;
    extracting: string;
    installed: string;
    status: Record<ProbeStatus, string>;
    problems: Record<ProbeProblem, string>;
    toolNames: Record<HostTool, string>;
    checking: string;
    lspGuidance: Record<OptionalLsp, string>;
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
  tutor: {
    settingsTab: string;
    protocolBadge: string;
    settingsIntro: string;
    connectionSectionTitle: string;
    handshakeSectionTitle: string;
    handshakeIdleHint: string;
    handshakeStatusIdle: string;
    handshakeStatusTesting: string;
    handshakeStatusOk: string;
    handshakeStatusFailed: string;
    policySectionHint: string;
    styleSectionHint: string;
    presetLabel: string;
    presets: Record<AcpAgentPreset, string>;
    commandLabel: string;
    commandPlaceholder: string;
    commandHint: string;
    policyLabel: string;
    policyGroupLabel: string;
    policies: Record<PermissionPolicy, { title: string; description: string }>;
    styleLabel: string;
    styleGroupLabel: string;
    styles: Record<TeachingStyle, string>;
    testHandshake: string;
    testingHandshake: string;
    handshakeOk: string;
    saveConfig: string;
    savedConfigToast: string;
    drawerTitle: string;
    drawerAriaLabel: string;
    closeDrawer: string;
    configureAgent: string;
    clearSession: string;
    emptyTitle: string;
    emptyHint: string;
    untrustedTitle: string;
    untrustedHint: string;
    noWorkspaceTitle: string;
    noWorkspaceHint: string;
    composerLabel: string;
    composerPlaceholder: string;
    send: string;
    stop: string;
    retry: string;
    streaming: string;
    starting: string;
    cancelled: string;
    reusedBadge: string;
    lazyIdleBadge: string;
    permissionTitle: string;
    permissionKinds: Record<TutorPermissionKind, string>;
    permissionStatuses: Record<TutorPermissionStatus, string>;
    outsideWorkspaceWarning: string;
    highRiskWarning: string;
    sensitiveBlockedDetail: string;
    allow: string;
    deny: string;
    fileChangedLabel: string;
    viewDiff: string;
    diffDialogTitle: string;
    diffBefore: string;
    diffAfter: string;
    openInEditor: string;
    newFileBefore: string;
    selection: {
      ariaLabel: string;
      explainConcept: string;
      explainCode: string;
      checkIssues: string;
      diagnoseError: string;
      ask: string;
    };
    quickPrompts: {
      explainConcept: string;
      explainCode: string;
      checkIssues: string;
      diagnoseError: string;
      checkFailure: string;
      verificationPanel: string;
      reflection: string;
      portConflict: string;
    };
    context: {
      quoteCardLabel: string;
      dismissQuote: string;
      sources: Record<ContextQuoteSource, string>;
      chipsGroupLabel: string;
      dimensions: Record<ContextDimension, string>;
      terminalErrorsChip: string;
      verificationChip: string;
      removeChip: string;
      previewChip: string;
      closePreview: string;
      restoreAll: string;
      tokenBudget: string;
      trimmedBadge: string;
      trimmedChipHint: string;
      trimmedEmptyPreview: string;
      sensitiveFilteredBadge: string;
      askAiAction: string;
      askAiReflection: string;
      askAiCheckFailure: string;
      askAiVerificationPanel: string;
    };
  };
  stores: Record<StoreKind, string>;
  settings: {
    returnToWorkspace: string;
    activeWorkspaceLabel: string;
    categoriesLabel: string;
    tabs: {
      tutor: string;
      runtime: string;
      editor: string;
      privacy: string;
    };
    editor: {
      intro: string;
      externalIdeLabel: string;
      externalIdeHint: string;
      shortcutsHeading: string;
      shortcutFocusMode: string;
      shortcutSaveFile: string;
      shortcutCloseOverlay: string;
    };
    privacy: {
      bannerTitle: string;
      bannerBody: string;
      zeroTelemetryBadge: string;
      trustHeading: string;
      trustHint: string;
      trustEmpty: string;
      trustKindLocal: string;
      trustKindGit: string;
      revokeTrust: string;
      trustRevokedToast: string;
      workspaceDataHeading: string;
      workspaceDataHint: string;
      noActiveWorkspaceHint: string;
      untrustedWorkspaceHint: string;
      clearTutorTitle: string;
      clearTutorHint: string;
      clearTutorAction: string;
      clearTutorToast: string;
      resetProgressTitle: string;
      resetProgressHint: string;
      resetProgressAction: string;
      resetProgressConfirm: string;
      resetProgressToast: string;
      metricsHeading: string;
      metricsHint: string;
      metricsPathLabel: string;
      metricsRefresh: string;
      metricsTotalLabel: string;
      metricsEmpty: string;
      metricsLoading: string;
      metricLabels: Record<MetricEventName, string>;
    };
  };
  errors: Record<ErrorCode, string>;
  relativeTime: {
    justNow: string;
    minutesAgo: string;
    hoursAgo: string;
    daysAgo: string;
  };
}

export const zhCN: Messages = {
  verification: {
    run: "运行验收", running: "验收中…", results: "验收结果", empty: "运行验收后在这里查看结果。", expected: "预期", actual: "实际",
    raw: "原始输出", complete: "标记完成", completed: "已完成", reset: "重置进度", resetTitle: "重置学习进度", resetDetail: "清除当前 Workspace 的完成记录和验收结果。代码会保留。", export: "导出学习记录",
    required: "必选", status: { running: "运行中", passed: "通过", failed: "失败", timedOut: "超时", skipped: "已跳过" },
  },
  skipToMain: "跳到主内容",
  brandTitle: "CodeLab Studio",
  topbar: {
    homeTooltip: "返回课程主页",
    courseMenuLabel: "切换或导入 Course",
    courseMenuEmpty: "选择 Course",
    builtinGroupLabel: "内置 Course Pack",
    importedGroupLabel: "已导入课程",
    importLocalAction: "导入本地 Markdown / Git 或创作课程包…",
    backToHomeAction: "返回课程主页",
    progressLabel: "实战 Exercise 进度",
    focusModeLabel: "专注模式",
    focusModeTooltip: "折叠 / 展开左侧手册（⌘/Ctrl + B）",
    envStatusLabel: "环境体检",
    envStatusTooltip: "本机环境与四语言 Managed Runtime 状态",
    appearance: "外观",
    appearanceTooltip: "外观定制：双主题、8 组强调色、代码字号与界面语言",
    tutorLabel: "AI 助教",
    tutorTooltip: "展开 / 收起本机 ACP 助教抽屉",
    settingsLabel: "设置",
    settingsTooltip: "设置（ACP 助教、四语言运行时、编辑器与桌面、隐私与本地数据）",
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
    title: "导入或创作课程包",
    subtitle: "打开 Markdown，导入本地或 Git 课程包，或开始创作草稿。",
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
  coursePack: {
    gitMode: "Git 仓库",
    gitUrl: "Git 仓库 URL",
    gitHint: "缓存仓库，授信前可预览课程语言和章节。",
    gitRefresh: "拉取上游更新",
    gitBadge: "Git Course Pack",
    gitLoading: "正在获取仓库…",
    gitTrustTitle: "信任此 Git Course Pack？",
    gitUrlRequired: "请输入 Git 仓库 URL。",
    authorMode: "创作课程包",
    courseName: "课程名称",
    language: "语言",
    draftDirectory: "课程包目录",
    draftHint: "选择空目录或输入新目录路径。",
    referenceSolution: "包含 Reference Solution",
    createDraft: "创建草稿",
    openDraft: "打开已有草稿",
    draftTitle: "课程包草稿",
    sourceFile: "源文件",
    sourceEditor: "源码编辑器",
    diagnostics: "Schema 诊断",
    previewUnavailable: "修复源文件诊断后可恢复课程预览。",
    trial: "试跑",
    trialTitle: "Trial Workspace",
    returnDraft: "返回草稿",
    trialHint: "保存源文件编辑，在临时 Workspace 中以学习者视角试跑课程。",
    draftConflict: "磁盘上的源文件已变更。请重新加载文件或保留编辑。",
    diagnosticReasons: {
      INVALID_JSON: "JSON 格式无效。",
      INVALID_STRUCTURE: "缺少必填项目或类型有误。",
      UNSUPPORTED_SCHEMA_VERSION: "不支持该 Schema 版本。",
      UNSUPPORTED_CHECK_TYPE: "不支持该 Check 类型。",
      EMPTY_REQUIRED_FIELD: "必填字段为空。",
      EMPTY_CHAPTER_ID: "必须填写章节 ID。",
      DUPLICATE_ID: "此标识符已被使用。",
    },
  },
  comparison: {
    referenceTrigger: "查看参考提示与 Diff",
    starterTrigger: "对比 Starter 恢复",
    referenceTitle: "Reference Solution",
    starterTitle: "从 Starter 恢复文件",
    currentFile: "当前 Workspace 文件",
    starter: "Starter",
    chooseFile: "要对比的文件",
    selectFile: "选择 Starter 文件",
    referenceEmpty: "当前章节没有 Reference Solution 文件。",
    starterEmpty: "当前课程没有可以恢复的 Starter 文件。",
    referenceApply: "应用 Reference Solution",
    starterApply: "恢复此文件",
    stale: "对比期间文件已更改。请关闭并重新打开对比，查看最新版本。",
    replaceFile: "确认后将用右侧版本替换所选文件。",
    draft: "包含未保存的草稿。",
  },
  execution: {
    run: "运行命令",
    changeCwd: "修改工作目录",
    cwd: "工作目录",
    cwdHint: "输入此 Workspace 内的目录。使用 . 表示 Workspace 根目录。",
    resetCwd: "使用课程目录",
    confirmTitle: "确认高危命令",
    confirmDetail: "此命令将以你的本机权限运行，可能造成破坏性更改。",
    confirmAction: "仍要运行",
    terminal: "终端",
    terminals: "Workspace 终端",
    newTerminal: "新建终端",
    clear: "清屏",
    maximize: "最大化终端",
    restore: "还原终端大小",
    close: "关闭终端",
    stop: "停止（中断）",
    restart: "重启",
    daemon: "Managed Daemon",
    running: "运行中",
    exited: "已退出（{code}）",
    port: "端口 {port}",
    portConflict: "本机端口已被占用。",
    showService: "切到服务终端",
    askAI: "问 AI",
    tutorUnavailable: "助教暂不可用。你可以复制终端报错以便排查。",
    copyErrors: "复制终端报错",
  },
  workspace: {
    languageStatus: {
      starting: "{language} LSP 启动中",
      ready: "{language} LSP 就绪",
      missing: "{language} LSP 缺失",
      degraded: "{language} LSP 已降级",
    },
    choose: "选择 Workspace",
    change: "切换 Workspace",
    resume: "继续使用 Workspace",
    chooseHint: "请选择本地目录开始编辑。课程提供 Starter 时，空目录会自动复制初始文件。",
    trust: "信任此课程",
    revokeTrust: "撤销信任",
    trustTitle: "信任此本地课程？",
    previewHint: "信任此课程后可打开 Workspace，并启用本地命令、验收、语言服务和助教。",
    trustDetail: "请仅信任你了解的来源。课程命令可使用当前用户的本地权限运行。",
    files: "文件",
    tabs: "已打开的文件",
    openFile: "从文件树打开一个文件。",
    openChapterFile: "打开 {path}",
    loading: "正在加载编辑器…",
    save: "保存",
    saved: "所有修改已保存",
    unsaved: "有未保存的修改",
    closeTab: "关闭 {path}",
    editFile: "编辑 {path}",
    changedOnDisk: "磁盘文件已更改",
    deletedOnDisk: "磁盘文件已删除",
    loadDisk: "加载磁盘版",
    keepDraft: "保留草稿",
    binary: "二进制文件",
    tooLarge: "文件超过 2 MB 编辑上限",
    missing: "文件已不存在",
    bytes: "{size} 字节",
    compare: "对比文件",
    pickComparison: "请从文件树或标签页选择另一份文本文件。",
    comparisonName: "{left} 与 {right}",
    comparisonUnavailable: "两份文件必须能以文本打开。",
    endCompare: "关闭对比",
    newFile: "新建文件",
    rename: "重命名文件",
    filePath: "Workspace 内路径",
    emptyFolder: "空目录",
    openIde: "在 IDE 中打开",
    chooseIde: "外部 IDE",
    cancel: "取消",
    discard: "放弃修改",
    switchTitle: "切换 Workspace 前如何处理未保存的修改？",
    saveAndSwitch: "保存并切换",
    closeTitle: "关闭文件前如何处理未保存的修改？",
    saveAndClose: "保存并关闭",
  },
  workbench: {
    ariaLabel: "只读手册工作台",
    manualAriaLabel: "Course 实践手册",
    stageAriaLabel: "工作台 Workspace 区域",
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
    terminalSplitterTitle: "拖拽调整终端与上方工作区高度，双击复原默认高度",
    terminalSplitterAriaLabel: "调整终端与上方工作区高度",
    treeSplitterTitle: "拖拽调整文件树宽度，双击复原默认宽度",
    treeSplitterAriaLabel: "调整文件树与编辑器宽度",
    tutorSplitterTitle: "拖拽调整 AI 助教宽度，双击复原默认宽度",
    tutorSplitterAriaLabel: "调整编辑器与 AI 助教宽度",
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
    checkKinds: {
      httpAssert: "HTTP 断言",
      commandTest: "命令测试",
      testSuite: "测试套件",
      selfCheck: "自查 Self-Check",
    },
  },
  runtime: {
    settingsTab: "四语言运行时",
    refresh: "重新探测",
    checking: "正在探测本机环境…",
    readyCount: "{ready}/{total}",
    installingCount: "安装中 · {count}",
    manage: "管理运行时",
    close: "关闭",
    install: "一键安装",
    installFor: "为 {language} 安装 Managed Runtime",
    retry: "重试",
    retryFor: "重试安装 {language} Managed Runtime",
    preferManaged: "优先托管版",
    preferFor: "为 {language} 优先使用 Managed Runtime",
    system: "系统",
    managed: "Managed Runtime",
    installDir: "隔离目录",
    optionalLsp: "可选 LSP",
    progressLabel: "{language} 安装进度",
    preparing: "正在准备安装",
    downloading: "正在下载 {artifact}",
    extracting: "正在解压 {artifact}",
    installed: "Managed Runtime 安装完成",
    status: { ok: "就绪", warn: "需检查", missing: "未找到" },
    problems: {
      versionTooOld: "版本过低",
      timedOut: "探测超时",
      failed: "探测失败",
      proxyDisabled: "模块代理已关闭",
      invalidProxy: "代理配置无效",
    },
    toolNames: {
      bun: "Bun", node: "Node.js", go: "Go", javac: "JDK",
      python3: "python3", python: "python", uv: "uv", uvPython: "Python（通过 uv）",
      git: "git", curl: "curl", goProxy: "GOPROXY",
      typeScriptLanguageServer: "typescript-language-server",
      gopls: "gopls", jdtls: "jdtls", pyright: "pyright",
    },
    lspGuidance: {
      typeScriptLanguageServer: "如需补全，请安装 typescript-language-server 和 typescript@6，并将服务加入 PATH。",
      gopls: "如需补全，可运行 go install golang.org/x/tools/gopls@latest，并将 gopls 加入 PATH。",
      jdtls: "如需补全，请安装 Eclipse JDT LS，并将 jdtls 启动脚本加入 PATH。",
      pyright: "如需补全，请安装 Pyright，并将 pyright-langserver 加入 PATH。",
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
  tutor: {
    settingsTab: "ACP 助教",
    protocolBadge: "ACP · JSON-RPC 2.0 / stdio",
    settingsIntro:
      "直接复用你本机已认证的 ACP 编码 Agent，仅在首次提问或握手测试时按需启动进程。",
    connectionSectionTitle: "ACP Agent 配置",
    handshakeSectionTitle: "握手测试与连接反馈",
    handshakeIdleHint: "点击下方按钮执行轻量 ACP initialize 握手，验证本机 CLI 可正常响应。",
    handshakeStatusIdle: "未测试",
    handshakeStatusTesting: "测试中…",
    handshakeStatusOk: "已连接",
    handshakeStatusFailed: "握手失败",
    policySectionHint: "默认采用引导只读策略，排查流畅无打断，同时拦截敏感文件与未授权写入/执行。",
    styleSectionHint: "控制助教回答问题的引导方式，也可在 AI 助教抽屉顶部随时切换。",
    presetLabel: "本机 ACP Agent 预设",
    presets: {
      claudeCode: "Claude Code（claude --acp）",
      geminiCli: "Gemini CLI（gemini --acp）",
      codexCli: "Codex CLI（codex --acp）",
      openCode: "OpenCode（opencode acp）",
      custom: "自定义 --acp 启动命令",
    },
    commandLabel: "启动命令",
    commandPlaceholder: "例如：my-agent --acp",
    commandHint: "切换 Agent 或启动命令后，将在下次发问时自动重建当前 Workspace 的 ACP 会话。",
    policyLabel: "Permission Policy 权限策略",
    policyGroupLabel: "选择 ACP 权限审批策略",
    policies: {
      guided_readonly: {
        title: "引导只读（推荐默认）",
        description:
          "自动放行当前 Workspace 内非敏感文件的只读请求；自动屏蔽 .env / *secret* / *key*；越界读取、所有文件写入与命令执行强制弹出审批卡。",
      },
      strict_ask_all: {
        title: "严格全审批",
        description:
          "除自动屏蔽敏感文件外，包括 Workspace 内文件读取在内的全部工具调用均须经对话流审批卡确认。",
      },
    },
    styleLabel: "教学风格",
    styleGroupLabel: "切换助教教学风格",
    styles: {
      socratic: "引导思考（苏格拉底）",
      direct: "直接解答",
    },
    testHandshake: "测试 ACP 握手",
    testingHandshake: "正在握手测试…",
    handshakeOk: "ACP 握手成功 · {agent}",
    saveConfig: "保存配置",
    savedConfigToast: "已保存 ACP 助教配置",
    drawerTitle: "AI 助教",
    drawerAriaLabel: "本机 ACP 助教抽屉",
    closeDrawer: "关闭 AI 助教抽屉",
    configureAgent: "配置 ACP 助教",
    clearSession: "清空对话",
    emptyTitle: "本机 ACP 助教已就绪",
    emptyHint:
      "采用严格懒启动：仅在你首次发问时拉起本机 ACP Agent，并在当前 Workspace 内复用同一会话。",
    untrustedTitle: "未信任 Course 已禁止启动 ACP 助教",
    untrustedHint: "请先在工作台授信当前 Course，再使用本机 ACP 编码 Agent。",
    noWorkspaceTitle: "请先绑定本地 Workspace",
    noWorkspaceHint: "选择一个本地 Workspace 目录后即可开启上下文感知 ACP 助教会话。",
    composerLabel: "向本机 ACP 助教提问",
    composerPlaceholder: "输入你的问题，Enter 发送，Shift+Enter 换行…",
    send: "发送",
    stop: "停止生成",
    retry: "重试",
    streaming: "正在生成回复…",
    starting: "正在启动本机 ACP Agent…",
    cancelled: "已停止生成",
    reusedBadge: "复用会话",
    lazyIdleBadge: "按需懒启动",
    permissionTitle: "ACP 权限审批",
    permissionKinds: {
      read: "读取文件",
      write: "写入文件",
      execute: "执行命令",
    },
    permissionStatuses: {
      pending: "等待审批",
      autoAllowed: "已自动放行（工作区只读）",
      approved: "已允许",
      denied: "已拒绝",
      blockedSensitive: "已自动屏蔽敏感文件",
      cancelled: "已取消",
    },
    outsideWorkspaceWarning: "越界访问：目标位于当前 Workspace 之外",
    highRiskWarning: "高危命令警告：可能对本机造成破坏性更改，请谨慎确认",
    sensitiveBlockedDetail: "命中敏感文件保护规则（.env / *secret* / *key*），已自动拦截。",
    allow: "允许本次",
    deny: "拒绝",
    fileChangedLabel: "ACP 已修改工作区文件",
    viewDiff: "查看修改 Diff",
    diffDialogTitle: "ACP 修改差异对照",
    diffBefore: "修改前",
    diffAfter: "修改后",
    openInEditor: "在编辑器中打开",
    newFileBefore: "（新文件）",
    selection: {
      ariaLabel: "划词快捷操作",
      explainConcept: "解释概念",
      explainCode: "解释代码",
      checkIssues: "查问题",
      diagnoseError: "诊断报错",
      ask: "提问",
    },
    quickPrompts: {
      explainConcept: "请解释这段概念，并结合当前章节说明它在实战中的作用。",
      explainCode: "请解释这段代码的作用与关键逻辑。",
      checkIssues: "请帮我检查这段代码的潜在问题或边界情况。",
      diagnoseError: "请帮我诊断这个报错的原因并给出排查方向。",
      checkFailure: "请帮我分析这条验收失败的原因与修复思路。",
      verificationPanel: "请结合当前验收结果帮我定位失败原因。",
      reflection: "我想和你聊聊这道课后思考题，请引导我深入思考。",
      portConflict: "终端提示本地端口冲突，请帮我排查占用原因与解决步骤。",
    },
    context: {
      quoteCardLabel: "引用上下文",
      dismissQuote: "移除引用卡片",
      sources: {
        manual: "手册选区",
        editor: "代码选区",
        terminal: "终端选区",
        checkFailure: "验收失败项",
        verificationPanel: "验收结果",
        reflection: "课后思考题",
        portConflict: "端口冲突",
      },
      chipsGroupLabel: "本轮 Context Snapshot 上下文",
      dimensions: {
        chapter: "当前章节",
        activeFile: "活跃文件",
        terminalErrors: "终端报错",
        verification: "验收状态",
      },
      terminalErrorsChip: "终端报错（{count} 行）",
      verificationChip: "验收 {summary}",
      removeChip: "移除 {name} 上下文",
      previewChip: "预览 {name} 上下文",
      closePreview: "关闭上下文预览",
      restoreAll: "恢复默认上下文",
      tokenBudget: "~{used} / {max} tok",
      trimmedBadge: "已按 6k 预算裁剪",
      trimmedChipHint: "已按 6,000 input tokens 优先级预算裁剪",
      trimmedEmptyPreview: "为保障高优先级选区与最新报错尾在 6,000 token 预算内完整送出，本维度内容已在本轮省略。",
      sensitiveFilteredBadge: "已屏蔽敏感文件",
      askAiAction: "问 AI",
      askAiReflection: "问 AI · 聊聊思考题",
      askAiCheckFailure: "问 AI 排查 {title}",
      askAiVerificationPanel: "问 AI 分析验收结果",
    },
  },
  stores: {
    workspace: "Workspace 绑定",
    progress: "Workspace 进度",
    metrics: "Workspace 指标",
    tutorHistory: "助教对话历史",
    runtimePreferences: "运行时偏好",
    settings: "全局偏好（settings）",
    recents: "最近 Workspace（recents）",
    trust: "信任列表（trust）",
  },
  settings: {
    returnToWorkspace: "返回 Workspace",
    activeWorkspaceLabel: "当前 Workspace",
    categoriesLabel: "设置分类",
    tabs: {
      tutor: "ACP 助教",
      runtime: "四语言运行时",
      editor: "编辑器与桌面",
      privacy: "隐私与本地数据",
    },
    editor: {
      intro: "主题、强调色、代码字号、默认外部 IDE 与界面语言会即时同步到顶部栏、外观抽屉、编辑器与课程主页。",
      externalIdeLabel: "默认外部 IDE",
      externalIdeHint: "点击工作区「在 IDE 中打开」时默认使用的本地编辑器。",
      shortcutsHeading: "桌面快捷键",
      shortcutFocusMode: "折叠 / 展开左侧手册（专注模式）",
      shortcutSaveFile: "保存当前活跃文件",
      shortcutCloseOverlay: "关闭当前抽屉、弹窗或菜单",
    },
    privacy: {
      bannerTitle: "100% 本地优先 · 零远程遥测",
      bannerBody:
        "CodeLab Studio 不上报任何远程埋点或云端分析。所有设置、Trust State、Tutor Session 对话历史与 Workspace 统计指标仅保存在本机磁盘，且本地指标严格不包含源码、原始命令、文件路径、URL 或聊天内容。",
      zeroTelemetryBadge: "零远程遥测",
      trustHeading: "已信任的外部 Course（Trust State）",
      trustHint: "内置 Course Pack 无需授信。撤销信任会立即锁定该外部 Course 的本地执行、验收与 ACP 助教权限。",
      trustEmpty: "当前没有已信任的外部本地课程或 Git Course Pack。",
      trustKindLocal: "本地路径",
      trustKindGit: "Git 仓库",
      revokeTrust: "撤销信任",
      trustRevokedToast: "已撤销外部 Course 信任",
      workspaceDataHeading: "当前 Workspace 本地数据清理",
      workspaceDataHint: "以下操作仅作用于当前活跃 Workspace 的 .codelab/ 元数据，不会删除你的代码文件。",
      noActiveWorkspaceHint: "打开一门课程并绑定本地 Workspace 后，可在此清空助教历史或重置练习进度。",
      untrustedWorkspaceHint: "当前外部 Course 尚未授信，无法操作其 Workspace 本地数据。",
      clearTutorTitle: "清空 Tutor Session 对话历史",
      clearTutorHint: "移除当前 Workspace 的全部本地助教对话记录并重置本机 ACP 会话。",
      clearTutorAction: "一键清空对话历史",
      clearTutorToast: "已清空当前 Workspace 的 Tutor Session 对话历史",
      resetProgressTitle: "重置 Exercise 学习进度",
      resetProgressHint: "清除当前 Workspace 的 Exercise 完成记录、自查勾选与验收结果，保留全部代码文件。",
      resetProgressAction: "重置当前练习进度",
      resetProgressConfirm: "确认重置进度",
      resetProgressToast: "已重置当前 Workspace 的 Exercise 进度",
      metricsHeading: "本地交互指标汇总",
      metricsHint: "仅记录结构化计数与枚举标签，按 Workspace 原子持久化至本地文件。",
      metricsPathLabel: ".codelab/metrics.json",
      metricsRefresh: "刷新统计",
      metricsTotalLabel: "累计记录：{count} 条",
      metricsEmpty: "绑定本地 Workspace 后将显示该工作区的 13 项本地交互指标计数。",
      metricsLoading: "正在读取本地指标…",
      metricLabels: {
        app_launched: "应用启动",
        course_opened: "打开 Course",
        preflight_executed: "环境体检",
        runtime_installed: "托管运行时安装",
        cmd_block_clicked: "运行 Command Block",
        cmd_risk_modal_action: "高危命令确认/取消",
        cmd_finished: "命令执行结束",
        file_tree_open: "展开文件树",
        selection_ai_clicked: "划词 / 失败项问 AI",
        ai_mode_switched: "切换教学风格",
        acp_permission_decision: "ACP 权限审批决策",
        verify_chapter_clicked: "点击运行验收",
        chapter_marked_done: "Exercise 标记完成",
      },
    },
  },
  errors: {
    COURSE_UNTRUSTED: "请先信任此课程，再打开 Workspace 或启用本地操作。",
    NO_ACTIVE_WORKSPACE: "请先选择一个活动的 Workspace。",
    WORKSPACE_PATH_OUTSIDE_ROOT: "“{path}” 不在 Workspace 可用路径内。",
    WORKSPACE_BOUND_TO_ANOTHER_COURSE: "此 Workspace 已绑定到其他课程。",
    WORKSPACE_FILE_NOT_FOUND: "找不到 “{path}”。",
    WORKSPACE_FILE_EXISTS: "“{path}” 已存在。",
    WORKSPACE_FILE_READ_FAILED: "无法读取 “{path}”。",
    WORKSPACE_FILE_WRITE_FAILED: "无法写入 “{path}”。",
    WORKSPACE_FILE_NOT_TEXT: "“{path}” 必须是 2 MB 以内的文本文件。",
    WORKSPACE_WATCH_FAILED: "无法监听 “{path}” 中的文件变化。",
    WORKSPACE_STARTER_FAILED: "无法复制 Starter。请检查初始文件和 Workspace 的权限。",
    EXTERNAL_IDE_OPEN_FAILED: "无法打开 {ide}。请确认 IDE 已安装并可启动。",
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
    TEST_SUITE_ADAPTER_UNAVAILABLE:
      "Test Suite {checkId} 需要声明受支持的 Course 语言（Go / JS/TS / Python / Java）。",
    JAVA_TOOLCHAIN_UNAVAILABLE:
      "未找到可用的 JDK 21（javac / java）。请安装本机 JDK 21，或在环境自检中安装托管 Java 运行时。",
    JAVA_JUNIT_LAUNCHER_UNAVAILABLE:
      "未找到 JUnit 5 Console Launcher JAR。请在环境自检中安装托管 Java 运行时，或设置 CODELAB_JUNIT_JAR。",
    JAVA_SOURCES_NOT_FOUND:
      "当前 Workspace 目录中未找到任何 .java 源文件。",
    GIT_COURSE_FETCH_FAILED: "无法获取 “{url}”。请检查 Git 是否已安装且仓库可以访问。",
    GIT_COURSE_CACHE_FAILED: "无法读写 Git 课程缓存。",
    NO_ACTIVE_GIT_COURSE: "请先打开 Git Course Pack。",
    NO_ACTIVE_DRAFT: "请先打开课程包草稿。",
    INVALID_DRAFT_PATH: "请选择应用数据和当前 Workspace 之外的课程包源目录。",
    DRAFT_DIRECTORY_NOT_EMPTY: "请选择空目录来创建课程包。",
    DRAFT_WRITE_FAILED: "无法创建课程包文件。",
    TRIAL_WORKSPACE_FAILED: "无法创建临时 Trial Workspace。",
    NO_ACTIVE_COURSE: "当前没有已打开的 Course 可供刷新。",
    STORAGE_READ_FAILED: "读取本地数据（{store}）失败，请检查目录权限后重试。",
    STORAGE_WRITE_FAILED: "保存本地数据（{store}）失败，请检查磁盘空间与写权限。",
    STORAGE_CORRUPTED: "检测到 {store} 配置文件损坏，已自动备份并恢复默认配置。",
    RUNTIME_TASK_START_FAILED: "无法启动运行时任务，请重试。",
    RUNTIME_PLATFORM_UNSUPPORTED: "当前平台没有可用的托管运行时。",
    RUNTIME_DOWNLOAD_FAILED: "运行时下载失败，请检查网络后重试。",
    RUNTIME_INTEGRITY_FAILED: "运行时校验失败，请重试下载。",
    RUNTIME_INSTALL_FAILED: "运行时安装失败，请检查磁盘空间与目录权限后重试。",
    VERIFICATION_BUSY: "验收正在运行，请等待当前验收结束。",
    CHECK_NOT_FOUND: "找不到此练习或验收项，请重新加载课程。",
    EXERCISE_COMPLETION_BLOCKED: "请先通过自动验收并勾选必选自查项。",
    DAEMON_PORT_CONFLICT: "端口 {port} 已被外部进程占用。请在外部停止该服务或更改课程端口后重试。",
    DAEMON_NOT_READY: "服务未就绪。请检查服务命令、端口与服务终端输出后重试。",
    EXECUTION_FAILED: "终端操作失败。",
    COMMAND_BLOCK_NOT_FOUND: "该命令块已不可用。请重新加载课程后重试。",
    COMMAND_CWD_INVALID: "工作目录「{path}」必须存在于当前 Workspace 内。",
    COMMAND_SHELL_UNAVAILABLE: "找不到 {shell}。在 Windows 上，Git for Windows 提供 bash 和 sh。",
    TERMINAL_NOT_FOUND: "该终端已不可用。",
    ACP_COMMAND_INVALID: "ACP 启动命令为空或引号格式无效，请检查设置。",
    ACP_AGENT_START_FAILED: "无法启动本机 ACP Agent 进程，请确认 CLI 已安装且命令可执行。",
    ACP_HANDSHAKE_FAILED: "ACP 握手超时或失败，请确认该命令支持 JSON-RPC 2.0 ACP 协议。",
    ACP_PROTOCOL_ERROR: "ACP 协议通信出错，请重试或重新配置 Agent。",
    ACP_PROCESS_EXITED: "ACP Agent 进程已意外退出，可点击重试重新连接。",
    TUTOR_PROMPT_EMPTY: "请输入要提问的内容。",
    TUTOR_BUSY: "助教正在生成回复，请等待完成或先停止当前生成。",
    TUTOR_PERMISSION_NOT_FOUND: "该权限审批请求已失效或已处理。",
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

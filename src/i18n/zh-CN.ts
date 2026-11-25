import type {
  AccentPreset,
  CheckKind,
  CourseLanguage,
  ErrorCode,
  StoreKind,
  HostTool,
  OptionalLsp,
  ProbeProblem,
  ProbeStatus,
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
  workspace: {
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
    checking: string;
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
    envStatusLabel: "环境体检",
    envStatusTooltip: "本机环境与四语言 Managed Runtime 状态",
    appearance: "外观",
    appearanceTooltip: "外观定制：双主题、8 组强调色、代码字号与界面语言",
    settingsLabel: "设置",
    settingsTooltip: "运行时设置",
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
  workspace: {
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
      typeScriptLanguageServer: "如需补全，请安装 typescript-language-server 和 typescript，并将服务加入 PATH。",
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
  stores: {
    workspace: "Workspace 绑定",
    progress: "Workspace 进度",
    metrics: "Workspace 指标",
    runtimePreferences: "运行时偏好",
    settings: "全局偏好（settings）",
    recents: "最近 Workspace（recents）",
    trust: "信任列表（trust）",
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
    NO_ACTIVE_COURSE: "当前没有已打开的 Course 可供刷新。",
    STORAGE_READ_FAILED: "读取本地数据（{store}）失败，请检查目录权限后重试。",
    STORAGE_WRITE_FAILED: "保存本地数据（{store}）失败，请检查磁盘空间与写权限。",
    STORAGE_CORRUPTED: "检测到 {store} 配置文件损坏，已自动备份并恢复默认配置。",
    RUNTIME_TASK_START_FAILED: "无法启动运行时任务，请重试。",
    RUNTIME_PLATFORM_UNSUPPORTED: "当前平台没有可用的托管运行时。",
    RUNTIME_DOWNLOAD_FAILED: "运行时下载失败，请检查网络后重试。",
    RUNTIME_INTEGRITY_FAILED: "运行时校验失败，请重试下载。",
    RUNTIME_INSTALL_FAILED: "运行时安装失败，请检查磁盘空间与目录权限后重试。",
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

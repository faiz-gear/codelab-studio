import type { AccentPreset, CourseLanguage, ErrorCode, StoreKind } from "../core/contracts";

export interface CoursePlaceholderCopy {
  id: string;
  language: CourseLanguage;
  title: string;
  description: string;
  totalExercises: number;
}

export interface Messages {
  skipToMain: string;
  brandTitle: string;
  topbar: {
    homeTooltip: string;
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
    resumeSummary: string;
    localFirstSummary: string;
    footerDesktop: string;
    footerLanguages: string;
    footerTutor: string;
    coursesHeading: string;
    filterGroupLabel: string;
    filterAll: string;
    courseCardHint: string;
    recentHeading: string;
    newCoursePack: string;
    recentEmptyTitle: string;
    recentEmptyHint: string;
    placeholderNotice: string;
    coursePlaceholders: readonly CoursePlaceholderCopy[];
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
    primaryCtaStart: "浏览内置 Course Pack",
    secondaryCtaImport: "导入 / 创作 Course Pack",
    resumeSummary: "最近进度 · {title}（{completed} / {total}）",
    localFirstSummary: "本地优先 · 零登录、离线完整可用",
    footerDesktop: "开源本地桌面端",
    footerLanguages: "支持 Go / JS·TS / Python / Java",
    footerTutor: "支持本机 ACP 助教",
    coursesHeading: "选择一门 Course 开始",
    filterGroupLabel: "按语言筛选",
    filterAll: "全部",
    courseCardHint: "即将接入只读手册工作台",
    recentHeading: "最近的 Workspace",
    newCoursePack: "+ 创作新 Course Pack",
    recentEmptyTitle: "暂无最近打开的 Workspace",
    recentEmptyHint:
      "从上方选择内置 Course 或导入本地 Course Pack 后，你的 Workspace 记录将显示在这里。",
    placeholderNotice: "「{target}」将在下一里程碑接入完整工作台流程。",
    coursePlaceholders: [
      {
        id: "go-goframe-practice",
        language: "Go",
        title: "Go 与 GoFrame 开发实践",
        description: "从标准库 net/http 到并发测试与 GoFrame 工程迁移。",
        totalExercises: 10,
      },
      {
        id: "ts-data-pipeline",
        language: "JS/TS",
        title: "TypeScript 数据流水线实战",
        description: "多文件函数判题：筛选过滤、去重聚合与并发限流。",
        totalExercises: 8,
      },
      {
        id: "python-data-clean",
        language: "Python",
        title: "Python 数据清洗与订单统计",
        description: "通过 uv 隔离环境运行多模块 pytest 自动化验收。",
        totalExercises: 6,
      },
      {
        id: "java-rate-limiter",
        language: "Java",
        title: "Java 21 并发限流器实战",
        description: "Temurin JDK 21 与 JUnit 5 多线程安全验收。",
        totalExercises: 7,
      },
    ],
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
    STORAGE_READ_FAILED: "读取本地数据（{store}）失败，请检查目录权限后重试。",
    STORAGE_WRITE_FAILED: "保存本地数据（{store}）失败，请检查磁盘空间与写权限。",
    STORAGE_CORRUPTED: "检测到 {store} 配置文件损坏，已自动备份并恢复默认配置。",
    SHUTDOWN_FAILED: "退出回收部分后台资源时发生异常：{failures}",
  },
  relativeTime: {
    justNow: "刚刚",
    minutesAgo: "{n} 分钟前",
    hoursAgo: "{n} 小时前",
    daysAgo: "{n} 天前",
  },
};

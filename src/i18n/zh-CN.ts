import type { MessageKey } from "./index";

export const zhCN = {
  "app.name": "CodeLab Studio",
  "shell.title": "CodeLab Studio",
  "shell.subtitle": "U0 前端验证壳 · Arc UI v3.2",
  "shell.locale.label": "语言",
  "shell.theme.label": "主题",
  "shell.theme.light": "浅色",
  "shell.theme.dark": "深色",
  "shell.accent.label": "强调色",
  "shell.viewport.label": "视口尺寸",
  "shell.status.offlineReady": "纯离线渲染就绪（零外部网络请求）",
  "error.unknown": "发生未知错误：{detail}",
  "error.invalidInput": "输入参数无效：{field}",
  "error.notReady": "当前环境尚未就绪，请稍后重试。",
} satisfies Record<MessageKey, string>;

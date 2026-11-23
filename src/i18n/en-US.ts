import type { MessageKey } from "./index";

export const enUS = {
  "app.name": "CodeLab Studio",
  "shell.title": "CodeLab Studio",
  "shell.subtitle": "U0 Frontend Verification Shell · Arc UI v3.2",
  "shell.locale.label": "Locale",
  "shell.theme.label": "Theme",
  "shell.theme.light": "Light",
  "shell.theme.dark": "Dark",
  "shell.accent.label": "Accent",
  "shell.viewport.label": "Viewport",
  "shell.epoch.label": "Session Epoch",
  "shell.ipc.label": "Desktop IPC",
  "shell.ipc.connecting": "Connecting to desktop core...",
  "shell.ipc.ready": "Desktop IPC ready",
  "shell.ipc.browser": "Browser preview mode",
  "shell.ipc.error": "Desktop IPC bootstrap failed",
  "shell.status.offlineReady": "Offline-ready (zero external network requests)",
  "error.unknown": "An unknown error occurred: {detail}",
  "error.invalidInput": "Invalid input: {field}",
  "error.notReady": "Environment is not ready yet. Please try again.",
} satisfies Record<MessageKey, string>;

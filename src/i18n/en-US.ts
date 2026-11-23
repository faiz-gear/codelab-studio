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
  "shell.status.offlineReady": "Offline-ready (zero external network requests)",
  "error.unknown": "An unknown error occurred: {detail}",
  "error.invalidInput": "Invalid input: {field}",
  "error.notReady": "Environment is not ready yet. Please try again.",
} satisfies Record<MessageKey, string>;

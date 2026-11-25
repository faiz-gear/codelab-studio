import { defineConfig } from "@playwright/test";

const baseURL = "http://127.0.0.1:1421";

export default defineConfig({
  testDir: "./src/__tests__/browser",
  testMatch: "**/*.browser.ts",
  use: {
    baseURL,
    browserName: "chromium",
    channel: process.env.CODELAB_BROWSER_CHANNEL,
    viewport: { width: 1440, height: 900 },
    trace: "retain-on-failure",
  },
  webServer: {
    command: "pnpm dev --host 127.0.0.1 --port 1421",
    url: baseURL + "/src/__tests__/browser/workspace.html",
  },
});

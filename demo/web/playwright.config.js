import { defineConfig, devices } from "@playwright/test";

const previewPort = Number.parseInt(process.env.RUVIZ_WEB_PREVIEW_PORT ?? "4173", 10);
if (!Number.isInteger(previewPort) || previewPort < 1 || previewPort > 65535) {
  throw new Error(`Invalid RUVIZ_WEB_PREVIEW_PORT: ${process.env.RUVIZ_WEB_PREVIEW_PORT}`);
}

const previewBaseUrl = `http://127.0.0.1:${previewPort}`;

export default defineConfig({
  testDir: "./tests",
  timeout: 120_000,
  expect: {
    timeout: 15_000,
  },
  fullyParallel: true,
  reporter: process.env.CI ? [["github"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: previewBaseUrl,
    viewport: { width: 1440, height: 1280 },
    deviceScaleFactor: 2,
    trace: "retain-on-failure",
  },
  webServer: {
    command: `bun run build && bunx vite preview --host 127.0.0.1 --port ${previewPort}`,
    url: previewBaseUrl,
    reuseExistingServer: !process.env.CI,
    timeout: 180_000,
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "firefox",
      use: { ...devices["Desktop Firefox"] },
    },
    {
      name: "webkit",
      use: { ...devices["Desktop Safari"] },
    },
  ],
});

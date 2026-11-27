import { defineConfig } from "@playwright/test";

const port = 4329;

export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  fullyParallel: false,
  workers: 1,
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    // Locally, use the Edge that ships with Windows (no browser download); CI installs Chromium.
    channel: process.env.CI ? undefined : "msedge",
  },
  webServer: {
    command: `node e2e/serve.mjs ${port}`,
    url: `http://127.0.0.1:${port}/`,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});

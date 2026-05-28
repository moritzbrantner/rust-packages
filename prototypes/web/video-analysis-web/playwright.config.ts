import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  workers: Number(process.env.PLAYWRIGHT_WORKERS ?? process.env.TEST_MAX_WORKERS ?? "2"),
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: "http://127.0.0.1:4175",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "VITE_HOST=127.0.0.1 VITE_PORT=4175 bun scripts/dev-with-rust-server.ts",
    url: "http://127.0.0.1:4175/",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});

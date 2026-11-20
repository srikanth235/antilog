import { defineConfig } from "@playwright/test";

// The smoke tests run against a built client (vite preview) and the server on its fakes.
// The server must be up on ANTILOG_API (default http://127.0.0.1:8080), or ANTILOG_SERVER_BIN names a
// binary to start (it needs DATABASE_URL and the environment in e2e/server.env.md).
const api = process.env.ANTILOG_API ?? "http://127.0.0.1:8080";
const bin = process.env.ANTILOG_SERVER_BIN;

export default defineConfig({
  testDir: "e2e",
  fullyParallel: false,
  workers: 1,
  reporter: [["list"]],
  use: { baseURL: "http://127.0.0.1:4173", trace: "off", screenshot: "off" },
  webServer: [
    {
      command: "pnpm build && pnpm preview --host 127.0.0.1",
      url: "http://127.0.0.1:4173",
      reuseExistingServer: true,
      timeout: 120_000,
      env: { ANTILOG_API: api },
    },
    ...(bin === undefined
      ? []
      : [
          {
            command: bin,
            url: `${api}/health`,
            reuseExistingServer: true,
            timeout: 60_000,
            env: {
              ANTILOG_FAKE_SIGN_IN: "1",
              ANTILOG_OPEN_ORGANIZATIONS: "1",
              ANTILOG_INSECURE_COOKIES: "1",
              ANTILOG_CODE_HOST_WEBHOOK_SECRET: "e2e-code-host-secret",
              ANTILOG_E2E_SEED: "1",
              ANTILOG_LISTEN: new URL(api).host,
            },
          },
        ]),
  ],
  projects: [
    {
      name: "light",
      use: {
        browserName: "chromium",
        colorScheme: "light",
        launchOptions: {
          executablePath: process.env.PLAYWRIGHT_CHROMIUM ?? "/opt/pw-browsers/chromium",
        },
      },
    },
    {
      name: "dark",
      use: {
        browserName: "chromium",
        colorScheme: "dark",
        launchOptions: {
          executablePath: process.env.PLAYWRIGHT_CHROMIUM ?? "/opt/pw-browsers/chromium",
        },
      },
    },
  ],
});

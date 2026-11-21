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
              // The connector tests call a mock MCP server on loopback and seal a credential.
              ANTILOG_CONNECTORS_ALLOW_PRIVATE: "1",
              ANTILOG_SECRET_MASTER_KEY:
                "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
              // The model-key tests seal a key under their own master.
              ANTILOG_SECRET_MODEL_KEY_MASTER:
                "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100",
              // The model picker lists OpenCode's models from its own lists; tests stay offline.
              ANTILOG_OPENCODE_URL: "http://127.0.0.1:1",
              ANTILOG_OPENCODE_GO_URL: "http://127.0.0.1:1",
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

import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The server serves its routes at the root; the app calls /api/* on its own origin.
// The dev server and the preview server forward /api/* to the local server.
const target = process.env.ANTILOG_API ?? "http://127.0.0.1:8080";
const proxy = {
  "/api": { target, changeOrigin: false, rewrite: (path: string) => path.replace(/^\/api/, "") },
};

export default defineConfig({
  plugins: [react()],
  server: { port: 5173, proxy, fs: { allow: [".."] } },
  preview: { port: 4173, proxy },
  build: { sourcemap: false },
  test: { environment: "node", include: ["src/**/*.test.ts"], expect: { requireAssertions: true } },
});

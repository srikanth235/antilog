# Antilog web app

React 19, Base UI, TanStack Router and Query, over the generated client in `src/api`. Styling comes from `design/mockups/` (`tokens.css`, `icons.css`, `logos.css`, `components.css`).

```bash
pnpm install --frozen-lockfile
pnpm dev          # http://localhost:5173; /api/* goes to ANTILOG_API (default http://127.0.0.1:8080)
pnpm typecheck && pnpm lint && pnpm test && pnpm build
pnpm e2e          # needs the server up (or ANTILOG_SERVER_BIN) with ANTILOG_PLATFORM=fake ANTILOG_FAKE_SIGN_IN=1 ANTILOG_OPEN_ORGANIZATIONS=1 ANTILOG_INSECURE_COOKIES=1
```

The server serves its routes at the root; the app calls `/api/*` on its own origin and the dev and preview servers strip the prefix. `src/api/overrides.ts` restates routes the committed OpenAPI document gets wrong (a duplicate operation id, free-form objects, query parameters filed as path parameters) and is deleted when the server fixes them. `.pnpmfile.cjs` gives openapi-typescript its own TypeScript 5.9.3, because TypeScript 7 has no compiler API for it. A part the API does not serve yet (the Changes, Terminal, Preview and Files panels, the best value of an experiment, wins) says so on the screen and is marked "(target)" in the code.

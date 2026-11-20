# Checks

## Today

On this branch the Rust checks below exist, and so does one for a design change:

```
python3 design/tools/tokens.py
python3 design/tools/tokens.py --check
```

The web app has its own checks, from `web/`: `pnpm install --frozen-lockfile`, `pnpm typecheck`, `pnpm lint`, `pnpm test`, `pnpm build` and `pnpm e2e` (Playwright against the server on its fakes; CI jobs `web-client` and `web-e2e`).

A doc-only change has no command: read the links and fix every statement the change made false. The rows marked (target) have no command yet. Say so in the report; never call an unrun layer verified.

## Target

One gate command with profiles (local, pull request, nightly, release) is the target, `cargo xtask gate --profile <name>`. The design check moves into it as `cargo xtask design` (see [design mockups](../design/mockups.md)). Until the gate exists, run the commands for the layer. The language rules and exact flags are in the stack skill for each layer.

| Layer | Run | Stack |
| --- | --- | --- |
| Rust (`antilog-core`, `antilog-server`, `antilog-proxy`, `antilog`) | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace` (`cargo test --doc` for doctests), `cargo deny check` (all four exist; CI job `rust`) | [stack/rust/antilog.md](../stack/rust/antilog.md) |
| An API type, route or event | the Rust checks, `cargo run -q -p antilog-server -- openapi | diff - openapi/antilog.json`, then `pnpm generate:api` in `web/` leaves no diff (CI jobs `rust` and `web-client`); see [change/api-endpoint.md](../change/api-endpoint.md) | [stack/rust/antilog.md](../stack/rust/antilog.md) |
| The CLI, an Antilog tool or `antilog mcp` | the Rust checks, then the CLI test script against a built `antilog` and a test server | [stack/rust/antilog.md](../stack/rust/antilog.md), [stack/mcp/antilog.md](../stack/mcp/antilog.md) |
| TypeScript (the web app, the generated client) | `pnpm install --frozen-lockfile`, `pnpm typecheck`, `pnpm lint`, `pnpm test`, `pnpm build`, `pnpm e2e` (all exist) (the browser tests are `@playwright/test`; never run `playwright install`, see [stack/playwright/antilog.md](../stack/playwright/antilog.md)) | [stack/typescript/antilog.md](../stack/typescript/antilog.md) |
| Swift (the macOS app) | `xcodebuild -project Antilog.xcodeproj -scheme Antilog -destination 'platform=macOS' test`, on macOS only | [stack/swift/antilog.md](../stack/swift/antilog.md) |
| The store: a migration or query | the storage suite against a Postgres container (`antilog_core::testing::store_suite`), `cargo sqlx prepare --workspace --check`, migrations applied to an empty database and to the last release | [stack/postgres/antilog.md](../stack/postgres/antilog.md) |
| The judge: the acceptance criteria, the seal checks, anti-cheat | the Rust checks, the acceptance criteria as one pure function with property tests (`proptest`) and a table of known cases, the seal checks (baseline, noise, planted cheats) against the fake session machine, and a test that a trial that changes a protected path is Rejected; see [judge/change-acceptance.md](../judge/change-acceptance.md) and [judge/anti-cheat.md](../judge/anti-cheat.md) | [stack/rust/antilog.md](../stack/rust/antilog.md) |
| A change to `.github/workflows/`, a new dependency, or a trust-boundary change | the checks of the layer, plus a review with the skill in [stack/security/antilog.md](../stack/security/antilog.md); a new dependency also `cargo deny check` | [stack/security/antilog.md](../stack/security/antilog.md) |
| A design board or token | `python3 design/tools/tokens.py`, then `python3 design/tools/tokens.py --check` (works today) | [design mockups](../design/mockups.md) |
| A doc only | read the links; fix every statement the change made false | |

Docs that change with the code: [change](../change/SKILL.md) says which.

## Fix the code, not the check

Never loosen a lint, a test, a snapshot or an allowlist to get green. A snapshot changes only when the behaviour it records was meant to change, and the diff is reviewed (`cargo insta review`). A number or budget moves only when the owner agrees.

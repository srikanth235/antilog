# Checks

## Today

On this branch there is one real check, for a design change:

```
python3 design/tools/tokens.py
python3 design/tools/tokens.py --check
```

A doc-only change has no command: read the links and fix every statement the change made false. Everything else below is target. Say so in the report; never call an unrun layer verified.

## Target

One gate command with profiles (local, pull request, nightly, release) is the target, `cargo xtask gate --profile <name>`. The design check moves into it as `cargo xtask design` (see [design mockups](../design/mockups.md)). Until the gate exists, run the commands for the layer. The language rules and exact flags are in the stack skill for each layer.

| Layer | Run (all target) | Stack |
| --- | --- | --- |
| Rust (`ratchet-core`, `ratchet-server`, `ratchet`) | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace` (`cargo test --doc` for doctests), `cargo deny check` | [stack/rust/ratchet.md](../stack/rust/ratchet.md) |
| An API type, route or event | the Rust checks, `cargo xtask openapi --check`, the generated TypeScript client up to date; see [change/api-endpoint.md](../change/api-endpoint.md) | [stack/rust/ratchet.md](../stack/rust/ratchet.md) |
| The CLI, a Ratchet tool or `ratchet mcp` | the Rust checks, then the CLI test script against a built `ratchet` and a test server | [stack/rust/ratchet.md](../stack/rust/ratchet.md), [stack/mcp/ratchet.md](../stack/mcp/ratchet.md) |
| TypeScript (the web app, the generated client) | `pnpm install --frozen-lockfile`, `pnpm typecheck`, `pnpm lint`, `pnpm test` (the browser tests are `@playwright/test`; never run `playwright install`, see [stack/playwright/ratchet.md](../stack/playwright/ratchet.md)) | [stack/typescript/ratchet.md](../stack/typescript/ratchet.md) |
| Swift (the macOS app) | `xcodebuild -project Ratchet.xcodeproj -scheme Ratchet -destination 'platform=macOS' test`, on macOS only | [stack/swift/ratchet.md](../stack/swift/ratchet.md) |
| The Helm chart or an OpenTofu module (target) | `helm lint`, `helm template` against each cloud's values, `tofu fmt -check` and `tofu validate`; an install on a local k3s cluster in CI | [stack/kubernetes/ratchet.md](../stack/kubernetes/ratchet.md) |
| The store: a migration or query | the storage suite against a Postgres container, `cargo sqlx prepare --check`, migrations applied to an empty database and to the last release | [stack/postgres/ratchet.md](../stack/postgres/ratchet.md) |
| An agent platform adapter | the Rust checks, and the platform test suite against the fake platform | [stack/rust/ratchet.md](../stack/rust/ratchet.md) |
| The judge: the acceptance criteria, the seal checks, anti-cheat | the Rust checks, the acceptance criteria as one pure function with property tests (`proptest`) and a table of known cases, the seal checks (baseline, noise, planted cheats) against the fake judge sandbox, and a test that a trial that changes a protected path is Rejected; see [judge/change-acceptance.md](../judge/change-acceptance.md) and [judge/anti-cheat.md](../judge/anti-cheat.md) | [stack/rust/ratchet.md](../stack/rust/ratchet.md) |
| A change to `.github/workflows/`, a new dependency, or a trust-boundary change | the checks of the layer, plus a review with the skill in [stack/security/ratchet.md](../stack/security/ratchet.md); a new dependency also `cargo deny check` | [stack/security/ratchet.md](../stack/security/ratchet.md) |
| A design board or token | `python3 design/tools/tokens.py`, then `python3 design/tools/tokens.py --check` (works today) | [design mockups](../design/mockups.md) |
| A doc only | read the links; fix every statement the change made false | |

Docs that change with the code: [change](../change/SKILL.md) says which.

## Fix the code, not the check

Never loosen a lint, a test, a snapshot or an allowlist to get green. A snapshot changes only when the behaviour it records was meant to change, and the diff is reviewed (`cargo insta review`). A number or budget moves only when the owner agrees.

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
| Rust (`ratchet-core`, `ratchet-server`, `ratchet-cloud`, `ratchet-runner`, `ratchet`) | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace` (`cargo test --doc` for doctests), `cargo deny check`, and `cargo build -p ratchet-core --target wasm32-unknown-unknown` (the core must build for wasm) | [stack/rust/ratchet.md](../stack/rust/ratchet.md) |
| An API type, route or event | the Rust checks, `cargo xtask openapi --check`, the generated TypeScript client up to date; see [change/api-endpoint.md](../change/api-endpoint.md) | [stack/rust/ratchet.md](../stack/rust/ratchet.md) |
| The CLI or its MCP tools | the Rust checks, then the CLI contract script against a built `ratchet` and a test server | [stack/rust/ratchet.md](../stack/rust/ratchet.md) |
| TypeScript (the web app, the generated client) | `pnpm install --frozen-lockfile`, `pnpm typecheck`, `pnpm lint`, `pnpm test` | [stack/typescript/ratchet.md](../stack/typescript/ratchet.md) |
| Swift (the macOS app) | `xcodebuild -project Ratchet.xcodeproj -scheme Ratchet -destination 'platform=macOS' test`, on macOS only | [stack/swift/ratchet.md](../stack/swift/ratchet.md) |
| The store: a migration or query (Postgres and SQLite) | the storage suite against both stores (`cargo test` with a Postgres container, and SQLite), `cargo sqlx prepare --check`, each dialect's migrations applied to an empty database and to the last release, the two dialects' tables and columns compared | [stack/postgres/ratchet.md](../stack/postgres/ratchet.md) |
| Infra: OpenTofu, Packer, Kubernetes manifests | `tofu fmt -check -recursive`, `tofu init -backend=false`, `tofu validate`, `tofu test`; `packer fmt -check` and `packer validate`; the backend contract suite | [stack/infra/ratchet.md](../stack/infra/ratchet.md) |
| A design board or token | `python3 design/tools/tokens.py`, then `python3 design/tools/tokens.py --check` (works today) | [design mockups](../design/mockups.md) |
| A doc only | read the links; fix every statement the change made false | |

Docs that change with the code: [change](../change/SKILL.md) says which.

## Fix the code, not the check

Never loosen a lint, a test, a snapshot or an allowlist to get green. A snapshot changes only when the behaviour it records was meant to change, and the diff is reviewed (`cargo insta review`). A number or budget moves only when the owner agrees.

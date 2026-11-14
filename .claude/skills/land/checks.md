# Checks

There is no single gate command yet. One command with profiles (local, pull request, nightly, release) is (target). Until then, run the commands below for the kind of change. They come from the README and `.github/workflows/`.

Set `export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0` first.

| Kind of change | Run |
| --- | --- |
| Any Rust | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast` |
| An API type, route or event | the Rust checks, then `cargo run -p ratchetd -- openapi` and `cargo test -p ratchetd`; see [change/api-endpoint.md](../change/api-endpoint.md) |
| The CLI or its MCP tools | `cargo build -p ratchet-cli -p ratchetd`, then `bash crates/ratchet-cli/tests/contract.sh target/debug/ratchet target/debug/ratchetd` |
| The ledger or a migration | `cargo test -p ratchet-ledger` |
| The engine or a demo | `cargo test -p ratchet-engine`; the end-to-end demos: build the demos, then `RATCHET_E2E=1 cargo test -p ratchet-engine --test engine runtime_simulation -- --nocapture --test-threads=1` and `target/debug/ratchetd simulate --project search` (or `forecast`, `cli`) |
| A design board or token | `python3 design/tools/tokens.py`, then `python3 design/tools/tokens.py --check` |
| TypeScript demos and infra tests | `pnpm install --frozen-lockfile && pnpm build && pnpm typecheck && pnpm test` |
| OpenTofu | `cd infra/aws && tofu fmt -check -recursive && tofu init -backend=false -input=false && tofu validate && tofu test` |
| Packer images | `packer fmt -check -diff infra/images` and the validate step in [ci.md](ci.md) |
| A doc only | read the links; fix every statement the change made false |

## Fix the code, not the check

Never loosen a lint, a test, a snapshot or an allowlist to get green. A snapshot changes only when the behaviour it records was meant to change, and the diff is reviewed (`cargo insta review`). A number or budget moves only when the owner agrees.

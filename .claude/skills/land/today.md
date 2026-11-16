# Land: the earlier code's checks and CI

This page is reference from the earlier Rust daemon on the branch `claude/peaceful-lamport-leak6p`. None of it runs on this branch. The target pages are [checks.md](checks.md), [tests.md](tests.md), [ci.md](ci.md) and [real-cloud.md](real-cloud.md).

## Commands

Set `export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0` first. There was no single gate command.

| Kind of change | Ran |
| --- | --- |
| Any Rust | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast` |
| An API type, route or event | the Rust checks, then `cargo run -p ratchetd -- openapi` and `cargo test -p ratchetd` |
| The CLI or its MCP tools | `cargo build -p ratchet-cli -p ratchetd`, then `bash crates/ratchet-cli/tests/contract.sh target/debug/ratchet target/debug/ratchetd` |
| The ledger or a migration | `cargo test -p ratchet-ledger` |
| The engine or a demo | `cargo test -p ratchet-engine`; the demos with `RATCHET_E2E=1 cargo test -p ratchet-engine --test engine runtime_simulation -- --nocapture --test-threads=1` and `target/debug/ratchetd simulate --project search` (or `forecast`, `cli`) |
| TypeScript demos and infra tests | `pnpm install --frozen-lockfile && pnpm build && pnpm typecheck && pnpm test` |
| OpenTofu | `cd infra/aws && tofu fmt -check -recursive && tofu init -backend=false -input=false && tofu validate && tofu test` |
| Packer images | `packer fmt -check -diff infra/images` and the validate step in CI |

## CI

Workflows were in `.github/workflows/`.

| Workflow | When | What it ran |
| --- | --- | --- |
| `ci.yml` | every push and pull request | `node` (pnpm install, build, typecheck, test), `infra` (OpenTofu 1.8.5 fmt, init, validate, test in `infra/aws`), `demos` (the three demos through `runtime_simulation` with `RATCHET_E2E=1`, then `ratchetd simulate`) |
| `native.yml` | push and pull request, with path filters | `rust` on Ubuntu and macOS: fmt, clippy, tests, then the CLI contract script |
| `images.yml` | push and pull request touching `Cargo.toml`, `crates/**`, `templates/**`, `infra/images/**`; by hand | static launcher and runner binaries per architecture, then `packer fmt` and `packer validate` (Packer 1.14.2, no AWS calls) |
| `harness-canary.yml` | by hand only | installed the latest agent CLIs, recorded fresh transcripts, ran the conformance tests. The nightly schedule was commented out on purpose: it spent provider credit |

Only CI could run the macOS Rust job and the Packer validation.

## Tests

- Simulation mode: `ratchetd serve --mode sim --ledger :memory:` ran the whole product on one machine with scripted agents, mock AWS and a local git remote; `ratchetd simulate --project <search|forecast|cli>` exited 0 on a verified win. A developer and demo tool, never proof of a real cloud.
- Ports with a mock: every outside system sat behind a port in `crates/ratchet-core/src/ports.rs` with mocks in `ports::mock`; `crates/ratchet-agents/src/scripted.rs` was the scripted agent.
- Contract tests: `crates/ratchetd/tests/api/contract.rs` round-tripped every route and a meta-test failed on a route with no case; `crates/ratchetd/tests/api/openapi.rs` checked the committed `api/openapi.yaml`.
- Snapshots: `insta` in `crates/ratchetd/tests/api/snapshots/` and several crates. Properties: `proptest` in `ratchet-core`, `ratchet-bench`, `ratchet-ledger`.
- CLI: `crates/ratchet-cli/tests/cli/` in process; `crates/ratchet-cli/tests/contract.sh` on a built binary.
- Secrets: `secrets_never_leave` in `crates/ratchetd/tests/api/real_service.rs`.
- Each crate had a `PARITY.md` mapping tests of the retired TypeScript it replaced.

## Real environment

Notes were `docs/REAL_ENV_TESTING.md` and `docs/real-env/` (`aws.md`, `bench.md`, `agents.md`, `engine.md`, `github.md`). Some were written for the retired TypeScript engine. The order was: local agents without AWS, then `aws.md` (apply, destroy, kill switch, reaper), `bench.md`, `agents.md`, `engine.md`. Nothing in the evaluator sandbox had been exercised on a real AWS account.

## Commit subjects

From the earlier log: `F6: contrast check calibrated on real pictures`, `P5b: steering field height`, `CI: cache the Rust build`, `Merge F6: app polish after P5b`. No pull request number suffix.

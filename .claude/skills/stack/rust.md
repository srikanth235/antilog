# Rust

## Constraints in this repo

- Edition 2024, `rust-version = "1.97"` (`Cargo.toml`). The workspace is `crates/*`. CI uses the stable toolchain with `rustfmt` and `clippy`.
- `unsafe_code = "forbid"` is set under `[lints.rust]` in `ratchet-bench`, `ratchet-connectors`, `ratchet-engine`, `ratchet-env` and `ratchet-sandbox`. Do not weaken it. Other crates have no such lint; do not add `unsafe` there without the owner.
- Set these for local builds, as the README does. The AWS SDK makes full debug info huge.

```bash
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
```

- `Cargo.lock` is not committed (CI comments say so). Do not add it as a side effect.
- Dependencies are declared per crate. The workspace manifest has no shared dependency table. See [libraries.md](libraries.md).

## Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo test -p ratchetd                # the contract tests and snapshots of the API
cargo insta review                    # review changed snapshots
```

Fix the code, not the check. Never add `#[allow]` or loosen a lint to get green.

## Tests

- Every external system (AWS, GitHub, the agent CLIs, git) sits behind a port in `crates/ratchet-core/src/ports.rs` with a real and a mock implementation (`ports::mock`). Code depends on the port, never on the real thing. A new outside system is a new port, with both.
- Snapshots use `insta`. Properties use `proptest` (`crates/ratchet-core/tests/properties.rs`, `crates/ratchet-bench/tests/properties.rs`, `crates/ratchet-ledger/tests/properties.rs`). How tests are written: [land/tests.md](../land/tests.md).
- Each crate has a `PARITY.md` mapping the tests of the code it replaced to Rust tests. Keep it true.

## Shape

- Errors in libraries are `thiserror` types. `anyhow` is used in the binaries (`ratchet-cli`, `ratchetd`).
- Async is `tokio`. Async traits use `async-trait`.
- The daemon's HTTP layer is `axum`, behind the `server` feature of `ratchetd`. Routes stay thin; see [change/api-endpoint.md](../change/api-endpoint.md).

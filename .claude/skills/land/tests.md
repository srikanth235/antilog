# Tests

- **Simulation mode.** `ratchetd serve --mode sim --ledger :memory:` runs the whole product on one machine: scripted agents, mock AWS, a local git remote. `ratchetd simulate --project <search|forecast|cli>` runs a demo through the real engine and exits 0 on a verified win. Simulation is a developer and demo tool. It shows the code works, not that a real cloud does.
- **Ports with a real and a mock implementation.** Every outside system sits behind a port in `crates/ratchet-core/src/ports.rs`, with mocks in `ports::mock`. Tests use the mock. Example: `crates/ratchet-engine/tests/engine/common.rs` builds an engine over mock ports; `crates/ratchet-agents/src/scripted.rs` is the scripted agent.
- **Contract tests.** `crates/ratchetd/tests/api/contract.rs` parses every route's response into its Rust type and checks it round-trips; a meta-test fails when a route has no case. `crates/ratchetd/tests/api/openapi.rs` checks the committed `api/openapi.yaml` is current. See `api/CONTRACT.md`.
- **Snapshots.** `insta` in `crates/ratchetd/tests/api/snapshots/` (routes, schemas, the event union) and in `ratchet-core`, `ratchet-bench`, `ratchet-agents`, `ratchet-sandbox`, `ratchet-env`. Review a change with `cargo insta review`.
- **Properties.** `proptest` in `crates/ratchet-core/tests/properties.rs`, `crates/ratchet-bench/tests/properties.rs` and `crates/ratchet-ledger/tests/properties.rs`.
- **Ledger.** In-memory ledger per test; `crates/ratchet-ledger/tests/migration.rs` checks migrations.
- **CLI.** `crates/ratchet-cli/tests/cli/` runs commands in process; `crates/ratchet-cli/tests/contract.sh` runs the same contract on a built binary.
- **No test asserts on a wall-clock duration.** Assert on counts.
- **Secrets.** A test that sets a key or token checks that no response, event or error carries it (`secrets_never_leave` in `crates/ratchetd/tests/api/real_service.rs`).
- Each crate has a `PARITY.md` mapping the tests of the TypeScript it replaced to Rust tests. Update it when a mapped test moves.
- Never skip, ignore or delete a test to get green.

# Change: the earlier daemon

This page keeps what the earlier code taught. It is on the branch `claude/peaceful-lamport-leak6p`, not on this branch, and it is reference only: the daemon `ratchetd`, a unix socket, one per-boot bearer token, SQLite. Paths below are from that branch. The target pages are [api-endpoint.md](api-endpoint.md), [write-action.md](write-action.md), [live-event.md](live-event.md), [ledger-table.md](ledger-table.md) and [runner-job.md](runner-job.md).

## An API endpoint

`api/openapi.yaml` was generated from the Rust types, and `api/CONTRACT.md` held the contract and its checks.

1. Request and response types in `crates/ratchetd/src/api/`. The handler in `crates/ratchetd/src/routes/` with `#[utoipa::path]`, thin: it decoded, called one method of the `AppService` trait (`crates/ratchetd/src/service.rs`) and encoded. The method was implemented for the simulation (`crates/ratchetd/src/sim/`) and for the real service (`crates/ratchetd/src/real/`).
2. `cargo run -p ratchetd -- openapi` rewrote `api/openapi.yaml`; `--check` exited 1 when it differed.
3. `cargo test -p ratchetd` failed on a stale file and round-tripped every route (`crates/ratchetd/tests/api/`). `cargo insta review` reviewed snapshot diffs.
4. CLI: `crates/ratchet-cli/src/cli.rs` and `commands/`. Every command took `--json`; a spending command needed `--yes`. The name went in `crates/ratchet-cli/tests/contract.sh`.
5. MCP: `ratchet mcp` generated a tool per CLI command from the clap definition (`crates/ratchet-cli/src/mcp.rs`). The chat's own tools were in `crates/ratchetd/src/mcp/tools.rs`.
6. No TypeScript client was generated.
7. Every route needed the bearer token (`crates/ratchetd/src/auth.rs`).

Lesson: one generated document plus a test that walks the route table caught most drift. Keep that.

## A write

The handler called one `AppService` method (`create_optimization`, `stop_optimization`). The method changed the ledger and the engine in `crates/ratchetd/src/real/service.rs` or `sim/`, then broadcast an `ApiEvent` through `AppService::subscribe` to `EventHub::publish` (`crates/ratchetd/src/hub.rs`). There was no actor, role or audit entry. Keeping a write in one service method made moving it behind an action layer one change.

## A live event

- One stream, `GET /v1/events`. `ApiEvent` in `crates/ratchetd/src/api/events.rs`; the SSE `event:` was its kebab-case `type`, `id:` a per-boot sequence number.
- Kinds: `hello` and `resync` (control), then `optimization`, `agent-event`, `candidate`, `ratchet`, `spend`, `notice`, `run-active`, `sandbox`, `environment-draft`, `connector`, `github`.
- `EventHub` numbered events, kept a window and replayed on `Last-Event-ID`; tests for replay, gaps and the bridge from a service. A restart gave `resync`.
- The union was snapshotted (`crates/ratchetd/tests/api/snapshots/api__openapi__api_event.snap`); the stream tested in `crates/ratchetd/tests/api/sse.rs`. A chat had its own stream (`ChatStreamEvent`, `/v1/chats`).

## A ledger table

- Crate `crates/ratchet-ledger`, SQLite through sqlx 0.9 (`runtime-tokio`, `sqlite`, `migrate`, `macros`), runtime-checked queries so building needed no database.
- Migrations in `crates/ratchet-ledger/migrations/` (`0001_init.sql`, `0002_chats.sql`, `0003_billing_value.sql`), run by `sqlx::migrate!` when the ledger opened (`src/store.rs`). New numbered file for each change, never an edit.
- Traits in `crates/ratchet-core/src/ledger.rs`, rows in `crates/ratchet-core/src/rows.rs`, mapping in `crates/ratchet-ledger/src/rowmap.rs`.
- Tests used an in-memory ledger (`Ledger::open_memory`). `tests/migration.rs` checked the schema and that opening twice applied each migration once. `tests/properties.rs` held proptests. Run: `cargo test -p ratchet-ledger`.
- Each crate had a `PARITY.md` mapping tests of the retired TypeScript engine to Rust tests.
- Only `ratchet_moves` and `spend_events` were append-only by trigger.

Lesson: the two append-only tables were the ones whose rows people trusted most; the target extends that to the audit log and receipts.

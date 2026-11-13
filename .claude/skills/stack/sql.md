# SQL

## Today

The ledger is SQLite through `sqlx` 0.9 with the `runtime-tokio`, `sqlite`, `migrate` and `macros` features (`crates/ratchet-ledger/Cargo.toml`). Queries are runtime-checked, so building needs no database. `macros` is there only for `sqlx::migrate!`. `ratchetd` also uses `sqlx` with `sqlite`.

- Migrations: numbered files in `crates/ratchet-ledger/migrations/`. Add a new file; never edit a shipped one. How: [change/ledger-table.md](../change/ledger-table.md).
- Foreign keys are on. The busy timeout is 5 seconds (`crates/ratchet-ledger/src/store.rs`).
- Check: `cargo test -p ratchet-ledger`. Tests use an in-memory ledger.
- Keep SQL in the ledger crate. Other crates use the traits in `crates/ratchet-core/src/ledger.rs`.
- Never store what can be derived (product principle 6). Value, return and status are computed from measured results.

## Target

(target) Postgres is the only state of the server, through `sqlx` with the `postgres` feature. Every row carries its organization; each module owns its tables; no joins across modules. Write SQL that is plain enough to move. See [libraries.md](libraries.md).

# A ledger table

## How the ledger works today

- The ledger is the crate `crates/ratchet-ledger`, SQLite through `sqlx` 0.9 (`runtime-tokio`, `sqlite`, `migrate`, `macros`). Queries are runtime-checked, so building needs no database.
- Migrations are in `crates/ratchet-ledger/migrations/` (`0001_init.sql`, `0002_chats.sql`, `0003_billing_value.sql`). `sqlx::migrate!("./migrations")` runs them when the ledger opens (`crates/ratchet-ledger/src/store.rs`).
- Add a table or column as a new numbered file. Never edit one that has shipped.
- The ledger implements the traits declared in `crates/ratchet-core/src/ledger.rs`. Row types are in `crates/ratchet-core/src/rows.rs`. Mapping is in `crates/ratchet-ledger/src/rowmap.rs`.
- Tests open an in-memory ledger (`Ledger::open_memory`, see `crates/ratchet-ledger/tests/common/`). `crates/ratchet-ledger/tests/migration.rs` checks the schema and that opening twice applies each migration once. `tests/properties.rs` holds proptests.
- Run: `cargo test -p ratchet-ledger`.
- Each crate has a `PARITY.md`. Keep `crates/ratchet-ledger/PARITY.md` true if a test moves.

## Target rules

(target) These hold once the server product and Postgres exist. Design new tables to fit them now where it costs nothing.

- Every row carries its organization.
- Each of the three modules (optimization, context, infrastructure) owns its own tables. The floor owns members, roles, tokens and the audit log.
- No joins across modules. A module reads another through its API.
- Postgres is the only state. SQLite is today's store, not the target.

# SQLite in a Durable Object (notes)

Source: Ratchet's own notes from the [SQLite documentation](https://www.sqlite.org/docs.html) (public domain) and Cloudflare's [SQLite storage API](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/) and [Durable Object limits](https://developers.cloudflare.com/durable-objects/platform/limits/) pages, read November 2026. No popular agent skill covers SQLite as a server's store, so this page is written here. It is for the cloud store only: one SQLite database per organization inside its Durable Object (target). [`../ratchet.md`](../ratchet.md) wins where they differ, and the platform itself is in [`../../cloudflare/general/durable-objects/SKILL.md`](../../cloudflare/general/durable-objects/SKILL.md).

The shared schema in [`../ratchet.md`](../ratchet.md) already avoids most of SQLite's surprises: text ids, integer milliseconds, JSON as text, states as text with a `check`. This page covers what is left.

## Tables

- **Declare tables `STRICT`.** By default SQLite stores any value in any column (type affinity), so a text in an integer column is kept silently. `STRICT` tables reject it, which is how Postgres behaves. Allowed types are then `INTEGER`, `REAL`, `TEXT`, `BLOB` and `ANY`; the shared schema needs only `INTEGER` and `TEXT`.
- **Text primary keys.** A table with a text id still has a hidden integer `rowid` and a separate index on the id. That is fine for most tables. For a small, hot table keyed only by id, `WITHOUT ROWID` saves the second lookup; measure before using it.
- **Every constraint the Postgres side has**, written the SQLite way: `not null`, `check`, `unique`, foreign keys with a deliberate `on delete`. A `check` on a state lists the same values as the Postgres one; the test that compares the lists covers both.
- **Append-only tables use a trigger** that raises on update and delete: `create trigger audit_no_update before update on audit_log begin select raise(abort, 'append-only'); end;` and the same for delete.
- **No `now()` in SQL.** Times come from Rust as integer milliseconds, so both stores record the same value and tests can fix the clock.

## Foreign keys

SQLite turns foreign key enforcement off unless `PRAGMA foreign_keys = on` is set per connection. D1 documents that it enforces them by default; the Durable Object pages say nothing. (target) The first storage test inserts a row with a missing parent and expects an error. If it does not get one, stop and settle it before writing more schema.

## Transactions

- `sql.exec` cannot run `BEGIN`, `COMMIT` or `SAVEPOINT`. A transaction is the storage API's `transactionSync` (or `transaction`) around the writes. Each call without one is its own transaction.
- Keep a transaction synchronous: no `await`, no fetch, no call to the runner or a model inside it. The object runs one request at a time between awaits, so a short synchronous block is also the concurrency control. There is no `SKIP LOCKED` and none is needed.
- (target) Whether workers-rs exposes `transactionSync` with the same guarantee is one of the questions the two-day Rust trial answers ([`../../rust/ratchet.md`](../../rust/ratchet.md)).

## Limits to design for

| Limit | Value | What it means here |
| --- | --- | --- |
| Storage per object | 10 GB | One organization's whole history. Prune finished jobs and old events in batches, as on Postgres. A full database fails writes with `SQLITE_FULL`: map it to a clear error, and alert before it happens using `databaseSize` |
| Bound parameters per query | 100 | A multi-row insert of 10 columns fits 10 rows. Batch inside one transaction instead of building one huge statement |
| Statement length | 100 KB | Never inline values; bind them |
| Columns per table | 100 | Not a concern for ledger tables |
| String, BLOB or row size | 8 MB | Ledger rows are small. A large document does not belong in the database at all |
| CPU per request | 30 s by default | Long work (an export, a big backfill) is split across alarms, each doing a bounded batch |

Billing counts rows read and written, and every index on a written column adds a written row. Index what the queries filter and sort on, and no more. `rowsRead` and `rowsWritten` on a cursor show what a query cost; use them in tests of hot queries.

## Migrations

- The SQLite directory has the same numbers as the Postgres one, and the object applies pending migrations on start, before it serves a request, inside one transaction each. The applied number is stored in a table the object reads first.
- SQLite's `alter table` can add a column and rename one, and little else. Changing a column's type or a constraint means the documented table rebuild: create the new table, copy, drop the old, rename, recreate indexes and triggers, all in one transaction, then check foreign keys (`pragma foreign_key_check`). Prefer expand and contract with added columns, as on Postgres, so a rebuild is rare.
- A rebuild of a large table can exceed one request's CPU. Test it at the size limits you design for.

## Numbers across the JavaScript boundary

Values pass through JavaScript between the SQLite engine and Rust, where a plain number is a 64-bit float. Integers above 2^53 can lose precision. Milliseconds since the epoch and money in minor units are far below that. (target) Check in the Rust trial how workers-rs maps `INTEGER`, and keep a test that round-trips the largest value each integer column may hold.

## Queries

- Read the plan with `explain query plan`; a `SCAN` of a large table on a hot path needs an index.
- Page with a keyset cursor (`where (created_at, id) > (?, ?) order by created_at, id limit ?`), never `offset`.
- JSON functions exist (`json_extract` and friends) but the shared schema stores JSON whole and never queries into it. Do not start.
- `returning` works (SQLite 3.35 and later), so an insert or a claim can return its row as on Postgres.

## Point-in-time recovery

Each object keeps 30 days of history. A restore is per object, so per organization: get a bookmark for a time, ask for it on next start, restart the object. A restore is an operator action with an audit entry, never something the application does on its own, and it never crosses into another organization.

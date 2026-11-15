# Postgres and SQL in Ratchet

(target) Postgres is the self-hosted store, reached through `sqlx` with the `postgres` feature. The cloud store is SQLite in a Durable Object (see "SQLite in the cloud" below). Queries live behind the `Store` trait in the Postgres store crate; the core never writes SQL. Nothing of it is on this branch. The earlier daemon used SQLite through `sqlx` with runtime-checked queries; that is not the rule here, and its migrations are reference only. Versions are in [libraries.md](../libraries.md): Postgres 18, `sqlx` 0.9 (and `sqlx-cli`), `testcontainers` for tests.

## Workflow and done criteria

```bash
sqlx migrate run                         # apply migrations to the local database
cargo sqlx prepare --workspace --check   # (target) offline query data is up to date
cargo nextest run -p <crate>             # (target) database tests, against Postgres in a container
```

**Done** when a new migration applies to an empty database and to one at the previous version, the offline query data is regenerated and committed, the tests pass, and a changed query has had its plan read ([general/postgres-best-practices.md](general/postgres-best-practices.md)). How a table or migration travels through the layers: [change/ledger-table.md](../../change/ledger-table.md).

## What the database holds, and does not

- **Ids, versions, numbers, states and audit rows only.** Ratchet stores no customer data. It never holds a dataset's rows, a holdout row, a connector secret, a model key or a token in the clear. The server never connects to customer data; the runner and the agents do. If a column would hold something a customer would call their data, stop and ask.
- **Never store what can be derived** (product principle 6). Value, Return and status are computed from measured results, not written as columns that can drift. If a derived value must be stored for speed, it is a cache with a rebuild path and a test that recomputes it.
- **The audit log is append-only.** Rows are inserted, never updated or deleted by the application. A job's state history, a verdict and a receipt are recorded, not overwritten. Tokens are stored as hashes. Receipts are signed ([libraries.md](../libraries.md)).
- **Every row carries its organization.** Each table that belongs to an organization has `organization_id`, indexed (usually the first column of its composite indexes), and every query filters by it. A query that forgets the organization is a data leak between customers. Prefer a function or a typed repository per module that takes the organization as an argument so it cannot be forgotten.
- **Each module owns its tables.** No joins across modules; a module asks another through its Rust API. Write SQL plain enough to move.
- **Row-level security is not used for end users.** There is one database client, the server, which enforces access in code ([organization skill](../../organization/SKILL.md)). Do not give the database users that mirror Ratchet's members. A policy on each organization table is an extra Postgres-only guard for a self-hosted server with several organizations; the code never relies on it, and the cloud has none because each organization has its own database.

## sqlx

- **Compile-time-checked queries.** Use `query!`, `query_as!` and `query_scalar!` so a wrong column or type fails the build. Commit the offline query data (`.sqlx/`) with every change to a query, and build CI with `SQLX_OFFLINE=true` so the build needs no database. `cargo sqlx prepare` regenerates it against a migrated database. Use the runtime `query()` only where a query is built dynamically, and then keep it small and tested.
- **Embedded migrations** run at start-up with `sqlx::migrate!`, before the server reports ready. In a multi-replica deployment only one replica applies them at a time (sqlx takes an advisory lock for this).
- **One pool**, configured at start (`max_connections`, `acquire_timeout`, `idle_timeout`, `max_lifetime`); see [general/async-services.md](../rust/general/async-services.md) and the connection rules in the general digest. Never hold a connection across a call to another service.
- **Transactions are explicit and short.** `begin()`, do the writes, `commit()`. Do not call out to the network, the runner or a model inside one. A transaction left uncommitted is rolled back on drop, but end it explicitly.
- **Map the errors.** Unique violation is a conflict (409), foreign key violation is a bad reference (422), serialization failure and deadlock mean retry the whole transaction (bounded, with backoff), a pool timeout is overload (503), the rest is internal (500, with the cause in the log).
- **Dynamic SQL is built with `QueryBuilder` and binds**, never string formatting of values. Identifiers are never taken from user input.
- **Keep SQL in the persistence code of its module**, behind a trait that the rest of the code uses. Tests of other modules use the in-memory double, not the database.

## Migrations: forward only

- Numbered files in `migrations/`, one change each. **Never edit a shipped migration** and never write a down migration: to undo, write a new forward one. A shipped migration is one that has been applied anywhere outside your machine.
- **Expand, then contract.** Add the new column or table, deploy code that writes both and reads the new, backfill, then drop the old in a later migration, never in the same release. The server and the runner can be a version apart (the runner is installed in the customer's infrastructure and upgrades on its own schedule), so a migration must not break the previous server version that is still serving during a rollout.
- **Be safe on a live table.** Set `lock_timeout` so a migration fails fast instead of queueing everyone behind it. Add indexes with `create index concurrently`, which cannot run in a transaction: put the statement alone in its file with the `-- no-transaction` directive at the top. Add constraints `not valid`, then `validate constraint` in a later step. Backfill big tables in batches, in a separate migration or job from the schema change. Details: [general/postgres-best-practices.md](general/postgres-best-practices.md).
- **Test the migration.** A test applies all migrations to an empty database and checks that the schema matches what the queries expect; the offline query check also catches drift. A migration that rewrites data has a test with sample rows.
- Foreign keys are on, and every foreign key column has an index. Use plain `snake_case` lowercase names, unquoted.

## The shared schema

Both stores run the same logical schema, so a type must exist in both.

- Ids are `text` (UUIDv7 made in Rust). Times are integer milliseconds since the epoch. JSON is `text`. States are `text` with a `check`. Money is integer minor units with a currency column.
- No Postgres-only type (`uuid`, `timestamptz`, `jsonb`, `numeric`, arrays, enums) in the shared schema. Use the plain type above even where Postgres has a better one.
- Migrations are numbered the same in both dialects, one directory per dialect, written twice. A number that exists in one directory and not the other fails CI. A test applies each directory to an empty database and compares the tables and columns.
- Append-only tables (audit log, events, verdicts, receipts, spend) are enforced by triggers in both dialects.
- A Postgres-only addition (row-level security, a partial index) goes in a Postgres-only migration file that changes no table the other dialect has.
- One storage test suite runs against both stores in CI ([land/tests.md](../../land/tests.md)).

## Types and ids

- Ids are text holding a UUIDv7, made in Rust (`uuid` with the `v7` feature) so a test can fix them, and time-ordered so inserts are not scattered across the index. The owner confirms the id scheme when the first schema lands (target). Never expose a sequential id that leaks counts to a customer.
- Times are integers, milliseconds since the epoch, UTC. Money and spend are integer minor units with a currency column, never a float. Counts and versions that must add up are integers.
- States are `text` with a `check` constraint, never a Postgres enum, so both dialects read the same. Mirror them as a Rust enum with an exhaustive `match`; a test lists the values in both and compares them.
- JSON is stored as text, only for a document the server stores whole and does not query into (a receipt payload, a spec). Anything you filter or join on is a column. Do not store a secret in it.
- Constrain in the database what the database can know: `not null`, `check`, `unique`, foreign keys with a deliberate `on delete`.

## The job queue

The queue is a table in the store: Postgres self-hosted, the organization's Durable Object in the cloud. No Redis, no message bus ([libraries.md](../libraries.md)). The claim query below is the Postgres one; in the cloud the object is the only writer, so a claim is a plain update in one transaction.

- **Claim with `FOR UPDATE SKIP LOCKED`**, in one statement that also marks the row, so concurrent workers each take a different job and none waits:

  ```sql
  update jobs
     set state = 'running', claimed_by = $1, lease_expires_at = now() + $2
   where id = (
     select id from jobs
      where state = 'queued' and run_after <= now()
      order by priority desc, created_at
      limit 1
      for update skip locked)
  returning *;
  ```

- **A claim is a lease.** A worker renews it while it works. A reaper puts jobs whose lease expired back to `queued` (counting the attempt), so a worker that dies does not hold a job forever. Jobs are therefore delivered **at least once**: the handler must be idempotent, keyed by the job id.
- **Runners pull.** The runner in the customer's infrastructure polls (or long-polls) for jobs over HTTPS and reports progress; the server does not connect to it. The claim is the same query, scoped to that runner's organization and capabilities. Results come back as ids, numbers and states, not customer data.
- **Bounded and observable.** Cap attempts, record the last error (without secrets), move a job that keeps failing to a `failed` state with a reason, and expose queue depth and oldest-job age as metrics. Keep a partial index for the hot path: `create index on jobs (priority desc, created_at) where state = 'queued'`.
- **Keep it small.** Autovacuum settings on the queue table are tightened because rows churn ([general/postgres-best-practices.md](general/postgres-best-practices.md)); finished jobs older than a retention period are moved or deleted in batches. `LISTEN/NOTIFY` may wake a worker sooner, but polling must still work if a notification is missed.
- **Use a transaction-scoped advisory lock** for singleton work (one scheduler, one reaper) rather than a dummy row.

## SQLite in the cloud (target)

- Ratchet cloud keeps one SQLite database per organization, in the built-in storage of one Durable Object per organization. The Worker reaches it only through that object, which is the only writer.
- The SQLite `Store` implements the same trait as the Postgres one and runs the SQLite migration directory. No `sqlx` here: the object's own SQL API is used.
- One writer means no row-level security, no `SKIP LOCKED` and no `NOTIFY`. The object holds live streams and timers (alarms) itself.
- Limits to design for: 10 GB per organization, about 1,000 requests a second per object, 30-day point-in-time recovery built in. Keep transactions short and page by keyset cursor.
- Residency: an object can be pinned to the EU, the US or FedRAMP (Durable Object jurisdictions).
- Foreign keys are on for each connection and every statement is parameterised, as in Postgres.
- Moving between cloud and self-hosted is an export and import of one neutral file format, not a database copy.
- Details (strict tables, foreign keys, transactions, limits, migrations, numbers across JavaScript): [general/sqlite.md](general/sqlite.md).

## Tests

- Run database tests against a real Postgres 18 in a container (`testcontainers`), one database per test or per module with a clean schema, not a mock. The earlier daemon's tests used an in-memory SQLite ledger; there is no in-memory stand-in for Postgres. The same storage suite also runs against the SQLite store, and a store test that passes on one and fails on the other is a schema or query bug.
- Test the claim query with concurrent workers (no job taken twice, none skipped forever), lease expiry, and idempotent re-delivery.
- Test that every organization-scoped query cannot see another organization's rows.

## Pitfalls

- **A migration that locks a hot table** blocks the whole server. Check what lock a statement takes before shipping it, and set `lock_timeout`.
- **`OFFSET` pagination** gets slower with depth; use keyset cursors ([general/postgres-best-practices.md](general/postgres-best-practices.md)).
- **A long-lived transaction** holds locks and stops vacuum. Set `idle_in_transaction_session_timeout`.
- **Unindexed foreign keys** make deletes and joins scan. Run the missing-index query in review.
- **`timestamp` without time zone, `float` for money, `varchar(n)` for nothing** are the usual type mistakes: see the types above.
- **Enum drift.** Adding a state in Rust without the migration (or the reverse) is caught by the test that compares the lists; keep that test.

## General guidance

[`general/sqlite.md`](general/sqlite.md): our own notes on SQLite in a Durable Object, from SQLite's and Cloudflare's documentation.

[`general/postgres-best-practices.md`](general/postgres-best-practices.md): query performance, indexes, connections and pooling, schema design, locking and concurrency, data access, diagnostics and migrations. It is a digest of [Supabase's postgres-best-practices](https://github.com/supabase/agent-skills) (MIT, [`licenses/supabase-agent-skills-LICENSE`](licenses/supabase-agent-skills-LICENSE)).

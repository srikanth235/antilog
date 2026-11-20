# Postgres and SQL in Antilog

Postgres is the only store: managed Postgres on Antilog cloud, and the client's own when self-hosted. It is reached through `sqlx` with the `postgres` feature. Queries live behind the `Store` trait in the Postgres store crate; the core never writes SQL. Nothing of it is on this branch. Versions are in [libraries.md](../libraries.md): Postgres 18, `sqlx` 0.9 (and `sqlx-cli`), `testcontainers` for tests.

## Workflow and done criteria

```bash
sqlx migrate run                         # apply migrations to the local database
cargo sqlx prepare --workspace --check   # offline query data is up to date
cargo nextest run -p <crate>             # database tests, against Postgres in a container (Docker needed)
```

**Done** when a new migration applies to an empty database and to one at the previous version, the offline query data is regenerated and committed, the tests pass, and a changed query has had its plan read ([general/postgres-best-practices.md](general/postgres-best-practices.md)). How a table or migration travels through the layers: [change/ledger-table.md](../../change/ledger-table.md).

## What the database holds, and does not

- **Ids, versions, numbers, states, commit ids, the conversation's words, spend and audit rows only.** Antilog stores no customer data. It never holds a dataset's rows, a holdout row, a connector secret, a model key or a token in the clear. The server never connects to customer data; the session machines do. If a column would hold something a customer would call their data, stop and ask.
- **Never store what can be derived** (product principle 6). Value, Return and status are computed from measured results, not written as columns that can drift. If a derived value must be stored for speed, it is a cache with a rebuild path and a test that recomputes it.
- **The audit log is append-only.** Rows are inserted, never updated or deleted by the application. A job's state history, a verdict and a receipt are recorded, not overwritten. Tokens are stored as hashes. Receipts are signed ([libraries.md](../libraries.md)).
- **Every row carries its organization.** Each table that belongs to an organization has `organization_id`, indexed (usually the first column of its composite indexes), and every query filters by it. A query that forgets the organization is a data leak between customers. Prefer a function or a typed repository per module that takes the organization as an argument so it cannot be forgotten.
- **Each module owns its tables.** No joins across modules; a module asks another through its Rust API. Write SQL plain enough to move.
- **Row-level security is not used for end users (the members).** There is one database client, the server, which enforces access in code ([organization skill](../../organization/SKILL.md)). Do not give the database users that mirror Antilog's members. A policy on each organization table is an extra guard; the code never relies on it. The server connects as a role that owns no table (the migrations run as the owner) and sets `antilog.organization_id` in each transaction through `PgStore::scoped`.

## sqlx

- **Compile-time-checked queries.** Use `query!`, `query_as!` and `query_scalar!` so a wrong column or type fails the build. Commit the offline query data (`.sqlx/`) with every change to a query, and build CI with `SQLX_OFFLINE=true` so the build needs no database. `cargo sqlx prepare` regenerates it against a migrated database. Use the runtime `query()` only where a query is built dynamically, and then keep it small and tested.
- **Embedded migrations** run at start-up with `sqlx::migrate!`, before the server reports ready. In a multi-replica deployment only one replica applies them at a time (sqlx takes an advisory lock for this).
- **One pool**, configured at start (`max_connections`, `acquire_timeout`, `idle_timeout`, `max_lifetime`); see [general/async-services.md](../rust/general/async-services.md) and the connection rules in the general digest. Never hold a connection across a call to another service.
- **Transactions are explicit and short.** `begin()`, do the writes, `commit()`. Do not call out to the network, a session machine or a model inside one. A transaction left uncommitted is rolled back on drop, but end it explicitly.
- **Map the errors.** Unique violation is a conflict (409), foreign key violation is a bad reference (422), serialization failure and deadlock mean retry the whole transaction (bounded, with backoff), a pool timeout is overload (503), the rest is internal (500, with the cause in the log).
- **Dynamic SQL is built with `QueryBuilder` and binds**, never string formatting of values. Identifiers are never taken from user input.
- **Keep SQL in the persistence code of its module**, behind a trait that the rest of the code uses. Tests of other modules use the in-memory double, not the database.

## Migrations: forward only

- Numbered files in `migrations/`, one change each. **Never edit a shipped migration** and never write a down migration: to undo, write a new forward one. A shipped migration is one that has been applied anywhere outside your machine.
- **Expand, then contract.** Add the new column or table, deploy code that writes both and reads the new, backfill, then drop the old in a later migration, never in the same release. Two server versions can serve at once during a rollout, so a migration must not break the previous server version that is still serving.
- **Be safe on a live table.** Set `lock_timeout` so a migration fails fast instead of queueing everyone behind it. Add indexes with `create index concurrently`, which cannot run in a transaction: put the statement alone in its file with the `-- no-transaction` directive at the top. Add constraints `not valid`, then `validate constraint` in a later step. Backfill big tables in batches, in a separate migration or job from the schema change. Details: [general/postgres-best-practices.md](general/postgres-best-practices.md).
- **Test the migration.** A test applies all migrations to an empty database and checks that the schema matches what the queries expect; the offline query check also catches drift. A migration that rewrites data has a test with sample rows.
- Foreign keys are on, and every foreign key column has an index. Use plain `snake_case` lowercase names, unquoted.

## One schema

- Ids are `uuid` (UUIDv7 made in Rust). Times are `timestamptz`. A document the server stores whole is `jsonb`. States are `text` with a `check`. Money is `bigint` minor units with a currency column.
- Append-only tables (audit log, events, verdicts, receipts, spend) are enforced by triggers.
- Moving an organization between deployments (Antilog cloud to the client's own) is an export and import of one neutral file format, not a database copy, so the schema can change without breaking a move.

## Types and ids

- Ids are `uuid` holding a UUIDv7, made in Rust (`uuid` with the `v7` feature) so a test can fix them, and time-ordered so inserts are not scattered across the index. The owner confirms the id scheme when the first schema lands (target). Never expose a sequential id that leaks counts to a customer.
- Times are `timestamptz`, written in UTC. Money and spend are integer minor units with a currency column, never a float. Counts and versions that must add up are integers.
- States are `text` with a `check` constraint, never a Postgres enum, so adding a state is a plain migration. Mirror them as a Rust enum with an exhaustive `match`; a test lists the values in both and compares them.
- JSON is `jsonb`, only for a document the server stores whole and does not query into (a receipt payload, a spec). Anything you filter or join on is a column. Do not store a secret in it.
- Constrain in the database what the database can know: `not null`, `check`, `unique`, foreign keys with a deliberate `on delete`.

## The job queue

The queue is a table in Postgres. No Redis, no message bus, no cloud queue ([libraries.md](../libraries.md)), so it runs the same on every cloud.

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
- **Bounded and observable.** Cap attempts, record the last error (without secrets), move a job that keeps failing to a `failed` state with a reason, and expose queue depth and oldest-job age as metrics. Keep a partial index for the hot path: `create index on jobs (priority desc, created_at) where state = 'queued'`.
- **Keep it small.** Autovacuum settings on the queue table are tightened because rows churn ([general/postgres-best-practices.md](general/postgres-best-practices.md)); finished jobs older than a retention period are moved or deleted in batches. `LISTEN/NOTIFY` may wake a worker sooner, but polling must still work if a notification is missed.

## Tests

- Run database tests against a real Postgres 18 in a container (`testcontainers`), one database per test or per module with a clean schema, not a mock. There is no in-memory stand-in for Postgres.
- Test the claim query with concurrent workers (no job taken twice, none skipped forever), lease expiry, and idempotent re-delivery.
- Test that every organization-scoped query cannot see another organization's rows.

## Pitfalls

- **A migration that locks a hot table** blocks the whole server. Check what lock a statement takes before shipping it, and set `lock_timeout`.
- **`OFFSET` pagination** gets slower with depth and skips or repeats rows while new ones arrive; the timeline, the audit log and trial lists page by a keyset cursor on `(time, id)` ([general/postgres-best-practices.md](general/postgres-best-practices.md)).
- **A long-lived transaction** holds locks and stops vacuum. Set `idle_in_transaction_session_timeout`.
- **Unindexed foreign keys** make deletes and joins scan. Run the missing-index query in review.
- **`timestamp` without time zone, `float` for money, `varchar(n)` for nothing** are the usual type mistakes: see the types above.
- **Enum drift.** Adding a state in Rust without the migration (or the reverse) is caught by the test that compares the lists; keep that test.

## General guidance

[`general/postgres-best-practices.md`](general/postgres-best-practices.md): query performance, indexes, connections and pooling, schema design, locking and concurrency, data access, diagnostics and migrations. It is a digest of [Supabase's postgres-best-practices](https://github.com/supabase/agent-skills) (MIT, [`licenses/supabase-agent-skills-LICENSE`](licenses/supabase-agent-skills-LICENSE)).

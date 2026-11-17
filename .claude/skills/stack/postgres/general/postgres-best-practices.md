# Postgres best practices (digest)

Source: [supabase-postgres-best-practices](https://github.com/supabase/agent-skills/tree/main/skills/supabase-postgres-best-practices) in supabase/agent-skills, by Supabase (version 1.1.1 in the skill header, MIT, [`../licenses/supabase-agent-skills-LICENSE`](../licenses/supabase-agent-skills-LICENSE)). The original is one short rule per file, each with an incorrect and a correct SQL example, in eight categories. This digest condenses the plain-Postgres rules and keeps the order of impact. [`../antilog.md`](../antilog.md) wins where they differ.

Dropped as Supabase-specific or not applicable to Antilog: row-level security and end-user privilege rules (`security-rls-basics`, `security-rls-performance`, `security-privileges`: Antilog's server is the only database client and enforces access in code, and stores no customer data), the Supabase pooler ports and dashboard links, full-text search and JSONB indexing (`advanced-*`; add them only when a query needs them), and the `auth.*` helpers.

## 1. Query performance (critical)

- **Index the columns you filter and join on.** A query that filters or joins on an unindexed column scans the table, and gets slower as it grows. `EXPLAIN` showing `Seq Scan` on a large table with a selective filter is the sign.
- **Composite indexes: equality columns first, range columns last** (`(status, created_at)` for `status = ... AND created_at > ...`). The index serves the leftmost prefix only: it does not help a query on `created_at` alone. One composite index beats two single-column indexes that must be combined.
- **Covering indexes** (`create index ... (email) include (name, created_at)`) let a query be answered from the index (index-only scan) when it selects columns it does not filter on. Use `INCLUDE`, not extra key columns.
- **Partial indexes** (`create index ... (created_at) where status = 'pending'`) are smaller and faster when a query always applies the same filter, such as live rows of a table whose finished rows pile up. The query's `WHERE` must imply the index predicate.
- **Pick the index type for the operator.** B-tree (default) for `=`, ranges, `IN`, `IS NULL`, ordering; GIN for arrays, JSONB containment (`@>`) and full-text; GiST for ranges and nearest-neighbour; BRIN for very large append-only tables ordered by time; hash is rarely worth it.

## 2. Connection management (critical)

- **Pool connections.** A Postgres connection costs memory (the source says 1 to 3 MB) and a process. Do not open one per request. `sqlx`'s `PgPool` is the app-side pool; set `max_connections` for the pool from the server's limit divided by the number of server replicas, not from the number of requests. Add a server-side pooler such as PgBouncer only when many processes share one database.
- **Size the limit from resources**, not by raising `max_connections` blindly. `work_mem` times connections should stay well under the machine's memory (the source suggests under 25 percent). A small pool (roughly cores times two, plus spindles) usually beats a large one.
- **Time out idle sessions.** Set `idle_in_transaction_session_timeout` (the source suggests 30 s) so a forgotten transaction does not hold locks and block vacuum, and `idle_session_timeout` for idle connections.
- **Prepared statements and transaction-mode pooling do not mix** unless the pooler supports protocol-level prepared statements. A named prepared statement lives on one server connection, and the next transaction may land on another. If a transaction-mode pooler sits in front, use unnamed statements or disable the statement cache (for `sqlx`, `statement_cache_capacity(0)`), or use session mode.

## 3. Schema design (high)

- **Types.** `bigint generated always as identity` for ids (not `int`, not `serial`); `text`, not `varchar(n)`, unless the limit is a rule; `timestamptz`, never `timestamp`; `boolean`, not a string; `numeric` for exact decimals, never `float` for money or counts that must add up; `text` with a `check` constraint, or an enum type, for a closed set of states.
- **Primary keys.** One database, ordinary ids: `bigint` identity. Ids that leave the system or are made on several machines: a time-ordered UUID. Random v4 UUIDs as keys on a large table scatter inserts across the index and fragment it. Postgres 18 has a built-in `uuidv7()`, so the extension the source names is not needed.
- **Index every foreign key column.** Postgres does not do it for you. Without it, joins on the key and `ON DELETE CASCADE` scan the child table. The source gives a catalogue query that lists foreign keys with no index; run it in review.
- **Lowercase snake_case identifiers, unquoted.** Quoted mixed-case names must be quoted forever and break tools and generated code.
- **Add constraints safely.** `ADD CONSTRAINT IF NOT EXISTS` does not exist. Guard with a `DO` block that checks `pg_constraint`, or, better, rely on forward-only migrations that run once.
- **Partition** only very large (the source says over 100 million rows), time-ordered tables, mainly so old data is dropped by dropping a partition instead of a slow `DELETE`. Not before it hurts.

## 4. Concurrency and locking (medium-high)

- **Keep transactions short.** A transaction holds its row locks until it ends. Never call out to the network or another service inside one: do the call first, then run the short update (`update ... where id = $1 and status = 'pending' returning *`). Use `statement_timeout` (and `set local` per transaction) so a runaway statement cannot hold locks.
- **Lock in a consistent order** to avoid deadlocks: take row locks with `select ... order by id for update` before updating several rows, or update them in one statement. Turn on `log_lock_waits` and watch `pg_stat_database.deadlocks`.
- **Queues: `FOR UPDATE SKIP LOCKED`.** Workers that all run `select ... for update` on the next pending row serialise behind one another. With `skip locked` each worker takes a different row without waiting. Claim and mark in one statement:

  ```sql
  update jobs
     set status = 'running', worker_id = $1, started_at = now()
   where id = (
     select id from jobs
      where status = 'queued'
      order by created_at
      limit 1
      for update skip locked)
  returning *;
  ```

- **Advisory locks** (`pg_advisory_xact_lock`, `pg_try_advisory_lock`) coordinate application-level exclusivity (one leader, one scheduler) without a dummy row to lock. Prefer the transaction-scoped form: it is released on commit or rollback, so a crash cannot leak it.

## 5. Data access patterns (medium)

- **No N+1.** One query per item in a loop is a round trip each. Fetch with `where id = any($1::bigint[])` or a join.
- **Batch writes.** One multi-row `insert ... values (...), (...)` (about a thousand rows a batch) or `COPY` for a bulk load, not one insert per row. With `sqlx`, `UNNEST` over arrays (`insert ... select * from unnest($1::bigint[], $2::text[])`) is the compile-time-checkable way.
- **Keyset pagination, not `OFFSET`.** `OFFSET n` reads and discards n rows, so deep pages are slow. Use `where (created_at, id) > ($1, $2) order by created_at, id limit 20`, with the cursor holding every sort column, ending in a unique one.
- **Upsert atomically.** `insert ... on conflict (...) do update set ... returning *` (or `do nothing`) instead of select-then-insert, which races.

## 6. Monitoring and diagnostics (low-medium)

- **`EXPLAIN (ANALYZE, BUFFERS)` before you guess.** Look for `Seq Scan` on a big table, a large `Rows Removed by Filter`, reads much larger than hits (cold data), a nested loop with a high loop count, and `Sort Method: external merge` (`work_mem` too low). `ANALYZE` runs the statement: wrap a write in a transaction you roll back.
- **`pg_stat_statements`** shows which statements cost the most in total and on average. Sort by `total_exec_time` for the biggest bill and by `mean_exec_time` for the slowest; reset after a fix to measure it.
- **Keep statistics fresh.** Autovacuum does it; run `analyze <table>` after a bulk change, and lower `autovacuum_vacuum_scale_factor` on high-churn tables such as a job queue so dead rows are reclaimed quickly. Check `pg_stat_user_tables` for `last_autovacuum`.

## Migrations: what the source covers, and what Antilog adds

The source has one migration rule (guarded constraint creation, above). The points below are not from it: they are standard Postgres practice, kept here so the migration advice is in one place. Check them against the Postgres documentation for the version in [`../../libraries.md`](../../libraries.md).

- A DDL statement takes a lock; some take `ACCESS EXCLUSIVE` and block every read and write on the table until they finish, and wait behind long transactions while queueing everyone else. Set `lock_timeout` in the migration so it fails fast instead of stalling the application.
- `create index concurrently` builds an index without blocking writes, but cannot run inside a transaction. `sqlx` wraps each migration in a transaction unless the file starts with the `-- no-transaction` directive.
- Add a constraint without a long lock in two steps: `add constraint ... check (...) not valid` (or `foreign key ... not valid`), then `validate constraint` later, which takes a weaker lock.
- Adding a column with a constant default is fast on current Postgres. Rewriting a column's type usually rewrites the table. Backfill large tables in batches, in a separate step from the schema change.
- Expand, then contract: add the new column or table, deploy code that writes both and reads the new one, then drop the old in a later migration. Never drop and replace in one release.

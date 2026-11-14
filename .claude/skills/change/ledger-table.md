# A table, column or migration

The ledger is Postgres 18 and the only state. It holds ids, versions, numbers, states and audit rows. It never holds a customer's data, a secret, a diff or a log. (target: nothing here is built yet.)

## Rules (target)

- Access is through sqlx. Queries are checked against the schema (offline query data is committed, see [stack/postgres/ratchet.md](../stack/postgres/ratchet.md)).
- Migrations are forward only, numbered files in `crates/ratchet-server/migrations/`. Add a table or column as a new file. Never edit one that has shipped. There are no down migrations: a mistake is fixed by the next migration.
- Every row carries its organization. A query without it can cross customers.
- Each of the three modules (optimization, context, infrastructure) owns its own tables. The floor owns members, roles, tokens and the audit log.
- No joins across modules. A module reads another through its API, by id and version.
- Spend rows, the audit log and the receipts are append-only, enforced by the database and not by the code that writes them.
- Jobs for the runner are rows in Postgres; see [runner-job.md](runner-job.md).

## Steps (target)

1. Write the migration. Name the owning module in the file's header.
2. Add or change the Rust row type and the functions that read and write it. Keep the module boundary.
3. If the column is visible, do [api-endpoint.md](api-endpoint.md).
4. Tests with a real Postgres in a container (testcontainers): the migration applies to an empty database and to the last release's schema; every table has an organization column; append-only tables refuse an update; a role without the permission cannot read the row through the API.
5. Check the SQL by [stack/postgres/ratchet.md](../stack/postgres/ratchet.md).

## Row-level security

- Every table with an organization column has a row-level security policy on it, as a second guard behind the query filter. The server sets the organization for each transaction; a query that forgets the filter returns nothing instead of another organization's rows.

The earlier SQLite ledger is in [today.md](today.md).

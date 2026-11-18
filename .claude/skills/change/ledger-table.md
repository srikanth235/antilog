# A table, column or migration

The ledger is the store, and the only state: Postgres 18, on Antilog cloud and self-hosted alike. The core sees one `Store` trait, so tests of other modules use an in-memory double. It holds ids, versions, numbers, states, the conversation's words (decided by the owner, see [product rulings](../product/rulings.md)), commit ids, spend and audit rows. It never holds a customer's data, a secret, tool output, a diff or a log. (target: nothing here is built yet.)

## Rules (target)

- Access is behind the `Store` trait; the core never writes SQL. The Postgres store uses sqlx, with queries checked against the schema (offline query data is committed, see [stack/postgres/antilog.md](../stack/postgres/antilog.md)).
- The shared schema uses text ids, integer millisecond timestamps and JSON as text, with no Postgres-only type.
- Migrations are forward only, numbered files in one directory. Add a table or column as a new file. Never edit one that has shipped. There are no down migrations: a mistake is fixed by the next migration.
- Every row carries its organization. A query without it can cross customers. On Antilog cloud each client also has its own database; the column stays so an export moves between deployments.
- Each area owns its own tables: the session (experiments' sessions, messages, cards, environments, platform accounts), the judge (experiments and their versions, seals, trials, measurements, receipts, the judge's queue) and the organization floor (members, projects, project members and their roles, tokens, approvals, policy, budgets (one per experiment, with an optional cap per project), the audit log). Customize items (evaluation templates, skill packs and their versions, connectors, agents, plugins) belong to the area that governs them: evaluation templates to the judge, skill packs, connectors and agents to the session. An experiment row carries its project id.
- No joins across areas. An area reads another through its API, by id and version.
- Spend rows, measurements, the audit log and the receipts are append-only, enforced by triggers and not by the code that writes them.
- Platform jobs are rows in the store; see [platform-job.md](platform-job.md).

## Steps (target)

1. Write the migration. Name the owning area (session, judge or organization) in the file's header.
2. Add or change the Rust row type and the functions that read and write it. Keep the area boundary.
3. If the column is visible, do [api-endpoint.md](api-endpoint.md).
4. Tests: the storage suite runs against Postgres in a container (testcontainers). The migration applies to an empty database and to the last release's schema; every table has an organization column; append-only tables refuse an update; a role without the permission cannot read the row through the API.
5. Check the SQL by [stack/postgres/antilog.md](../stack/postgres/antilog.md).

## Row-level security

- Every table with an organization column has a row-level security policy on it, as a second guard behind the query filter. The server sets the organization for each transaction; a query that forgets the filter returns nothing instead of another organization's rows.


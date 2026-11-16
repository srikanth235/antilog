# A table, column or migration

The ledger is the store, and the only state: Postgres 18 self-hosted, SQLite in the organization's Durable Object in the cloud. The core sees one `Store` trait and both are supported from day 0. It holds ids, versions, numbers, states, the conversation's words (decided by the owner, see [product rulings](../product/rulings.md)), commit ids, spend and audit rows. It never holds a customer's data, a secret, tool output, a diff or a log. (target: nothing here is built yet.)

## Rules (target)

- Access is behind the `Store` trait; the core never writes SQL. The Postgres store uses sqlx, with queries checked against the schema (offline query data is committed, see [stack/postgres/ratchet.md](../stack/postgres/ratchet.md)).
- The shared schema uses text ids, integer millisecond timestamps and JSON as text, with no Postgres-only type.
- Migrations are forward only, numbered files, one directory per dialect (Postgres and SQLite), written twice with the same numbers. Add a table or column as a new file in each. Never edit one that has shipped. There are no down migrations: a mistake is fixed by the next migration.
- Every row carries its organization. A query without it can cross customers. In the cloud each organization has its own database, so a cross-customer read is not possible there; the column stays so a file moves between cloud and self-hosted.
- Each area owns its own tables: the session (experiments' sessions, messages, cards, environments, platform accounts), the judge (experiments and their versions, seals, trials, measurements, receipts, the judge's queue) and the organization floor (members, projects, project members and their roles, tokens, approvals, policy, budgets (one per experiment, with an optional cap per project), the audit log). Customize items (evaluation templates, skill packs and their versions, connectors, agents, plugins) belong to the area that governs them: evaluation templates to the judge, skill packs, connectors and agents to the session. An experiment row carries its project id.
- No joins across areas. An area reads another through its API, by id and version.
- Spend rows, measurements, the audit log and the receipts are append-only, enforced by triggers in both dialects and not by the code that writes them.
- Platform jobs are rows in the store; see [platform-job.md](platform-job.md).

## Steps (target)

1. Write the migration in both dialects. Name the owning area (session, judge or organization) in the file's header.
2. Add or change the Rust row type and the functions that read and write it. Keep the area boundary.
3. If the column is visible, do [api-endpoint.md](api-endpoint.md).
4. Tests: the one storage suite runs against both stores, Postgres in a container (testcontainers) and SQLite. The migration applies to an empty database and to the last release's schema in each dialect; both dialects end with the same tables and columns; every table has an organization column; append-only tables refuse an update; a role without the permission cannot read the row through the API.
5. Check the SQL by [stack/postgres/ratchet.md](../stack/postgres/ratchet.md).

## Row-level security (Postgres only)

- Every table with an organization column has a row-level security policy on it, as a second guard behind the query filter. The server sets the organization for each transaction; a query that forgets the filter returns nothing instead of another organization's rows. The policy is a Postgres-only migration. SQLite has none: in the cloud the organization's own database is the boundary.

The earlier SQLite ledger is in [today.md](today.md).

# A table, column or migration

The ledger is the store, and the only state: Postgres 18, on Antilog cloud and self-hosted alike. The core sees one `Store` trait, so tests of other modules use an in-memory double. It holds ids, versions, numbers, states, the conversation's words (decided by the owner, see [product rulings](../product/rulings.md)), commit ids, spend and audit rows. It never holds a customer's data, a secret, tool output, a diff or a log. Built: the organization and session side (organizations, members, projects and project roles, access tokens, sign-in sessions, experiments' rows, sessions, spend, policy, approvals, audit log) and row-level security on the organization's own tables (not yet on `experiments`, `sessions` and `spend`). Not built: the judge's tables (target).

## Rules (target)

- Access is behind the `Store` trait, one supertrait per area in `crates/antilog-core/src/store/` (a fake per area in `fakes/`, the Postgres impl per area in `crates/antilog-server/src/pg/`); the core never writes SQL. A write takes its audit entry and stores both in one transaction. The Postgres store uses sqlx, with queries checked against the schema (offline query data is committed, see [stack/postgres/antilog.md](../stack/postgres/antilog.md)).
- The shared schema uses text ids, integer millisecond timestamps and JSON as text, with no Postgres-only type.
- Migrations are forward only, numbered files in one directory. Add a table or column as a new file. Never edit one that has shipped. There are no down migrations: a mistake is fixed by the next migration.
- Every row carries its organization. A query without it can cross customers. On Antilog cloud each client also has its own database; the column stays so an export moves between deployments.
- Each area owns its own tables: the session (experiments' sessions, messages, cards, environments, platform accounts), the judge (experiments and their versions, seals, trials, measurements, receipts, the judge's queue) and the organization floor (members, projects, project members and their roles, tokens, approvals, policy, budgets (one per experiment, with an optional cap per project), the audit log). Customize items (evaluation templates, skill packs and their versions, connectors, agents, plugins) belong to the area that governs them: evaluation templates to the judge, skill packs, connectors and agents to the session. An experiment row carries its project id.
- No joins across areas. An area reads another through its API, by id and version.
- Spend rows, measurements, the audit log and the receipts are append-only, enforced by triggers and not by the code that writes them. The same goes for invariants the database can know: the last project owner, nested caps, final session states, a decided approval (errcodes `AL001` to `AL005`, mapped to `Invariant` in the store).
- Platform jobs are rows in the store; see [platform-job.md](platform-job.md).

## Steps (target)

1. Write the migration. Name the owning area (session, judge or organization) in the file's header.
2. Add or change the Rust row type and the functions that read and write it. Keep the area boundary.
3. If the column is visible, do [api-endpoint.md](api-endpoint.md).
4. Tests: the storage suite runs against Postgres in a container (testcontainers). The migration applies to an empty database and to the last release's schema; every table has an organization column; append-only tables refuse an update; a role without the permission cannot read the row through the API.
5. Check the SQL by [stack/postgres/antilog.md](../stack/postgres/antilog.md).

## Row-level security

- Every table with an organization column has a row-level security policy on it, as a second guard behind the query filter. The server sets the organization for each transaction; a query that forgets the filter returns nothing instead of another organization's rows. Built: `PgStore::scoped` sets `antilog.organization_id` for the transaction, the server connects as a role that owns no table and is no superuser, and the migration guards that look across rows are `security definer`. An audit row may be appended with no organization set; reading the log needs one. The tests run the storage suite as such a role.


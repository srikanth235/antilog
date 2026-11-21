# A table, column or migration

The ledger is the store, and the only state: Postgres 18, on Antilog cloud and self-hosted alike. The core sees one `Store` trait, so tests of other modules use an in-memory double. It holds ids, versions, numbers, states, the conversation's words (decided by the owner, see [product rulings](../product/rulings.md)), commit ids, spend and audit rows. It never holds a customer's data, a secret, a log or the whole of a tool's output (its first 4 KB is kept). The one diff it keeps is the Changes panel's: the runner's latest report per session and repo (`session_changes`, `session_commit_changes`, cut to 2 MB per diff, replaced on each report). Built: organizations, members, projects and project roles, access tokens, sign-in sessions, sessions (each with its project, model, repos, environment version and optional cap), session events, spend, policy, approvals, environments, model keys, connectors, skills, plugins and the audit log, all behind row-level security. The judge's tables, experiments and the per-experiment selections are dropped by migration `20261121000000_cloud_sessions_only.sql`; `daa445f` has them.

## Rules

- Access is behind the `Store` trait, one supertrait per area in `crates/antilog-core/src/store/` (a fake per area in `fakes/`, the Postgres impl per area in `crates/antilog-server/src/pg/`); the core never writes SQL. A write takes its audit entry and stores both in one transaction. The Postgres store uses sqlx, with queries checked against the schema (offline query data is committed, see [stack/postgres/antilog.md](../stack/postgres/antilog.md)).
- The shared schema uses text ids, integer millisecond timestamps and JSON as text, with no Postgres-only type.
- Migrations are forward only, numbered files in one directory. Add a table or column as a new file. Never edit one that has shipped. There are no down migrations: a mistake is fixed by the next migration.
- Every row carries its organization. A query without it can cross customers. On Antilog cloud each client also has its own database; the column stays so an export moves between deployments.
- Each area owns its own tables: the session (sessions, events, cards, environments, the queue) and the organization floor (members, projects, project members and their roles, tokens, approvals, policy, limits and caps, the audit log). Customize items (skill packs and their versions, connectors, agents, plugins) belong to the session. A session row carries its project id.
- No joins across areas. An area reads another through its API, by id and version.
- Spend rows and the audit log are append-only, enforced by triggers and not by the code that writes them. The same goes for invariants the database can know: the last project owner, nested caps, final session states, a decided approval (errcodes `AL001` to `AL005`, mapped to `Invariant` in the store).

## Steps

1. Write the migration. Name the owning area (session or organization) in the file's header.
2. Add or change the Rust row type and the functions that read and write it. Keep the area boundary.
3. If the column is visible, do [api-endpoint.md](api-endpoint.md).
4. Tests: the storage suite runs against Postgres in a container (testcontainers). The migration applies to an empty database and to the last release's schema; every table has an organization column; append-only tables refuse an update; a role without the permission cannot read the row through the API.
5. Check the SQL by [stack/postgres/antilog.md](../stack/postgres/antilog.md).

## Row-level security

- Every table with an organization column has a row-level security policy on it, as a second guard behind the query filter. The server sets the organization for each transaction; a query that forgets the filter returns nothing instead of another organization's rows. Built: `PgStore::scoped` sets `antilog.organization_id` for the transaction, the server connects as a role that owns no table and is no superuser, and the migration guards that look across rows are `security definer`. An audit row may be appended with no organization set; reading the log needs one. The tests run the storage suite as such a role.


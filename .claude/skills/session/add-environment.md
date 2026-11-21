# Add or change an environment

An environment is what a session runs on: default repos, a network level, network secrets, plain variables, a setup script and a size (see [docs/V0.md](../../../docs/V0.md)). An organization manages its own, and every organization has the built-in `default` (limited network with package registries, 2 vCPU, 8 GiB), which is not stored and cannot be changed.

Built: `antilog-types` (`Environment`, `NewEnvironment`, and `EnvironmentSpec` with `NetworkLevel` in `runner.rs`), `antilog-core/src/environments.rs` (validation and the actions), `store/environment.rs` with its fake and Postgres store, the migration `20261120700000_environments.sql`, the routes in `api/session.rs`, Settings, Connections in the web app, and the proxy's allowlist.

## Rules

- **Versions.** Creating makes version 1; an edit is the next version, never an update in place. The versions are append-only (a trigger refuses a change), because a session records the version it ran. An environment's id comes from its name once and is kept. An edit made from an old version loses with a conflict.
- **Archive.** An archived environment is no longer offered and takes no new version; sessions already on it keep running, and its versions stay readable.
- **Who.** Everyone reads. Owners and editors create, edit and archive (`environment.create`, `environment.edit`, `environment.archive` in the permission matrix); a denial is audited.
- **Network.** `none` reaches only the platform hosts (the model gateway, MCP and the code host, always allowed); `limited` reaches exact host names (no wildcard, port or address) and the package registries when switched on; `full` reaches any host except a private, loopback or metadata address. The proxy enforces it from the session's own version ([../../../docs/V0-contracts.md](../../../docs/V0-contracts.md)).
- **Default repos.** `owner/repo`, at most 50. A new session that names the environment starts with them, then the ones the person picked (`environments::pin`, merged in `session::start`).
- **Setup script.** Runs as root in each fresh container after the clones and before the agent, from the folder that holds the checkouts, under a 10 minute limit; a failure or timeout does not fail the session ([runner.md](runner.md)). At most 64 KB.
- **Network secrets.** Host (exact name), header, optional prefix, and a value that is write-only: sealed on the way in (`environment_secrets`), never returned by the API, never sent to a runner. An edit that leaves the value out keeps the stored one; a new secret without a value is refused. The proxy adding them to requests is (target); until then the screen says they are kept and not applied.
- **Variables are plain.** The agent can read them, and the screen says so. A model key never goes there: model keys stay sealed with the organization and are never sent into a session. Names `ANTILOG_*` and the proxy variables are refused. The setup script is at most 64 KB.

## Steps for a change

1. Types and validation: `antilog-core/src/environments.rs`; run `cargo run -q -p antilog-server -- openapi > openapi/antilog.json` and `pnpm generate:api` in `web/`.
2. A new column: [../change/ledger-table.md](../change/ledger-table.md); `cargo sqlx prepare --workspace`.
3. The screen: a board first ([../change/screen.md](../change/screen.md)); `SettingsConnections` is the list, `SettingsEnvironment` and `SettingsEnvironmentSecrets` the edit dialog.
4. A change an agent proposes arrives as a Change environment card; a person approves it and the session moves to the environment's next version ([tools.md](tools.md)). `environments::changed` applies a change to a version with the same checks as an edit, and `save_for_session` writes the version; a new field an environment holds must be carried there, in `CardPayload::ChangeEnvironment`'s views and in the runner's `ApplyEnvironment` if it can apply live.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md). The store suite (`antilog-core/src/testing.rs`, run on both stores), the API test for roles, the proxy tests for each level and `web/e2e/environments.spec.ts` cover it.

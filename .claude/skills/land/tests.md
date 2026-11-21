# Tests

How tests are written here.

## Approach (target)

- **Schema tests on the OpenAPI document.** Every route in the document has a case; each answer parses into its Rust type; a meta-test fails when a route has no case. The committed document and the generated TypeScript client must be current.
- **A fake runner, not a simulation mode.** The runner's tests give it a plain-process `Sandbox` and a fake OpenCode server; the container backend is tested through a recording `Cli`. There is no `--mode sim`, and no product feature exists only for demos.
- **One storage suite.** The suite is written against the `Store` trait and runs against a real Postgres through testcontainers, never a stand-in. Migrations apply to an empty database and to the last release's schema.
- **Snapshots with insta.** For the document, the event union and error bodies. A snapshot changes only when behaviour was meant to, and the diff is reviewed.
- **Properties.** `proptest` for the budget arithmetic, the runner's queue transitions and the export and import round trip; see [stack/security/antilog.md](../stack/security/antilog.md).
- **The action layer.** Allowed role writes and audits once; denied role writes nothing and audits the denial; the event follows the commit. A route-walk test calls every route as each role (owner, editor, viewer) and compares with the matrix.
- **Secrets never leave.** A test that sets a key, a token and a connector secret, then checks no response, event, error, log, or audit entry carries it.
- **The server never connects to data.** A test that the server process opens no connection to a connector or to any host its configuration does not name. Self-hosted, only configured endpoints are contacted: the sign-in provider and the code host.
- **Budgets.** The gateway refuses a call at each cap (session, project, organization); a cap cannot pass the one above it (409 `over_cap`).
- **Prompts.** What agents are told (the tool descriptions and the endpoint's instructions) is covered by the tool tests; see [session/prompts.md](../session/prompts.md).
- **Clients.** Web tests run against the generated client, in `@playwright/test`; to look at a screen yourself while you work, use `playwright-cli` ([stack/playwright/antilog.md](../stack/playwright/antilog.md)). Swift tests cover state and words shared with the web app (target).
- **No test asserts on a wall-clock duration.** Assert on counts.
- Never skip, ignore or delete a test to get green.

# Tests

None of this exists yet; it is how tests are written once code lands (target). Earlier practice is in [today.md](today.md).

## Approach (target)

- **Contract tests on the OpenAPI document.** Every route in the document has a case; each answer parses into its Rust type; a meta-test fails when a route has no case. The committed document and the generated TypeScript client must be current.
- **A fake platform, not a simulation mode.** The server is tested against a fake agent platform that starts sessions, emits events (including `measure` calls), runs evaluator sandboxes and records tags, caps and clean-up. There is no `--mode sim`, and no product feature exists only for demos. See [change/platform-job.md](../change/platform-job.md).
- **One suite for every platform.** The platform contract suite runs against every adapter, against the fake and against each real platform with the owner's go-ahead (see [infra/add-platform.md](../infra/add-platform.md)). A tiny end-to-end spike on all three platforms comes first.
- **One storage suite for both stores.** The suite is written against the `Store` trait and runs against a real Postgres through testcontainers, never a stand-in, and against SQLite. Migrations apply to an empty database and to the last release's schema in each dialect, and both dialects end with the same tables and columns.
- **The core builds for wasm.** `ratchet-core` also builds for `wasm32-unknown-unknown`; see [ci.md](ci.md).
- **Snapshots with insta.** For the document, the event union, receipts and error bodies. A snapshot changes only when behaviour was meant to, and the diff is reviewed.
- **Properties.** `proptest` for the accept rule, the noise floor and the budget arithmetic, and for run state transitions and the export and import round trip; see [stack/security/ratchet.md](../stack/security/ratchet.md).
- **The action layer.** Allowed role writes and audits once; denied role writes nothing and audits the denial; the event follows the commit. A route-walk test calls every route as each role and compares with the matrix.
- **Secrets never leave.** A test that sets a key, a token and a connector secret, then checks no response, event, error, log, audit entry or receipt carries it.
- **The server never connects to data.** A test that the server process opens no connection to a connector or to any host its configuration does not name. Self-hosted, only configured endpoints are contacted: the sign-in provider, the code host and the agent platforms.
- **Receipts.** A receipt read back has every field, and one whose numbers were changed after signing fails verification.
- **Clients.** The CLI contract script runs against a built binary and a test server. Web tests run against the generated client, in `@playwright/test`; to look at a screen yourself while you work, use `playwright-cli` ([stack/playwright/ratchet.md](../stack/playwright/ratchet.md)). Swift tests cover state and words shared with the web app.
- **No test asserts on a wall-clock duration.** Assert on counts.
- Never skip, ignore or delete a test to get green.

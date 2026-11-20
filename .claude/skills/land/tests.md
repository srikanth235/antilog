# Tests

None of this exists yet; it is how tests are written once code lands (target).

## Approach (target)

- **Schema tests on the OpenAPI document.** Every route in the document has a case; each answer parses into its Rust type; a meta-test fails when a route has no case. The committed document and the generated TypeScript client must be current.
- **A fake machine, not a simulation mode.** The judge runs the sealed command through the `SessionMachine` trait; tests give it `FakeMachine`, scripted by commit. There is no `--mode sim`, and no product feature exists only for demos.
- **One storage suite.** The suite is written against the `Store` trait and runs against a real Postgres through testcontainers, never a stand-in. Migrations apply to an empty database and to the last release's schema.
- **Snapshots with insta.** For the document, the event union, receipts and error bodies. A snapshot changes only when behaviour was meant to, and the diff is reviewed.
- **Properties.** `proptest` for the acceptance criteria (one pure function, see [judge/change-acceptance.md](../judge/change-acceptance.md)), the noise floor, the bar that rises with the number of trials tried, the budget arithmetic, the judge's queue transitions and the export and import round trip; see [stack/security/antilog.md](../stack/security/antilog.md).
- **The action layer.** Allowed role writes and audits once; denied role writes nothing and audits the denial; the event follows the commit. A route-walk test calls every route as each role (owner, editor, viewer) and compares with the matrix.
- **Secrets never leave.** A test that sets a key, a token and a connector secret, then checks no response, event, error, log, audit entry or receipt carries it.
- **The server never connects to data.** A test that the server process opens no connection to a connector or to any host its configuration does not name. Self-hosted, only configured endpoints are contacted: the sign-in provider and the code host.
- **The judge.** The seal checks fail an experiment whose evaluation passes a planted cheat; a trial that changes a protected path is Rejected, with the protected files taken from the seal; no agent session holds the judge's credential; a Kept trial is measured again in a fresh checkout and on the holdout; no response, event or card carries a holdout value. See [judge/anti-cheat.md](../judge/anti-cheat.md).
- **Prompts.** What agents are told is versioned and tested against a set of repos with known wins (target); see [session/prompts.md](../session/prompts.md).
- **Receipts.** A receipt read back has every field, and one whose numbers were changed after signing fails verification.
- **Clients.** The CLI test script runs against a built binary and a test server. Web tests run against the generated client, in `@playwright/test`; to look at a screen yourself while you work, use `playwright-cli` ([stack/playwright/antilog.md](../stack/playwright/antilog.md)). Swift tests cover state and words shared with the web app.
- **No test asserts on a wall-clock duration.** Assert on counts.
- Never skip, ignore or delete a test to get green.

# What organization owes

| Promise | The rule |
| --- | --- |
| Permission | Every read and write is checked against the organization and project role, in one place. |
| Audit | Every write is a named action with an actor, recorded in the audit log. Changing agent platform accounts is among them. |
| Secrets | No response, event or log carries a secret. |
| No surprise calls | A self-hosted server contacts only the endpoints its administrator configured. A session contacts only the endpoints its experiment configures. |

## How each is checked

Each check is (target) unless it says "today".

**Permission.** Check: one test per route that the role matrix is applied, driven from the route table in the OpenAPI document, so a new route without a check fails. Today the route-table approach exists for authentication only, in the earlier code (`every_route_needs_the_token` in `crates/ratchetd/tests/api/auth.rs`). Role checks do not exist.

**Audit.** Check: a test that every write route produces exactly one audit entry with an actor and a name, in the same transaction as the change, and that a denial is audited too. There is no audit log yet.

**Secrets.** Check: a test that sets a key, a token and a connector secret and finds none in any response, event, error, log or audit entry. Today, in the earlier code, settings secrets answered as flags (`crates/ratchetd/tests/api/contract.rs`, `api/CONTRACT.md`); a scan of events and logs was to add.

**No surprise calls.** Check: a test that a self-hosted server makes no request to a host its configuration does not name, and that Ratchet contacts only the agent platforms and connectors an organization configured. To add. Today, in the earlier code, base URLs for GitHub were overridable by environment variable (`crates/ratchet-github/src/lib.rs`), with no test for the rule.

Earlier-code facts are in [today.md](today.md).

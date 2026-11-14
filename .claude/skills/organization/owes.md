# What organization owes

| Promise | The rule |
| --- | --- |
| Permission | Every read and write is checked against the organization and project role, in one place. |
| Audit | Every write is a named action with an actor, recorded in the audit log. |
| Secrets | No response, event or log carries a secret. |
| No surprise calls | A self-hosted server contacts only the endpoints its administrator configured. A runner contacts only the Ratchet server it was installed for. |

## How each is checked

**Permission.** Check: one test per route that the role matrix is applied, driven from the route table, so a new route without a check fails. The route-table approach exists for authentication: `every_route_needs_the_token` in `crates/ratchetd/tests/api/auth.rs` walks `route_table()` in `crates/ratchetd/src/openapi.rs`. Role checks do not exist, so the role version is to add.

**Audit.** Check: a test that every write route produces exactly one audit entry with an actor and a name. To add. There is no audit log in the repo.

**Secrets.** Check: tests that settings and connector responses carry only "is set", never a value. Partly exists: `crates/ratchetd/tests/api/contract.rs` checks that settings secrets answer as flags, and `api/CONTRACT.md` records that no response can carry a secret. A scan of events and logs for secret values is to add.

**No surprise calls.** Check: a test that a self-hosted build makes no request to a host the configuration does not name. To add. Today base URLs for GitHub are overridable by environment variable (`crates/ratchet-github/src/lib.rs`), but no test asserts the rule.

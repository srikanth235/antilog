# What organization owes

| Promise | The rule |
| --- | --- |
| Permission | Every read and write is checked against the member's organization-wide role, in one place. |
| Approval | A tool call that matters runs only after a person with the right role approves its card, or policy pre-approves it. Add credential is never pre-approved. |
| Audit | Every tool call that changes something, and every approval, is recorded with an actor. Changing agent platform accounts is among them. |
| Budget | An experiment never spends past its budget, and an organization never past its monthly cap. |
| Secrets | No response, event or log carries a secret. |
| No surprise calls | A self-hosted server contacts only the endpoints its administrator configured. A session contacts only the endpoints its environment allows. |

## How each is checked

Each check is (target). Earlier-code facts are in [today.md](today.md).

**Permission.** A test per route that the role matrix is applied, driven from the route table in the OpenAPI document, so a new route without a check fails.

**Approval.** A test that each card kind runs its tool only after approval, that a viewer cannot approve, and that Add credential cannot be set to Allow.

**Audit.** A test that every write route and every approval produces exactly one audit entry with an actor and a name, in the same transaction as the change, and that a denial is audited too.

**Budget.** A test that the judge refuses `measure` when a budget is spent, that the experiment's sessions become Waiting for you with a Raise budget card, and that nothing raises a budget past the monthly cap, and a person with the right role can change it any time.

**Secrets.** A test that sets a key, a token and a credential and finds none in any response, event, error, log or audit entry.

**No surprise calls.** A test that a self-hosted server makes no request to a host its configuration does not name.

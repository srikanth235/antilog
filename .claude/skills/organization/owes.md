# What organization owes

| Promise | The rule |
| --- | --- |
| Permission | Every read and write is checked against the member's role in the project it touches (the organization role unless the project set one), in one place. |
| Approval | A tool call that matters runs only after a person with the right role approves its card, or policy pre-approves it. Add credential is never pre-approved. |
| Audit | Every tool call that changes something, and every approval, is recorded with an actor. Starting, messaging and stopping a session, changing a session's budget, changing model keys, project membership, a project's or the organization's limits, and policy are among them. |
| Budget | A session never spends past its own cap, a project never past its monthly cap, and an organization never past its monthly cap. |
| Secrets | No response, event or log carries a secret. |
| No surprise calls | A self-hosted server contacts only the endpoints its administrator configured. A session contacts only the endpoints its environment allows. |

## How each is checked

Built: the permission matrix and project override (`antilog-core/tests/organization.rs`), every route refusing a caller with no credential (walked from the OpenAPI document, `antilog-server/tests/api.rs`), approvals by role, policy and audit rows, budgets and nested caps, and no secret in a list or the audit log. Target: the route walk as every role, a test per card kind, and the no-surprise-calls test.

**Permission.** A test per route that the role matrix is applied, driven from the route table in the OpenAPI document, so a new route without a check fails. A test that a project's role overrides the organization role for that project only, and that a session is never reachable through a project its caller has no role in.

**Approval.** A test that each of the six card kinds runs its tool only after approval, that a viewer cannot approve, and that Add credential cannot be set to Allow.

**Audit.** A test that every write route and every approval produces exactly one audit entry with an actor and a name, in the same transaction as the change, and that a denial is audited too.

**Budget.** A test that the gateway refuses a model call when the session's, the project's or the organization's cap is spent, that a session's cap cannot pass its project's cap and a project's cannot pass the organization's (409 `over_cap`), and that a person with the right role can change a session's cap any time.

**Secrets.** A test that sets a key, a token and a credential and finds none in any response, event, error, log or audit entry.

**No surprise calls.** A test that a self-hosted server makes no request to a host its configuration does not name.

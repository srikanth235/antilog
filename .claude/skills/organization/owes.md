# What organization owes

| Promise | The rule |
| --- | --- |
| Permission | Every read and write is checked against the member's role in the project it touches (the organization role unless the project set one), in one place. |
| Approval | A tool call that matters runs only after a person with the right role approves its card, or policy pre-approves it. Add credential is never pre-approved, and a value model is set or changed only with a project owner's approval. |
| Audit | Every tool call that changes something, and every approval, is recorded with an actor. Changing agent platform accounts, project membership, a project's budget, a value model, a hand off, a failover and exporting a report are among them. |
| Budget | An experiment never spends past its budget, a project never past its monthly cap, if it has one, and an organization never past its monthly cap. |
| Secrets | No response, event or log carries a secret. |
| No surprise calls | A self-hosted server contacts only the endpoints its administrator configured. A session contacts only the endpoints its environment allows. |

## How each is checked

Built: the permission matrix and project override (`antilog-core/tests/organization.rs`), every route refusing a caller with no credential (walked from the OpenAPI document, `antilog-server/tests/api.rs`), approvals by role, policy and audit rows, budgets and nested caps, and no secret in a list or the audit log. Target: the route walk as every role, a test per card kind, and the no-surprise-calls test.

**Permission.** A test per route that the role matrix is applied, driven from the route table in the OpenAPI document, so a new route without a check fails. A test that a project's role overrides the organization role for that project only, and that an experiment is never reachable through a project its caller has no role in.

**Approval.** A test that each of the eight card kinds runs its tool only after approval, that a changed value model needs a project owner, that a viewer cannot approve, and that Add credential cannot be set to Allow.

**Audit.** A test that every write route and every approval produces exactly one audit entry with an actor and a name, in the same transaction as the change, and that a denial is audited too.

**Budget.** A test that the judge refuses `measure` when a budget is spent, that the experiment's sessions become Waiting for you with a Raise budget card, and that nothing raises a budget past the monthly cap, and a person with the right role can change it any time. A test that an experiment's budget cannot pass its project's cap, and a project's cap cannot pass the organization's.

**Secrets.** A test that sets a key, a token and a credential and finds none in any response, event, error, log or audit entry.

**No surprise calls.** A test that a self-hosted server makes no request to a host its configuration does not name.

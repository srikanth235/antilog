# Organization: objects and words

[`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md) does not define the access words below. "Default" and "Settings" there are unrelated to roles. None of the objects exist yet. Roles and settings belong to the organization.

| Object | Means | Not |
| --- | --- | --- |
| organization | The top owner of members, experiments and settings; every table row carries one | a GitHub org, a cloud account |
| member | A person in an organization, known by an identity from a sign-in provider | an account with a password |
| role | What a member may do across the organization: owner, editor or viewer | a job title |
| group mapping | A rule that maps a group claim from the identity provider to a role | a separate permission system |
| sign-in session | A signed-in browser or app session, short-lived | a Ratchet session, which is one agent's work inside an experiment |
| access token | A secret a program uses to act as a member; shown once, stored as a hash | a sign-in session |
| service account | A non-human member with its own tokens and a role | a shared login |
| approval card | A tool call a person approves in the chat | a settings form |
| policy | Per card kind: Always ask, Allow up to a limit, or Allow | a role |
| budget | What one experiment may spend across all its sessions and their judge time | the monthly cap |
| monthly cap | The most an organization may spend in a month | a budget |
| action | One named write: it checks permission, writes an audit entry and emits an event | a route, a handler |
| audit entry | An append-only record of a tool call that changed something, or of an approval: actor, name, target, time, outcome | a log line |
| event | A live message that something changed, carrying no secret | an audit entry |

Roles in one line each: viewer reads experiments and approves nothing; editor also starts experiments, talks to agents, approves cards and changes an experiment's budget; owner also manages members, sign-in, policy, agent platform accounts, the default budget and the cap. Roles are organization-wide: visibility does not follow repo access.

## How this area refers to the others

- Session and judge: by object id only. Organization never reads a measurement, a dataset row or a sandbox.
- Every other area calls the action layer for every write and reads the permission answer from one place.
- Audited actions are tool calls that change something and approvals.

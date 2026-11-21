# Organization: objects and words

The access words below are not in [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md). Organizations, members, projects, project roles, access tokens, policy, approvals, budgets and the audit log are in the store ([../change/ledger-table.md](../change/ledger-table.md)); sign-in sessions (a hashed random token in a cookie, twelve hours) and the action layer are built; group mappings are not. Roles belong to the organization and, where a project sets one, to the project.

| Object | Means | Not |
| --- | --- | --- |
| organization | The top owner of members, projects, sessions and settings; every table row carries one | a GitHub org, a cloud account |
| project | A group of sessions for one area of the code, inside an organization. Every session belongs to exactly one project. It has members, a description, an optional monthly cap and an optional default budget per session | a repo, a folder, a second organization |
| member | A person in an organization, known by an identity from a sign-in provider | an account with a password |
| role | What a member may do: owner, editor or viewer. The organization role is the default in every project | a job title |
| project member | A member with a role for one project. The role is the organization role unless a project owner sets another for that project | a second sign-in, a team |
| group mapping | A rule that maps a group claim from the identity provider to a role (target) | a separate permission system |
| sign-in session | A signed-in browser or app session, short-lived | an Antilog session, which is one agent's work in a project |
| access token | A secret a program uses to act as a member; shown once, stored as a hash | a sign-in session |
| service account | A non-human member with its own tokens and a role | a shared login |
| approval card | A tool call a person approves in the chat | a settings form |
| policy | Per card kind (six): Always ask, Allow up to a limit, or Allow | a role |
| model key | An organization's own key for one model provider (Anthropic, OpenAI or OpenCode Zen), set by an owner, kept sealed with the server's master key and never shown or sent into a session | an access token |
| session budget | A session's own optional cap, in US cents. It sits inside its project's monthly cap | a monthly cap |
| project cap | An optional monthly limit on the spend of all of a project's sessions, and the project's default budget per session (`PUT /projects/{id}/limits`: `monthlyCapCents`, `defaultBudgetCents`). It sits inside the organization's monthly cap | a session budget |
| monthly cap | The most an organization may spend in a month, with an organization default budget per session (`PUT /organization/limits`) | a session budget |
| running sessions cap | The most sessions an organization has on machines at once (10 by default, `maxRunningSessions`, set by an owner with the limits); queued sessions wait for a place | a budget |
| action | One named write: it checks permission, writes an audit entry and emits an event | a route, a handler |
| audit entry | An append-only record of a tool call that changed something, or of an approval: actor, name, target, time, outcome | a log line |
| event | A live message that something changed, carrying no secret | an audit entry |

## Roles

In a project: a viewer reads its sessions and approves nothing; an editor also starts sessions, messages and stops any session of the project, decides its cards and changes a session's budget; an owner also manages the project's roles and limits (the monthly cap and the default budget per session). Organization owners also create projects, manage members, sign-in, policy, the default agent, model keys, connectors, plugins, tokens for others, the organization's limits, remove skills and read the audit log. A person's organization role is their default in every project, and a project can set someone's role for that project, higher or lower. Visibility does not follow repo access.

`GET /budgets` returns `{organization: {capCents, spentCents}, projects: [{projectId, capCents, spentCents}]}`.

## How this area refers to the others

- Session: by session id and project id only. Organization never reads tool output, a connector's rows or a sandbox.
- Every other area calls the action layer for every write and reads the permission answer from one place.
- Audited actions are tool calls that change something and approvals.

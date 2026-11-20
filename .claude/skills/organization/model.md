# Organization: objects and words

The access words below are not in [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md). Organizations, members, projects, project roles, access tokens, policy, approvals, budgets and the audit log are in the store ([../change/ledger-table.md](../change/ledger-table.md)); sign-in sessions (a hashed random token in a cookie, twelve hours) and the action layer are built; group mappings are not. Roles belong to the organization and, where a project sets one, to the project.

| Object | Means | Not |
| --- | --- | --- |
| organization | The top owner of members, experiments and settings; every table row carries one | a GitHub org, a cloud account |
| project | A group of experiments for one area of the code, inside an organization. Every experiment belongs to exactly one project. It has members, a description, an optional monthly cap and an optional default budget per experiment | a repo, a folder, a second organization |
| member | A person in an organization, known by an identity from a sign-in provider | an account with a password |
| role | What a member may do: owner, editor or viewer. The organization role is the default in every project | a job title |
| project member | A member with a role for one project. The role is the organization role unless a project owner sets another for that project | a second sign-in, a team |
| group mapping | A rule that maps a group claim from the identity provider to a role | a separate permission system |
| sign-in session | A signed-in browser or app session, short-lived | an Antilog session, which is one agent's work inside an experiment |
| access token | A secret a program uses to act as a member; shown once, stored as a hash | a sign-in session |
| service account | A non-human member with its own tokens and a role | a shared login |
| approval card | A tool call a person approves in the chat | a settings form |
| value model | A project owner's figure for a metric, sealed with an experiment and never set by an agent (see [../judge/model.md](../judge/model.md)) | a budget, a price |
| report | A monthly page per organization and per project, exported as PDF and CSV and emailed to chosen members (contents in [notifications.md](notifications.md)) | a log, the audit log |
| ideas | Experiments Antilog suggests on a project page, ranked by expected value over cost. Antilog proposes, a person starts | a backlog an agent starts by itself |
| policy | Per card kind (nine): Always ask, Allow up to a limit, or Allow | a role |
| budget | What one experiment may spend across all its sessions and their judge time. It sits inside its project's cap | a monthly cap |
| project cap | An optional monthly limit on the spend of all of a project's experiments. It sits inside the organization's monthly cap | a budget |
| monthly cap | The most an organization may spend in a month | a budget |
| action | One named write: it checks permission, writes an audit entry and emits an event | a route, a handler |
| audit entry | An append-only record of a tool call that changed something, or of an approval: actor, name, target, time, outcome | a log line |
| event | A live message that something changed, carrying no secret | an audit entry |

Roles in one line each, in a project: viewer reads its experiments and approves nothing (a viewer of a project reads its report); editor also starts experiments, talks to agents, approves cards and changes an experiment's budget; owner also manages the project's members, its budget (the cap and the default per experiment), its value models and the project itself. Organization owners also manage members, sign-in, policy, the default agent, model keys, the default budget and the organization cap. A person's organization role is their default in every project, and a project can set someone's role for that project, higher or lower. Visibility does not follow repo access.

## How this area refers to the others

- Session and judge: by object id and project id only. Organization never reads a measurement, a dataset row or a sandbox.
- Every other area calls the action layer for every write and reads the permission answer from one place.
- Audited actions are tool calls that change something and approvals.

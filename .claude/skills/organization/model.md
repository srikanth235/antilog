# Organization: objects and words

`docs/GLOSSARY.md` does not define any of these words. "Default" and "Settings" there are unrelated to roles. None of the objects below exist yet, except the session token, which today is a per-boot bearer token and the MCP session token.

| Object | Means | Not |
| --- | --- | --- |
| organization | The top owner of projects, members and settings; every table row carries one | a GitHub org, a cloud account |
| project | A unit of work inside an organization; optimizations, connectors and accounts belong to one | a repo |
| member | A person in an organization, known by an identity from a sign-in provider | an account with a password |
| role | What a member may do on a project: owner, editor or viewer | a job title |
| group mapping | A rule that maps a group claim from the identity provider to a role | a separate permission system |
| session | A signed-in browser or app session, short-lived | an optimization session or a chat |
| access token | A secret a program uses to act as a member; shown once, stored as a hash | a session |
| service account | A non-human member with its own tokens and a role | a shared login |
| action | One named write: it checks permission, writes an audit entry and emits an event | a route, a handler |
| audit entry | An append-only record of an action: actor, name, target, time, outcome | a log line |
| event | A live message that something changed, carrying no secret | an audit entry |

Roles in one line each: viewer reads; editor reads and changes optimizations and runs; owner also manages members, accounts and settings.

## How this area refers to the others

- Optimization, context, infra: by object id and project id only. Organization never reads a verdict, a dataset row or a machine.
- Every other area calls the action layer for every write and reads the permission answer from one place.

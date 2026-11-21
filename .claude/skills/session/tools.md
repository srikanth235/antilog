# Session: the setup tools

Built in `antilog-core/src/tools/` (`mod.rs` holds the tool definitions and the dispatch, `setup.rs` the four tools) and `session/`: each setup tool creates an approval, and settling a decided card applies it (a raised budget raises the session's cap). Cards, policy and who may approve are in [../organization/approvals.md](../organization/approvals.md).

## How the tools reach a session

The runner reaches the server over outbound HTTPS only ([docs/V0.md](../../../docs/V0.md)), so a self-hosted server behind a firewall works. Each session gets one MCP endpoint, `/mcp/sessions/{id}` (built: streamable HTTP on rmcp, `antilog-server/src/api/mcp.rs`), reached with a bearer token for `Purpose::Mcp`. It serves the four setup tools and the tools of every organization connector, namespaced `<connector>__<tool>`. Each connector tool is On, Off or Ask first; see [add-connector.md](add-connector.md). The endpoint's instructions say what the setup tools do and that a person approves each request. The conversation keeps only a tool's name. There is no `antilog mcp` and no REST tool endpoint.

## The setup tools

Each shows a card in the chat. The agent never sees a credential value.

| Tool | Inputs | Card | Who approves | Refuses |
| --- | --- | --- | --- | --- |
| `attach_repo` | `repo` (owner/name) | Attach repo | An owner or editor of the session's project; policy may pre-approve | A name that is not owner/name; a viewer's approval |
| `request_credential` | `connector`, `reason`, `scope` | Add credential: with a secure form whose value is sealed on the server | An owner or editor, always; policy can never pre-approve it | A scope with write or admin rights; a value in chat |
| `set_environment` | `network`, `add_hosts`, `remove_hosts`, `set_variables`, `unset_variables`, `setup_script`, `add_repos`, `remove_repos` | Change environment: the change as a diff against the version the session runs, with what applies now and what at the next start | An owner or editor; policy may pre-approve | A call that names nothing to change or changes nothing; a variable named `ANTILOG_*` or a proxy name; a host that is not an exact name; hosts at a level other than limited |
| `request_budget` | `amount_cents`, `reason` | Raise budget, with what it may cost | An owner or editor; policy may allow up to a limit | A session with no cap of its own (it says so and points to Settings); an amount of zero or less |

An editor or owner can also change a session's cap at any time without a card (`PUT /sessions/{id}/budget`), inside its project's and the organization's caps. Each refusal reaches the agent as a plain reason, and the session shows a calm one-line card, never a blank.

## Changing the session's environment

Approving a Change environment card makes the environment's next version (built on its newest version; a session on the built-in default gets an environment of its own, `Session <id>`), moves the session to it and tells the agent what applied. The environment is shared: other sessions on it get the version when they start, and the card says so ("only this session" is not built). The network applies on the next tunnel, since the proxy reads the session's version on every one. A running machine is sent the version (`Command::ApplyEnvironment`): its variables reach commands run from then on (`BASH_ENV`, see [runner.md](runner.md)) and a changed setup script runs now, a failure being a note and a line for the agent; a machine no runner holds applies both when it next starts. Default repos the new version brings are cloned now. A person can do the same from the Overview (`PUT /sessions/{id}/environment`, `session.set_environment`, editors and owners), and the agent is told as it is of a decided card. Both write `environment.edit` or `environment.create` and `session.set_environment` to the audit log. Code: `antilog-core/src/session/environment.rs`.

## Rules for a new tool

- It names what would happen and what it may cost on its card.
- It changes nothing until approved, unless policy pre-approves it.
- A call that changes something, and its approval, is an audit row.

Follow [../change/SKILL.md](../change/SKILL.md) to add one.

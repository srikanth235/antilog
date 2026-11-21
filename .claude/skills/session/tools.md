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
| `set_environment` | `image`, `setup_script`, `size`, `network` | Change environment | An owner or editor; policy may pre-approve | A call that names nothing to change |
| `request_budget` | `amount_cents`, `reason` | Raise budget, with what it may cost | An owner or editor; policy may allow up to a limit | A session with no cap of its own (it says so and points to Settings); an amount of zero or less |

An editor or owner can also change a session's cap at any time without a card (`PUT /sessions/{id}/budget`), inside its project's and the organization's caps. Each refusal reaches the agent as a plain reason, and the session shows a calm one-line card, never a blank.

## Rules for a new tool

- It names what would happen and what it may cost on its card.
- It changes nothing until approved, unless policy pre-approves it.
- A call that changes something, and its approval, is an audit row.

Follow [../change/SKILL.md](../change/SKILL.md) to add one.

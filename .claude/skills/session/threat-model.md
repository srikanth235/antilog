# Session: threat model

What a hijacked agent could do, and what stops it Assume every agent can be prompt-injected: it reads text that people outside the organization can write, and it follows instructions in that text more often than anyone would like. The design must hold when the agent is working for someone else.

## Untrusted text an agent reads

- Repo files, including dependencies, vendored code, issues quoted in comments, and generated files.
- Connector data: rows, documents and whatever an MCP server returns.
- Skill packs and plugins an organization added.

A person's own messages in the chat are trusted as that person's, within their role.

## What an injected agent holds, and the limit on each

| It holds | The limit | What still gets through |
| --- | --- | --- |
| Repo access in its machine | What the repo's credentials allow (the real code host is not built; a pull request comes later, from the agent with the repo's credentials) | It can push anything those credentials allow, including data it read |
| A connector's tools through the session endpoint | The credential stays on the server; each tool is On, Off or Ask first and each call is audited | It can use any On tool with any input, and read what the connector returns |
| Network from its machine | Only the endpoints the environment allows | Any allowed endpoint that accepts writes is a way out |
| The setup tools | Cards ask a person unless policy pre-approved them; Add credential always asks | It can spend up to the session's, the project's and the organization's caps, and write misleading text on a card |
| The conversation | Stored as words, plus each tool call's label, input and output cut to 4 KB with credentials taken out | Whatever it writes in a message is stored, including data it read |

It never holds a model key or a cloud credential.

## Rules

- Treat every tool argument from an agent as untrusted input, checked by the action layer like a request from outside.
- Show the agent's text on a card as the agent's, never as Antilog's or a person's. A card shows exactly what will happen, not the agent's summary of it.
- Keep allowed endpoints narrow: an environment that allows a host that accepts writes says so on the Change environment card.
- When a session acts against its limits (reaches a blocked host, spends to a cap), record it in the audit log and tell a person.

## What this does not stop

Data the agent may read can leave through its pushes, its messages or an allowed endpoint. The defence is what the agent may reach: switch on only connectors, and set On only tools, whose answers the customer is willing to see in the agent's output. Say so where a connector is added ([add-connector.md](add-connector.md)).

# Session: threat model

What a hijacked agent could do, and what stops it (target). Assume every agent can be prompt-injected: it reads text that people outside the organization can write, and it follows instructions in that text more often than anyone would like. The design must hold when the agent is working for someone else.

## Untrusted text an agent reads

- Repo files, including dependencies, vendored code, issues quoted in comments, and generated files.
- Connector data: rows, documents and whatever an MCP server returns.
- Pull request review comments and CI output while a delivered session stays attached.
- The history of other sessions, whose hypotheses an injected agent may have written.
- Skill packs and plugins from Discover.

A person's own messages in the chat are trusted as that person's, within their role.

## What an injected agent holds, and the limit on each

| It holds | The limit | What still gets through |
| --- | --- | --- |
| A git token for its own branch | Pushes only to the session's branch, short-lived, one per repo | It can push anything to that branch, including data it read. The branch is on the code host |
| Read-only connector credentials | Read-only, leave the holdout out, short-lived | It can read every row the credential allows |
| Network from its sandbox | Only the endpoints the environment allows | Any allowed endpoint that accepts writes is a way out |
| The Antilog tools | Cards ask a person unless policy pre-approved them; Add credential always asks; `deliver` refuses a trial that is not Kept | It can spend the budget on trials, and write misleading hypotheses and pull request text |
| The conversation | Stored as words only, never tool output | Whatever it writes in a message is stored, including data it read |

It never holds the holdout credential, a write credential, a model key, a cloud credential or the judge's signing key. It cannot mark a win, change a sealed experiment or reach the judge sandbox.

## Rules

- Treat every tool argument from an agent as untrusted input, checked by the action layer like a request from outside.
- Show the agent's text on a card as the agent's, never as Antilog's or a person's. A card shows exactly what will happen, not the agent's summary of it.
- Keep allowed endpoints narrow: an environment that allows a host that accepts writes says so on the Change environment card.
- Scan what an agent pushes for secrets and for data that looks like connector rows before a pull request opens, and show a hit on the Open pull request card. A scan narrows the leak through git and does not close it.
- The second opinion also reads the pull request text, and flags instructions aimed at a reviewer.
- When a session acts against its experiment (tries a protected path, reaches a blocked host, asks for the holdout), record it on the timeline and in the audit log, and tell a person.

## What this does not stop

Data the agent may read can leave through its branch, its messages or an allowed endpoint. The defence is what the agent may read: connectors give it only what the customer is willing to see in a pull request. Say so where a connector is added ([add-connector.md](add-connector.md)).

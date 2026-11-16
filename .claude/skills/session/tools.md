# Session: spawn and the setup tools

All target. Every session gets the Ratchet tools. This page covers `spawn` and the four setup tools. The work tools `seal`, `measure`, `history`, `deliver` and `watch` are in [../judge/tools.md](../judge/tools.md). Cards, policy and who may approve are in [../organization/approvals.md](../organization/approvals.md).

## How the tools reach a session

Ratchet declares them as tools the client provides and answers them over the session's own event stream. No platform has to reach the server, so a self-hosted server behind a firewall works. Two points are unproven and checked in the spike:

- Microsoft Foundry Agent Service: the model emits a `function_call` and the caller returns a `function_call_output` with the same call id, but a documented 10-minute expiry for tool outputs may apply.
- Google Agent Runtime: answering a client tool over the stream is not documented, and bidirectional streaming is in preview with a 10-minute limit per query.

`measure` is asynchronous. It returns a ticket at once and the measurement arrives later as a session event, which sidesteps both limits.

`ratchet mcp` serves the same tools to an agent on a person's own machine (Claude Code, Codex), so a change can be proposed anywhere and is measured only by the judge.

## spawn

| | |
| --- | --- |
| Inputs | An agent (a model on a platform), an environment, a first message. The new session is in the same experiment |
| Card | Spawned session: agent, status, its best; opens that session as its own chat inside the experiment |
| Who approves | Nobody, inside the budget. The spawned session spends the experiment's budget |
| Refuses | A call from a spawned session; a budget that is spent; an agent whose platform account is not configured |

The parent is told when a spawned session finishes. A person can open any spawned session and talk to it, and can start sibling sessions from the composer.

## The setup tools

Each shows a card in the chat. The agent never sees a credential value.

| Tool | Inputs | Card | Who approves | Refuses |
| --- | --- | --- | --- | --- |
| `attach_repo` | A repo on the code host | Attach repo | An owner or editor; policy may pre-approve | A repo the code host account cannot reach; a viewer's approval |
| `request_credential` | A connector, a reason, a scope | Add credential: a secure form where a person types the value; it goes straight to the platform's credential store | An owner or editor, always; policy can never pre-approve it | A scope that includes the holdout or write rights; a value in chat |
| `set_environment` | Image or setup script, size, network | Change environment | An owner or editor; policy may pre-approve | A change that would exceed the budget's rate; hosts the platform cannot allow |
| `request_budget` | An amount and a reason | Raise budget, with what it may cost | An owner or editor; policy may allow up to a limit | An amount that takes the experiment's budget over the organization's monthly cap |

A person (editor or owner) can also change the experiment's budget at any time without a card, from the scoreboard strip, the experiment detail page or the Seal experiment card. Each refusal comes back to the agent as a plain reason, and the session shows a calm one-line card, never a blank.

## Rules for a new tool

- It names what would happen and what it may cost on its card.
- It changes nothing until approved, unless policy pre-approves it.
- A call that changes something, and its approval, is an audit row.
- It works the same through `ratchet mcp`.

Follow [../change/SKILL.md](../change/SKILL.md) to add one.

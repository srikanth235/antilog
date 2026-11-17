# Session: spawn and the setup tools

All target. Every session gets the Antilog tools. This page covers `spawn` and the four setup tools. The work tools `seal`, `measure`, `history`, `deliver` and `watch` are in [../judge/tools.md](../judge/tools.md). Cards, policy and who may approve are in [../organization/approvals.md](../organization/approvals.md).

## How the tools reach a session

Antilog declares them as tools the client provides and answers them over the session's own event stream. No platform has to reach the server, so a self-hosted server behind a firewall works. Two platform limits are unproven and checked in the spike ([add-platform.md](add-platform.md)).

`measure` is asynchronous: it returns a ticket at once and the measurement arrives later as a session event, which sidesteps both limits.

`antilog mcp` serves the same tools to an agent on a person's own machine (Claude Code, Codex), so a change can be proposed anywhere and is measured only by the judge.

## spawn

| | |
| --- | --- |
| Inputs | An agent (a model on a platform), an environment, a first message. The new session is in the same experiment. The call hands over a summary of what was tried (from `history`) so the new session does not repeat a dead end. With no agent named it defaults to the organization's chosen lower-cost agent; naming a costlier one needs a reason |
| Card | Spawned session: agent with its cost tier, status, its best; opens that session as its own chat inside the experiment |
| Who approves | Nobody, inside the budget. The spawned session spends the experiment's budget |
| Refuses | A call from a spawned session; a budget that is spent; an agent whose platform account is not configured |

The parent is told when a spawned session finishes. A person can open any spawned session and talk to it, and can start sibling sessions from the composer.

## The setup tools

Each shows a card in the chat. The agent never sees a credential value.

| Tool | Inputs | Card | Who approves | Refuses |
| --- | --- | --- | --- | --- |
| `attach_repo` | A repo on the code host | Attach repo | An owner or editor of the experiment's project; policy may pre-approve | A repo the code host account cannot reach; a viewer's approval |
| `request_credential` | A connector, a reason, a scope | Add credential: connector-aware (see [add-connector.md](add-connector.md)), with a secure form whose value goes straight to the platform's credential store | An owner or editor, always; policy can never pre-approve it | A scope that includes the holdout or write rights; a value in chat |
| `set_environment` | Image or setup script, size, network | Change environment | An owner or editor; policy may pre-approve | A change that would exceed the budget's rate; hosts the platform cannot allow |
| `request_budget` | An amount and a reason | Raise budget, with what it may cost | An owner or editor; policy may allow up to a limit | An amount that takes the experiment's budget over its project's cap or the organization's monthly cap |

An editor or owner can also change the experiment's budget at any time without a card. Each refusal reaches the agent as a plain reason, and the session shows a calm one-line card, never a blank.

## Rules for a new tool

- It names what would happen and what it may cost on its card.
- It changes nothing until approved, unless policy pre-approves it.
- A call that changes something, and its approval, is an audit row.
- It works the same through `antilog mcp`.

Follow [../change/SKILL.md](../change/SKILL.md) to add one.

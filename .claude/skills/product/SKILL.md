---
name: product
description: >-
  What is Ratchet, what do we owe the customer, and what do we refuse to build? Use for any product, scope or trade-off question; "should we", "is this in scope", "which area owns this"; adding a feature, page, card, tool or integration; naming or wording anything a user reads; deciding what to refuse; and whenever a change touches sessions, the judge (contracts, measurements, the holdout, acceptance), approvals, budgets, or members, roles and the audit log. It states the target product as settled rulings, the governing principles, the non-goals, the open calls and when to stop and ask the owner.
---

# Product

Ratchet is a chat and a judge. The chat works like Claude Code on the web: a person talks to a coding agent that works in a sandbox with repos attached. The judge is a set of tools every session gets. An agent asks the judge to measure a change, and only the judge's numbers count. Only a measured win becomes a pull request. There are no modes, no plan step and no orchestration loop on the server: the agent drives, and the server relays sessions, runs the judge and keeps a ledger.

Three objects hold it together: a **session** (a chat with one agent in one sandbox), a **contract** (a sealed definition of better) and a **measurement** (the judge's signed result for one trial). Everything else is a view over measurements. Settings are tools: setup happens in the chat as an approved tool call, and a settings page lists and revokes what was approved. The organization floor (members, roles, sign-in, tokens, approvals, budgets, audit log, notifications) sits under all of it.

The target is a server with a web app as the client of its API, run two ways: self-hosted (a Docker Compose service, server plus Postgres, using the agent platform accounts the customer already has) or Ratchet cloud (we host the server and run every session on Ratchet's platform accounts). Ratchet runs no sandboxes for agents: they run as sessions on agent platforms (Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service, Google Agent Runtime) behind one trait. Nothing here is built (target).

Use the governing principles and the non-goals below as settled. Apply them directly when they answer a question. The rest is in the files below.

## Governing principles

1. **A chat and a judge.** Every feature belongs to the session, the judge or the organization floor. If it fits none, question it.
2. **The judge decides, never a person or an agent.** The moment a hand or a proposer can mark a win, no win means anything.
3. **Settings are tools.** Setup happens in the chat as an approved tool call. A settings page lists and revokes.
4. **The agent drives.** The server relays, measures and records. It runs no loop.
5. **Ratchet never holds the data.** Ids, versions, numbers, states, the conversation's words and audit rows only. Data is read only inside a platform's sandbox.
6. **Everything is governed.** Approvals by role, every change recorded.
7. **Derive, do not store.** Value, Return and status come from measurements.
8. **Failure is never shown as zero or empty, and nothing unchecked is shown as healthy.**
9. **One API, every client.** One set of tools, the same words everywhere.
10. **Self-hosting is first-class.** No call to a service the administrator did not configure.

## Where the rulings are

| If the change touches | Read |
| --- | --- |
| What every area owes the customer | [promises.md](promises.md) |
| What we decided not to build | [non-goals.md](non-goals.md) |
| The design as dated rulings, what it replaces, the open calls | [rulings.md](rulings.md) |
| Sessions, agents, platforms, environments, credentials, connectors, spawn, budgets | [session owes](../session/owes.md), [rules](../session/rules.md) |
| Contracts, seal, measure, acceptance, the holdout, anti-cheat, receipts, watch | [judge owes](../judge/owes.md), [rules](../judge/rules.md) |
| Members, roles, sign-in, tokens, approvals, policy, the audit log, notifications | [organization owes](../organization/owes.md), [rules](../organization/rules.md) |
| How a screen looks, behaves and reads | the [design](../design/SKILL.md) skill |
| How a change crosses the layers | the [change](../change/SKILL.md) skill |

## Decision process

Ask these in order. The first one that bites usually settles it.

1. **Is it a non-goal?** A shaped version of one (a "light" judge on a laptop, a "just this once" stored password, a "quick" hand-marked win) is still the non-goal.
2. **Which area owns it?** The session, the judge or the organization floor. If none, question the feature before building it.
3. **Does it break a promise, or a never-bend rule, of that area?** Read the area's `owes.md` and `rules.md`.
4. **Does it make the server connect to data or store it, or read data anywhere other than a platform's sandbox?** If yes, the design is wrong.
5. **Can an agent or a person change what counts?** The holdout, the sealed contract, the judge sandbox and the acceptance criteria are not the proposer's. Chat can change what an agent tries, never what counts.
6. **Is it setup that asks before the first message, or lives only on a settings page?** Make it an approved tool call in the chat. Settings lists and revokes.
7. **Does it add a second mechanism for a job that has one?** Use the one that exists.
8. **Does it work self-hosted, with no outside calls?** Only configured endpoints may be contacted.
9. **Is it the same in every client?** Same state, same words, same behaviour.

If a question survives all of these, it is a product call. Write the options with a recommendation and what each costs, and ask the owner. Do not decide it by implementing it.

## Stop and ask the owner

- A non-goal would have to be reversed.
- A promise or a never-bend rule (the judge's trust rules, the anti-cheat) would change, or a new one is needed.
- A new dependency, a new outside system or a new hosting assumption, including a customer's CI runner as a judge sandbox.
- Anything that stores or moves customer data, or touches regulated-customer readiness.
- Anything that touches one of the open calls in [rulings.md](rulings.md).
- A new card kind, or a card that could be pre-approved when it should always ask.
- The licence, pricing or what Ratchet cloud offers beyond the settled rulings.

## Before you finish

- Name the area that owns the change: session, judge or organization.
- Name the promises it touches and how each stays kept.
- Say what is target and what is built. Mark target as "(target)" in docs.
- No unbuilt thing is described as existing.
- Wording follows [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md): "session" and "contract", "Return", never "automation" or "ROI". "Experiment" is retired pending the owner's confirmation.
- Open product calls are written down as options, not decided.

---
name: product
description: >-
  What is Antilog, what do we owe the customer, and what do we refuse to build? Use for any product, scope or trade-off question; "should we", "is this in scope", "which area owns this"; adding a feature, page, card, tool or integration; naming or wording anything a user reads; deciding what to refuse; and whenever a change touches sessions, projects, approvals, budgets, or members, roles and the audit log. It states the v0 product (cloud sessions only; experiments and the judge are parked) as settled rulings, the governing principles, the non-goals, the owner's decisions, the open questions and when to stop and ask the owner.
---

# Product

> The v0 design is in [docs/V0.md](../../../docs/V0.md) and wins over this page and its rulings.

Antilog is a chat with a coding agent in a cloud session, as Claude Code's cloud sessions are. A person states a goal in a project and talks to an agent that runs in a machine of its own with repos attached. The agent asks for setup (a repo, a credential, an environment change, more budget) through Antilog's tools, and a person approves each request. Spend is capped by the organization, the project and, optionally, the session. There are no modes, no plan step and no orchestration loop on the server: the agent drives, and the server relays sessions and keeps a ledger.

Experiments and the judge (measuring a change, trials, wins, receipts, value) are **parked**, not redesigned (owner, 21 Nov 2026). Their code is deleted; commit `daa445f` keeps it. See [judge](../judge/SKILL.md) and the Parked section of [rulings](rulings.md). Do not build toward them, and do not leave a hook for them.

Three things hold it together. A **project** groups sessions for one area of the code, with roles per project. A **session** (one agent's work in one machine) belongs to one project. A **card** is a tool call a person approves. Reusable setup (skill packs, connectors, agents, plugins) lives in one Customize page. The organization floor (members, roles, projects, sign-in, tokens, approvals, budgets, audit log) sits under all of it.

The target is a server with a web app as the client of its API. It runs two ways with Postgres: Antilog cloud (phase 1) and self-hosted (phase 2, in the client's own cloud account). See [rulings](rulings.md). Agents run as sessions on a runner, one machine per session ([docs/V0.md](../../../docs/V0.md)). The runner and the proxy are built; the real code host is not. Customers bring their own model keys; the proxy meters the spend.

## Governing principles

1. **A chat in a cloud session.** Every feature belongs to the session or the organization floor, and is shown inside a project, a session or Customize. If it fits neither, question it.
2. **The agent drives.** The server relays and records. It runs no loop.
3. **Settings are tools.** Setup happens in the chat as an approved tool call. A settings page lists and revokes.
4. **Antilog never holds the data.** Ids, states, the conversation's words, spend and audit rows only. Data is read only inside the session's machine.
5. **Everything is governed.** Approvals by role, every change recorded, spend capped.
6. **Derive, do not store.** Status and spend totals come from rows already kept.
7. **Failure is never shown as zero or empty, and nothing unchecked is shown as healthy.**
8. **One API, every client.** Same words everywhere.
9. **Self-hosting is first-class.** No call to a service the administrator did not configure.

## Where the rulings are

| If the change touches | Read |
| --- | --- |
| What every area owes the customer | [promises.md](promises.md) |
| What we decided not to build | [non-goals.md](non-goals.md) |
| The settled product, the owner's decisions, the parked judge and the open questions | [rulings.md](rulings.md) |
| Sessions, agents, environments, credentials, connectors, skill packs | [session owes](../session/owes.md), [rules](../session/rules.md) |
| Projects, members, roles, sign-in, tokens, approvals, policy, budgets, the audit log | [organization owes](../organization/owes.md), [rules](../organization/rules.md) |
| The parked experiments and judge | [judge](../judge/SKILL.md) |
| How a screen looks, behaves and reads | the [design](../design/SKILL.md) skill |
| How a change crosses the layers | the [change](../change/SKILL.md) skill |

## Decision process

Ask these in order. The first one that bites usually settles it.

1. **Is it a non-goal?** A shaped version of one is still the non-goal.
2. **Is it parked?** Experiments, trials, measuring, wins, receipts and value are parked. A request for one is a request to bring the judge back: ask the owner.
3. **Which area owns it?** The session or the organization floor. If neither, question the feature before building it.
4. **Does it break a promise, or a never-bend rule, of that area?** Read the area's `owes.md` and `rules.md`.
5. **Does it make the server connect to data or store it, or read data anywhere other than the session's machine?** If yes, the design is wrong.
6. **Is it setup that asks before the first message, or lives only on a settings page?** Make it an approved tool call in the chat. Settings lists and revokes.
7. **Does it add a second mechanism for a job that has one?** Use the one that exists.
8. **Does it work self-hosted, with no outside calls?** Only configured endpoints may be contacted.
9. **Is it the same in every client?** Same state, same words, same behaviour.

If a question survives all of these, it is a product call. Write the options with a recommendation and what each costs, and ask the owner. Do not decide it by implementing it.

## Stop and ask the owner

- A non-goal would have to be reversed, or the parked judge would come back.
- A promise or a never-bend rule would change, or a new one is needed.
- A new dependency, a new outside system or a new hosting assumption.
- Anything that stores or moves customer data, or touches regulated-customer readiness.
- Anything that touches one of the open questions in [rulings.md](rulings.md).
- A new card kind, or a card that could be pre-approved when it should always ask.
- Anything that favours one agent's vendor.
- The licence, pricing or what Antilog cloud offers beyond the settled rulings.

## Before you finish

- Name the area that owns the change: session or organization.
- Name the promises it touches and how each stays kept.
- Say what is target and what is built. Mark target as "(target)" in docs.
- No unbuilt or parked thing is described as existing.
- Wording follows [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md): "session" is the noun for what a user starts, it belongs to a project; "Raise budget", never "top up".
- Open product questions are written down as options, not decided.

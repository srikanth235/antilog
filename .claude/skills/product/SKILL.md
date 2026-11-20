---
name: product
description: >-
  What is Antilog, what do we owe the customer, and what do we refuse to build? Use for any product, scope or trade-off question; "should we", "is this in scope", "which area owns this"; adding a feature, page, card, tool or integration; naming or wording anything a user reads; deciding what to refuse; and whenever a change touches sessions, the judge (experiments, measurements, the holdout, acceptance), approvals, budgets, or members, roles and the audit log. It states the target product as settled rulings, the governing principles, the non-goals, the owner's decisions, the open questions and when to stop and ask the owner.
---

# Product

> The v0 design is in [docs/V0.md](../../../docs/V0.md) and wins over this page and its rulings.

Antilog is a chat and a judge. The chat works like Claude Code on the web: a person talks to a coding agent that works in a machine with repos attached. The judge is a set of tools every session gets. An agent asks the judge to measure a change, and only the judge's numbers count. Only a measured win becomes a pull request. There are no modes, no plan step and no orchestration loop on the server: the agent drives, and the server relays sessions, runs the judge and keeps a ledger.

Four things hold it together. A **project** groups the experiments for one area of the code, with roles per project. An **experiment** (the sealed goal and everything measured against it) belongs to one project. A **session** (one agent's work in one machine) belongs to an experiment. A **measurement** is the judge's signed result for one trial. Reusable setup (evaluations, skill packs, connectors, agents, plugins) lives in one Customize page. Everything else is a view over measurements. Settings are tools: setup happens in the chat as an approved tool call, and a settings page lists and revokes what was approved. The organization floor (members, roles, projects, sign-in, tokens, approvals, budgets, audit log, notifications) sits under all of it.

The target is a server with a web app as the client of its API, sold as services: our engineers deliver engagements on it, and client engineers take over. It runs two ways with Postgres: Antilog cloud (phase 1, on our AWS account) and self-hosted (phase 2, in the client's own AWS, Azure or GCP account). See the business model and deployment in [rulings](rulings.md). Agents run as sessions on a runner, one machine per session ([docs/V0.md](../../../docs/V0.md)). Nothing here is built (target).

## The three aims

Beside trust (the judge's rules never bend), the product serves three aims. Every feature is checked against them.

1. **Cost control.** Every dollar and token of an experiment is spent well: cost per trial, an estimate before spending, a quick check before the full measurement, a pause when trials stop paying (patience), history read first, the right-sized agent, compaction, and a view of where the spend went.
2. **No tie to one model provider.** Agents are measured against each other on the customer's own code (the agent scoreboard), Auto picks among the agents the organization turned on, a session can be handed off to another agent, and a failing agent fails over. No agent is chosen by brand.
3. **Return for the organization.** A value model says what a unit of the metric is worth, a project owner sets it, and Return counts everything spent, including experiments that found nothing. Ideas are ranked by expected value over expected cost, and a monthly report shows it.

Use the governing principles and the non-goals below as settled. Apply them directly when they answer a question. The rest is in the files below.

## Governing principles

1. **A chat and a judge.** Every feature belongs to the session, the judge or the organization floor, and is shown inside an experiment, a project or Customize. If it fits none, question it.
2. **The judge decides, never a person or an agent.** The moment a hand or a proposer can mark a win, no win means anything.
3. **Settings are tools.** Setup happens in the chat as an approved tool call. A settings page lists and revokes.
4. **The agent drives.** The server measures and records. It runs no loop.
5. **Antilog never holds the data.** Ids, versions, numbers, states, the conversation's words and audit rows only. Data is read only inside the session's machine.
6. **Everything is governed.** Approvals by role, every change recorded.
7. **Derive, do not store.** Value, Return, cost per trial and status come from measurements and spend records.
8. **Failure is never shown as zero or empty, and nothing unchecked is shown as healthy.**
9. **One API, every client.** One set of tools, the same words everywhere.
10. **Self-hosting is first-class.** No call to a service the administrator did not configure.

## Where the rulings are

| If the change touches | Read |
| --- | --- |
| What every area owes the customer | [promises.md](promises.md) |
| What we decided not to build | [non-goals.md](non-goals.md) |
| The settled product, the owner's decisions and the open questions | [rulings.md](rulings.md) |
| Sessions, agents, environments, credentials, connectors, skill packs, spawn, budgets | [session owes](../session/owes.md), [rules](../session/rules.md) |
| Experiments, evaluation templates, seal, measure, acceptance, the holdout, anti-cheat, receipts, watch | [judge owes](../judge/owes.md), [rules](../judge/rules.md) |
| Projects, members, roles, sign-in, tokens, approvals, policy, budgets, the audit log, notifications | [organization owes](../organization/owes.md), [rules](../organization/rules.md) |
| How a screen looks, behaves and reads | the [design](../design/SKILL.md) skill |
| How a change crosses the layers | the [change](../change/SKILL.md) skill |

## Decision process

Ask these in order. The first one that bites usually settles it.

1. **Is it a non-goal?** A shaped version of one (a "light" judge on a laptop, a "just this once" stored password, a "quick" hand-marked win) is still the non-goal.
2. **Which area owns it?** The session, the judge or the organization floor. If none, question the feature before building it.
3. **Does it break a promise, or a never-bend rule, of that area?** Read the area's `owes.md` and `rules.md`.
4. **Does it make the server connect to data or store it, or read data anywhere other than the session's machine?** If yes, the design is wrong.
5. **Can an agent or a person change what counts?** The holdout, the sealed experiment, the acceptance criteria are not the proposer's. Chat can change what an agent tries, never what counts.
6. **Is it setup that asks before the first message, or lives only on a settings page?** Make it an approved tool call in the chat. Settings lists and revokes.
7. **Does it add a second mechanism for a job that has one?** Use the one that exists.
8. **Does it work self-hosted, with no outside calls?** Only configured endpoints may be contacted.
9. **Is it the same in every client?** Same state, same words, same behaviour.

If a question survives all of these, it is a product call. Write the options with a recommendation and what each costs, and ask the owner. Do not decide it by implementing it.

## Stop and ask the owner

- A non-goal would have to be reversed.
- A promise or a never-bend rule (the judge's trust rules, the anti-cheat) would change, or a new one is needed.
- A new dependency, a new outside system or a new hosting assumption.
- Anything that stores or moves customer data, or touches regulated-customer readiness.
- Anything that touches one of the open questions in [rulings.md](rulings.md).
- A new card kind, or a card that could be pre-approved when it should always ask.
- Anything that lets an agent set a value model, or that favours one agent's vendor.
- The licence, pricing or what Antilog cloud offers beyond the settled rulings.

## Before you finish

- Name the area that owns the change: session, judge or organization.
- Name the promises it touches and how each stays kept.
- Say what is target and what is built. Mark target as "(target)" in docs.
- No unbuilt thing is described as existing.
- Wording follows [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md): "experiment" is the noun for what a user creates, it belongs to a project, and a session is one agent's work inside it; "Return", never "contract", "automation" or "ROI".
- Open product questions are written down as options, not decided.

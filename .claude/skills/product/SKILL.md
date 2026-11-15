---
name: product
description: >-
  What is Ratchet, what do we owe the customer, and what do we refuse to build? Use for any product, scope or trade-off question; "should we", "is this in scope", "which area owns this"; adding a feature, page, setting or integration; naming or wording anything a user reads; deciding what to refuse; and whenever a change touches the benchmark and accept rule, datasets and the holdout, clouds and spend, or members, roles and the audit log. It states the target product as settled rulings, the governing principles, the non-goals, and when to stop and ask the owner.
---

# Product

Ratchet points coding agents at one measurable goal and lets only the changes that beat a trusted benchmark move forward. Agents propose. A benchmark decides. A win carries its measurements and is kept. The product is organised around three basics and a shared floor: optimization (what "better" means), context (what a proposer may know) and infrastructure (where work happens), over the organization (members, roles, sign-in, tokens, audit log). Today Ratchet is a local daemon and a CLI. The target is a server product with a web app as the client of its API, run two ways: self-hosted (a Docker Compose service, server plus Postgres, in the customer's infrastructure) or Ratchet cloud (we host the server). Work runs in the customer's infrastructure, or, if an owner chooses Ratchet-managed on Ratchet cloud, in Ratchet's sandboxes. This skill states the target as settled rulings. What exists today is described in `stack`, `land`, `change` and `design`.

Use the governing principles and the non-goals below as settled. Apply them directly when they answer a question. The rest is in the files below.

## Governing principles

1. **Three basics.** Every feature belongs to optimization, context or infrastructure, or to the organization floor. If it fits none, question it. A fourth home is how a product turns into a pile of pages.
2. **Optimization depends on context and infrastructure; they never depend on it.** Set up once, reused by many experiments. A reverse dependency makes the shared parts impossible to change.
3. **A benchmark decides, never a person or an agent.** The moment a hand or a proposer can mark a win, no win means anything.
4. **Work and data stay in the customer's infrastructure, unless the owner chooses Ratchet-managed.** The server orchestrates and records. It never connects to data and stores none: only ids, versions, numbers, states and audit rows. Data that passes through us is data we must secure. Under Ratchet-managed, agent code reads data in our sandboxes, removed when the run ends, and nothing is kept.
5. **Everything is governed.** Who may define, grant, spend and approve is decided by role, and every write is recorded. A team cannot trust what it cannot audit.
6. **Derive, do not store.** Value, return and status are computed from measured results. A stored figure is a second truth that drifts.
7. **Failure is never shown as zero or as empty, and nothing unchecked is shown as healthy.** A false all-clear is the worst failure a result page can have.
8. **One API, every client.** The web app (target), the CLI and MCP show the same state in the same words. Two answers to one question is a bug in one of them.
9. **One mechanism per job.** One action layer, one event stream, one way to reach each outside system. The second mechanism is the one that goes stale.
10. **Self-hosting is first-class.** Nothing may require a call to a service the administrator did not configure. A server that phones home cannot be run in a locked-down network. Ratchet cloud is supported beside it, never instead of it.

## Where the rulings are

| If the change touches | Read |
| --- | --- |
| What every area owes the customer | [promises.md](promises.md) |
| What we decided not to build | [non-goals.md](non-goals.md) |
| Boundaries between areas, and the open questions | [rulings.md](rulings.md) |
| The benchmark, the accept rule, what may change | [optimization owes](../optimization/owes.md), [rules](../optimization/rules.md) |
| Connectors, datasets, grants, the holdout, notes | [context owes](../context/owes.md), [rules](../context/rules.md) |
| Clouds, environments, workspaces, agents, budgets, clean-up | [infra owes](../infra/owes.md), [rules](../infra/rules.md) |
| Members, roles, sign-in, tokens, the audit log | [organization owes](../organization/owes.md), [rules](../organization/rules.md) |
| The web client's shape, skills and plugins | [rulings.md](rulings.md) |
| How a screen looks, behaves and reads | the [design](../design/SKILL.md) skill |
| How a change crosses the layers | the [change](../change/SKILL.md) skill |

## Decision process

Ask these in order. The first one that bites usually settles it.

1. **Is it a non-goal?** A shaped version of one (a "light" evaluator on a laptop, a "just this once" stored password) is still the non-goal.
2. **Which basic owns it?** Optimization, context, infrastructure or the organization floor. If none, question the feature before building it.
3. **Does it break a promise, or a never-bend rule, of that area?** Read the area's `owes.md` and `rules.md`.
4. **Does it move data or work out of the customer's infrastructure without the owner's choice of Ratchet-managed, or make the server connect to data?** If yes, the design is wrong.
5. **Does it let a proposer change the scoring or read the holdout?** The holdout, the evaluation, the evaluator and the accept rule are not the proposer's. It may see each trial's measured value on the evaluation data.
6. **Does it add a second mechanism for a job that has one?** Use the one that exists.
7. **Does it work self-hosted, with no outside calls?** Only configured endpoints may be contacted.
8. **Is it the same in every client?** Same state, same words, same behaviour.

If a question survives all of these, it is a product call. Write the options with a recommendation and what each costs, and ask the owner. Do not decide it by implementing it.

## Stop and ask the owner

- A non-goal would have to be reversed.
- A promise or a never-bend rule would change, or a new one is needed.
- A new dependency, a new outside system or a new hosting assumption.
- Anything that stores or moves customer data, or touches regulated-customer readiness.
- Whether Ratchet drafts something or the customer brings it (see the open questions in [rulings.md](rulings.md)).
- The licence, pricing or what Ratchet cloud offers beyond the settled rulings.

## Before you finish

- Name the basic that owns the change.
- Name the promises it touches and how each stays kept.
- Say what is target and what is built. Mark target as "(target)" in docs.
- No unbuilt thing is described as existing.
- Wording follows [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md): "experiment", "Return", never "automation" or "ROI".
- Open product calls are written down as options, not decided.

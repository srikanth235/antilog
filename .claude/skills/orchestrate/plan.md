# Plan: Antilog cloud on AWS (first release)

(target) Nothing here is built. This plan orders the sessions that build the first release ([rulings](../product/rulings.md), "First release") in `us-east-1`. Each session gets a brief ([brief.md](brief.md)) and reports in the format in [land/report.md](../land/report.md).

## Gates (owner)

Real cloud work and new dependencies wait for these ([owner.md](owner.md), [land/real-cloud.md](../land/real-cloud.md)):

| Gate | Needed before |
| --- | --- |
| A dedicated AWS test account in `us-east-1`, a budget alert and a cap per run | the spike, track E's deploys |
| Bedrock model access to Claude and AgentCore enabled in that account | the spike |
| A GitHub App for test repos, plus Google and GitHub OAuth clients | track C's real sign-in, the spike's pull request |
| Spike scope: AgentCore only now, the other four platforms later (recommended), or all five as written | the spike |

## Order

```
0 Foundation ──► 1 Spike (real cloud, AgentCore)
      │
      └──► tracks A–E in parallel, on fakes ──► F Web app ──► 2 Integration on AWS ──► 3 First-client gates
```

### 0. Foundation (one session, first)

Lays the shared interface so the tracks can run in parallel.

- The Cargo workspace: `antilog-types` (API types, ids, states), `antilog-core`, `antilog-server`, `antilog-agentcore`, `antilog` (CLI and MCP). `forbid(unsafe_code)` everywhere.
- The traits, each with a test double: `Store`, `Platform` (agent and judge sandbox), `Timer`, `Clock`, `Signer`, `Secrets`.
- Postgres migrations with `sqlx`, OpenAPI generation, the TypeScript client generator.
- CI: fmt, clippy, nextest, `cargo deny`, the Postgres tests. Update the Checks section of `AGENTS.md`.
- Skills: [stack/rust](../stack/rust/antilog.md), [stack/postgres](../stack/postgres/antilog.md), [land](../land/SKILL.md).

### 1. Spike (one session, real cloud, after the gates)

One tiny Speed experiment end to end on AgentCore: a session, a trial pushed to its branch, a judge sandbox holding a credential no agent sees, one benchmark run many times. It answers the questions in [rulings](../product/rulings.md), "Still unproven: the spike": repeats, the first example that fits, the agent image, usage timing, quotas, one branch per session, scale to zero, cost of one experiment and pair interference. Each answer goes into `rulings.md`, and the tracks adjust.

### Tracks A–E (parallel, fakes only)

| Track | Area skill | Delivers |
| --- | --- | --- |
| A. Ledger | [change/ledger-table](../change/ledger-table.md) | Tables and the Postgres `Store`: organizations, projects, members, experiments, sessions, trials, measurements, receipts, budgets, approvals, the audit log, value models |
| B. Judge | [judge](../judge/SKILL.md) | Seal and seal checks; measure with paired runs; the bar (lower bound above zero at 1 − 0.05/k); the holdout; Kept; ed25519 receipts; deliver; Return and the Report |
| C. Organization | [organization](../organization/SKILL.md) | Sign-in with GitHub and Google; roles; approvals and policy; budgets and the monthly cap; the audit log |
| D. Session | [session](../session/SKILL.md) | The agent trait on the fake platform, the event relay (about 10-minute steps), the Antilog tools served to a session and by `antilog mcp`, setup tools, spend |
| E. AWS | [stack/aws](../stack/aws/antilog.md) | The Lambda entry (`lambda_http`, streaming), the EventBridge Scheduler timer, the AgentCore adapter, Secrets Manager, OpenTofu for Lambda, Aurora Serverless v2, the scheduler, S3 and CloudFront, Budgets |

A and B start first; C and D follow once A's tables for their area exist. E builds against the traits and needs the gates only to deploy.

### F. Web app (after A–D's endpoints)

Screens from the boards in `design/mockups/`, on the generated client: sign-in, Projects, the experiment page (chat and cards), experiment detail, Customize (Evaluations only for the first release), Settings. Skills: [change/screen](../change/screen.md), [design](../design/SKILL.md).

### 2. Integration on AWS (one session, real cloud)

The steps in [land/real-cloud.md](../land/real-cloud.md), "Order", 3 to 6, on AgentCore in the test account: tags, the budget stop, no route from a session to the judge sandbox, clean-up; then a full session, from a sealed experiment to a Kept win and its pull request.

### 3. Before the first client

The push scan and the second opinion ([rulings](../product/rulings.md), "First release"). A pilot on a real client repository needs the owner's go-ahead.

## Rules for this plan

- One area per session. A change across areas is split, with the shared types written first in `antilog-types`.
- Mockups first for anything a user sees.
- A new dependency outside [libraries.md](../stack/libraries.md) is a product call.
- When code lands, drop the "(target)" marks it makes true, in the same change.

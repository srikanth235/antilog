# Plan: Antilog cloud on AWS (first release)

> This is the first-release plan and is history in part: agent platforms, the spike, the Timer and the relay are retired by [docs/V0.md](../../../docs/V0.md).

(target) Nothing here is built. This plan orders the sessions that build the first release ([rulings](../product/rulings.md), "First release") in `us-east-1`. Each session gets a brief ([brief.md](brief.md)) and reports in the format in [land/report.md](../land/report.md).

## Gates (owner)

Real cloud work and new dependencies wait for these ([owner.md](owner.md), [land/real-cloud.md](../land/real-cloud.md)):

| Gate | Needed before |
| --- | --- |
| A dedicated AWS test account in `us-east-1`, a budget alert and a cap per run | track E's deploys |
| A GitHub App for test repos, plus Google and GitHub OAuth clients | track C's real sign-in, the pull request |

## Order

```
0 Foundation ──► tracks A–E in parallel, on fakes ──► F Web app ──► 2 Integration on AWS ──► 3 First-client gates
```

### 0. Foundation (one session, first)

Lays the shared interface so the tracks can run in parallel.

- The Cargo workspace: `antilog-types` (API types, ids, states), `antilog-core`, `antilog-server`, `antilog` (CLI and MCP). `forbid(unsafe_code)` everywhere.
- The traits, each with a test double: `Store`, `SessionMachine`, `Clock`, `Signer`, `Secrets`.
- Postgres migrations with `sqlx`, OpenAPI generation, the TypeScript client generator.
- CI: fmt, clippy, nextest, `cargo deny`, the Postgres tests. Update the Checks section of `AGENTS.md`.
- Skills: [stack/rust](../stack/rust/antilog.md), [stack/postgres](../stack/postgres/antilog.md), [land](../land/SKILL.md).

### Tracks A–E (parallel, fakes only)

| Track | Area skill | Delivers |
| --- | --- | --- |
| A. Ledger | [change/ledger-table](../change/ledger-table.md) | Tables and the Postgres `Store`: organizations, projects, members, experiments, sessions, trials, measurements, receipts, budgets, approvals, the audit log, value models |
| B. Judge | [judge](../judge/SKILL.md) | Seal and seal checks; measure with paired runs; the bar (lower bound above zero at 1 − 0.05/k); the holdout; Kept; ed25519 receipts; deliver; Return and the Report |
| C. Organization | [organization](../organization/SKILL.md) | Sign-in with GitHub and Google; roles; approvals and policy; budgets and the monthly cap; the audit log |
| D. Session | [session](../session/SKILL.md) | The Antilog tools served to a session and by `antilog mcp`, setup tools, spend; the runner and the proxy ([docs/V0.md](../../../docs/V0.md)) come next |
| E. AWS | [stack/aws](../stack/aws/antilog.md) | The Lambda entry (`lambda_http`, streaming), Secrets Manager, OpenTofu for Lambda, Aurora Serverless v2, S3 and CloudFront, Budgets |

A and B start first; C and D follow once A's tables for their area exist. E builds against the traits and needs the gates only to deploy.

### F. Web app (after A–D's endpoints)

Screens from the boards in `design/mockups/`, on the generated client: sign-in, Projects, the experiment page (chat and cards), experiment detail, Customize (Evaluations only for the first release), Settings. Skills: [change/screen](../change/screen.md), [design](../design/SKILL.md).

### 2. Integration on AWS (one session, real cloud)

The steps in [land/real-cloud.md](../land/real-cloud.md), "Order", in the test account: the budget stop, clean-up; then a full session, from a sealed experiment to a Kept win and its pull request.

### 3. Before the first client

The push scan and the second opinion ([rulings](../product/rulings.md), "First release"). A pilot on a real client repository needs the owner's go-ahead.

## Rules for this plan

- One area per session. A change across areas is split, with the shared types written first in `antilog-types`.
- Mockups first for anything a user sees.
- A new dependency outside [libraries.md](../stack/libraries.md) is a product call.
- When code lands, drop the "(target)" marks it makes true, in the same change.

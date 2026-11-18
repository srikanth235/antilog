# AGENTS.md

Antilog is a chat and a judge. A person states a goal and talks to a coding agent in a sandbox with repos attached, and every session gets the judge's tools: an agent asks the judge to measure a change, only the judge's numbers count, and only a measured win becomes a pull request. It serves three aims beside trust: cost control (every dollar and token of an experiment is spent well), no tie to one model provider, and Return for the organization. It is organised around four things (a **project** that groups experiments, an **experiment** in it, a **session** inside the experiment and a **measurement**), one Customize page for reusable setup (evaluations, skill packs, connectors, agents, plugins) and three areas: the **session** (the chat and where an agent runs), the **judge** (what "better" means and how a win is decided) and the **organization** floor (members, roles in the organization and per project, sign-in, tokens, approvals, budgets, audit log). Settings are tools: setup happens in the chat as an approved tool call. The target is a server with a web app as the client of its API, one Helm chart on Kubernetes with Postgres, run by us as Antilog cloud on AWS (phase 1) or self-hosted in a client's own AWS, Azure or GCP account (phase 2). Antilog is sold as services: our engineers deliver engagements on it, and client engineers take over. Agents run as sessions on agent platforms (Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service, Google Agent Runtime): the customer's own when self-hosted, ours on Antilog cloud. Target design is marked "(target)".

## This branch starts fresh

`main` holds the agreed design: these skills, the design boards and tokens in `design/` (`mockups/`, `tokens.toml`, `tools/`), the logo in `design/icon/` and the glossary. This branch adds the Foundation: the Cargo workspace in `crates/` (`antilog-types`, `antilog-core` with its traits and fakes, `antilog-server` with the Postgres `Store`, `antilog-agentcore` and `antilog`, the last two empty), `openapi/antilog.json` and the generated client in `web/`. The ledger adds the organization and session tables, with one `Store` trait per area (organization, session, budget, approval), their fakes and the storage suite. The judge in `antilog-core/src/judge/` adds seal, paired measurement, the acceptance function, receipts and value, with a `JudgeStore` trait, its fake and its Postgres tables (`pg/judge.rs`). The organization floor adds, in `antilog-core`, the action layer with its one permission matrix (`organization.rs`), sign-in with a fake provider (`auth.rs`), approvals and policy (`approval.rs`) and budgets and caps (`budget.rs`), and in `antilog-server` the GitHub and OpenID Connect providers (`signin.rs`) and the endpoints (`api/auth.rs`, `api/organization.rs`), with row-level security on the organization's own tables. The session area adds the `Platform` trait (agent and judge sandbox in one, with `FakePlatform`), the relay in `antilog-core/src/session/` (steps of about 10 minutes that save the platform's cursor and book the next wake-up through `Timer`), the tools in `antilog-core/src/tools/` (served to a session by the relay and by `antilog mcp`), spend metering and the budget stop in `spend.rs`, the session endpoints in `antilog-server/src/api/session.rs`, and `antilog mcp` in `crates/antilog`. The wire-up joins them: the endpoints sit behind sign-in and the permission matrix (experiments, sessions, tools, receipts), `antilog-server/src/deps.rs` builds the dependencies (Postgres plus the fakes, chosen by `ANTILOG_PLATFORM=fake`), row-level security covers every table with an organization, and the setup tools and the seal card raise their cards through `approval.rs`. The AWS track adds `EnvSecrets` (`secrets.rs`) and the platform contract suite (`platform_suite.rs`) to the core, the AgentCore `Platform` in `crates/antilog-agentcore` (tested against a stub runtime, never against AWS yet), the Lambda entry and the EventBridge Scheduler `Timer` in `crates/antilog-aws`, the OpenTofu module in `deploy/tofu/aws/` and `.github/workflows/deploy-aws.yml`; nothing has been applied to an account. The web app's API adds `GET /me`, sign-in that needs no organization id (the identity's oldest membership signs in), one list of pending cards (`approval.rs`), the conversation (`session_events`, `GET /sessions/{id}/events`; words and tool names only), spend per experiment by kind and session (`budget.rs`), agents with Auto and environments (`agents.rs`) and evaluation templates (`evaluations.rs`), each with its migration, fake and Postgres store. No other platform adapter and no `watch` yet. Everything else is not built yet.

Every skill describes the target. A path or command that does not exist on this branch yet is marked "(target)".

When code lands on this branch, update the pages that name its paths and commands and drop their "(target)" marks, in the same change.

## Checks

Rust (from the repo root; Docker is needed for the Postgres tests):

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo deny check
cargo sqlx prepare --workspace --check   # DATABASE_URL points at a database with the migrations applied
cargo run -q -p antilog-server -- openapi | diff - openapi/antilog.json
cargo lambda build --release --arm64 -p antilog-aws        # the Lambda binary; CI also invokes /health locally (cargo lambda watch and invoke)
tofu fmt -check -recursive deploy/tofu
tofu -chdir=deploy/tofu/aws init -backend=false && tofu -chdir=deploy/tofu/aws validate
```

A changed type or route: regenerate `openapi/antilog.json` (`cargo run -q -p antilog-server -- openapi > openapi/antilog.json`), then the client (`cd web && pnpm install && pnpm generate:api`). A changed query: `cargo sqlx prepare --workspace` and commit `.sqlx/`. For a design change, edit the light board and run `python3 design/tools/tokens.py`, then `python3 design/tools/tokens.py --check`. Git hooks in `.githooks/` check each commit (Claude Code sessions turn them on at start; elsewhere run `git config core.hooksPath .githooks`), and `.github/workflows/checks.yml` runs the same checks on every push, plus the Rust and web-client jobs.

- Fix the code, not the check. Do not loosen a lint, test or snapshot to get green.

## Skills

Guidance lives in [`.claude/skills/`](.claude/skills), each a short router. Start from the question:

- `product`: what do we owe the customer, and what do we refuse to build?
- `session`: how does the chat work, and where does an agent run? Also skill packs and connectors.
- `judge`: what does better mean, and how is a win decided?
- `organization`: who may approve what, and how is it recorded? Projects and roles per project.
- `design`: what does it look like and say? Boards, light and dark, the macOS app.
- `change`: how does this kind of change travel through the layers?
- `stack`: how do I write this language here (Rust, TypeScript and React, modern HTML, CSS and browser APIs, Swift for macOS, Postgres, Kubernetes and Helm, security review, MCP, Playwright), and which library may I use?
- `land`: how do I prove it and ship it?
- `orchestrate`: how is multi-session work run? Only for a session that directs other sessions.
- `antilog`: how to drive Antilog from the shell.

## Conventions

- **Docs describe current state.** When a change makes a doc wrong, fix the doc in the same change. Target design is marked "(target)".
- **Product calls go to the owner.** When a choice is about what the product should be and the `product` skill does not settle it, ask rather than guess.
- **Mockups first.** Every visual or UX change starts as a board in `design/mockups/`.
- **Child session vs subagent.** A "child session" (or just "session", for implementation work) is a separate Claude Code session started from this one with `create_session`; it has its own container and shows in the owner's session list. A "subagent" runs inside this session and reports only to it. Implementation work goes to child sessions on Sonnet, at most three at once (owner, 18 Nov); the orchestrator runs on Opus. Ask when a request could mean either.
- **Vocabulary** follows [`docs/GLOSSARY.md`](docs/GLOSSARY.md): "experiment" is the noun for what a user creates, in the apps, the CLI, the API, the words catalogue and user-facing docs; it belongs to a project, and a session is one agent's work inside it. Say "skill pack" in docs where "skill" could mean the files in `.claude/skills/`. Retired words are listed at the end of the glossary.

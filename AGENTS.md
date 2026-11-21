# AGENTS.md

Antilog is a chat with a coding agent in a cloud session, as Claude Code's cloud sessions are. A person states a goal in a project and talks to an agent that runs in a machine of its own with repos attached. The agent asks for setup (a repo, a credential, an environment change, more budget) through Antilog's tools, and a person approves each request. Spend is capped by the organization, the project and, optionally, the session. A **project** groups **sessions**, as Claude Code's sidebar groups sessions under a project; a session belongs to one project. The organization floor holds members, roles, sign-in, approvals, budgets and the audit log.

`docs/V0.md` is the owner's design for v0 (cloud sessions only: a runner that polls a queue, one machine per session, OpenCode, one proxy). The skills follow it; where a board still disagrees, the page wins. Delete code that a decision there makes irrelevant in the same change that replaces it. Experiments and the judge are parked, not hidden: their code is deleted and commit `daa445f` keeps it.

The runner, the proxy (`crates/antilog-proxy`) and the session flow are built; the real code host is not. A path or command that does not exist yet is marked "(target)".

## Checks

Rust (from the repo root; Docker is needed for the Postgres tests):

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo deny check
cargo sqlx prepare --workspace --check   # DATABASE_URL points at a database with the migrations applied
cargo run -q -p antilog-server -- openapi | diff - openapi/antilog.json
```

A changed route or type: regenerate `openapi/antilog.json`, then `pnpm generate:api` in `web/`. A changed query: `cargo sqlx prepare --workspace`. A design change: `python3 design/tools/tokens.py`, then `--check`. Hooks in `.githooks/` and CI run the same checks. Fix the code, not the check.

## Skills

Guidance lives in [`.claude/skills/`](.claude/skills), each a short router:

- `product`: what we owe the customer; what we refuse to build.
- `session`: the chat, where an agent runs, skill packs, connectors.
- `judge`: parked with the experiments; one page that points at `daa445f`.
- `organization`: who may approve what; how it is recorded; budgets.
- `design`: boards, tokens, wording.
- `change`: how a kind of change travels through the layers.
- `stack`: how to write each language here; which library may be used.
- `land`: how to prove a change and ship it.
- `orchestrate`: only for a session that directs other sessions.
- `antilog`: driving Antilog from the shell (`antilog runner`) and over HTTP.

## Conventions

- **Docs describe current state.** Fix a doc in the change that makes it wrong.
- **Product calls go to the owner.** Ask when `product` does not settle it.
- **Mockups first.** Every visual or UX change starts as a board in `design/mockups/`.
- **Child session vs subagent.** A child session comes from `create_session`; a subagent runs inside this one. Implementation goes to child sessions on Sonnet, at most three at once.
- **Vocabulary** follows [`docs/GLOSSARY.md`](docs/GLOSSARY.md): "session" is what a user starts and a project groups; say "skill pack" where "skill" could mean the files in `.claude/skills/`.

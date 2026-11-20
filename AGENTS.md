# AGENTS.md

Antilog is a chat and a judge. A person states a goal and talks to a coding agent in a session; every session gets the judge's tools. An agent asks the judge to measure a change, only the judge's numbers count, and only a measured win becomes a pull request. A **project** groups **experiments**; an experiment holds **sessions** and the judge's **measurements**. The organization floor holds members, roles, sign-in, approvals, budgets and the audit log.

`docs/V0.md` is the owner's design for v0 (a runner that polls a queue, one machine per session, OpenCode, one proxy). It wins over any skill, ruling or board until the skills are rewritten to match it. Delete code that a decision there makes irrelevant in the same change that replaces it.

The runner and the proxy (`crates/antilog-proxy`) are built; the real code host is not. A path or command that does not exist yet is marked "(target)".

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
- `judge`: what "better" means; how a win is decided.
- `organization`: who may approve what; how it is recorded.
- `design`: boards, tokens, wording.
- `change`: how a kind of change travels through the layers.
- `stack`: how to write each language here; which library may be used.
- `land`: how to prove a change and ship it.
- `orchestrate`: only for a session that directs other sessions.
- `antilog`: driving Antilog from the shell.

## Conventions

- **Docs describe current state.** Fix a doc in the change that makes it wrong.
- **Product calls go to the owner.** Ask when `product` does not settle it.
- **Mockups first.** Every visual or UX change starts as a board in `design/mockups/`.
- **Child session vs subagent.** A child session comes from `create_session`; a subagent runs inside this one. Implementation goes to child sessions on Sonnet, at most three at once.
- **Vocabulary** follows [`docs/GLOSSARY.md`](docs/GLOSSARY.md): "experiment" is what a user creates; say "skill pack" where "skill" could mean the files in `.claude/skills/`.

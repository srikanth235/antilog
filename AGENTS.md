# AGENTS.md

Ratchet points coding agents at one measurable goal and lets only the changes that beat a trusted benchmark move forward. It is organised around three basics: **optimization** (what "better" means), **context** (what a proposer may know) and **infrastructure** (where work happens), over a shared floor, the **organization** (members, roles, sign-in, tokens, audit log). The target is a server with a web app as the client of its API, self-hosted (Docker Compose) or run by us as Ratchet cloud (on Cloudflare), with work and data staying in the customer's infrastructure unless an owner chooses Ratchet-managed sandboxes. Target design is marked "(target)".

## This branch starts fresh

`main` holds the agreed design and no product code: these skills, the design boards and tokens in `design/` (`mockups/`, `tokens.toml`, `tools/`), the logo in `design/icon/` and the glossary. Nothing is built here yet.

An earlier Rust daemon and CLI live on the branch `claude/peaceful-lamport-leak6p`. They are reference, not a base to build on. Every skill describes the target. A path or command that does not exist on this branch yet is marked "(target)". What the earlier code did, and what it taught, is in the `today.md` of `optimization`, `context`, `infra`, `organization`, `change`, `land` and `ratchet`.

When code lands on this branch, update the pages that name its paths and commands and drop their "(target)" marks, in the same change.

## Checks

There is no code on this branch, so there is no build or test to run yet. For a design change, edit the light board and run `python3 design/tools/tokens.py`, then `python3 design/tools/tokens.py --check`. Git hooks in `.githooks/` check each commit (Claude Code sessions turn them on at start; elsewhere run `git config core.hooksPath .githooks`), and `.github/workflows/checks.yml` runs the same checks on every push. Add the real checks here with the first code.

- Fix the code, not the check. Do not loosen a lint, test or snapshot to get green.

## Skills

Guidance lives in [`.claude/skills/`](.claude/skills), each a short router. Start from the question:

- `product`: what do we owe the customer, and what do we refuse to build?
- `optimization`: what does better mean, and how is a win decided?
- `context`: what may a proposer know?
- `infra`: where does work happen, and what may it cost?
- `organization`: who may do what, and how is it recorded?
- `design`: what does it look like and say? Boards, light and dark, the macOS app.
- `change`: how does this kind of change travel through the layers?
- `stack`: how do I write this language here (Rust, TypeScript and React, modern HTML, CSS and browser APIs, Swift for macOS, Postgres, OpenTofu, Packer, Helm), and which library may I use?
- `land`: how do I prove it and ship it?
- `orchestrate`: how is multi-session work run? Only for a session that directs other sessions.
- `ratchet`: how to drive Ratchet from the shell.

## Conventions

- **Docs describe current state.** When a change makes a doc wrong, fix the doc in the same change. Target design is marked "(target)".
- **Product calls go to the owner.** When a choice is about what the product should be and the `product` skill does not settle it, ask rather than guess.
- **Mockups first.** Every visual or UX change starts as a board in `design/mockups/`.
- **Vocabulary** follows [`docs/GLOSSARY.md`](docs/GLOSSARY.md): "experiment" is the noun for the thing a user creates, in the apps, the CLI, the API, the words catalogue and user-facing docs; "optimization" only names the basic and its skill; never "automation"; "Return", never "ROI".

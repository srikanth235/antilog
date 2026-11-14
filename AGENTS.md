# AGENTS.md

Ratchet points coding agents at one measurable goal and lets only the changes that beat a trusted benchmark move forward. It is organised around three basics: **optimization** (what "better" means), **context** (what a proposer may know) and **infrastructure** (where work happens), over a shared floor, the **organization** (members, roles, sign-in, tokens, audit log). The target is a server with a web app as the client of its API, self-hosted (Docker Compose) or run by us as Ratchet cloud, with work and data staying in the customer's infrastructure. Target design is marked "(target)".

## This branch starts fresh

`main` holds the agreed design and no product code: these skills, the web client prototype in `design/web/`, the logo in `design/icon/` and the glossary. Nothing is built here yet.

An earlier Rust daemon and CLI live on the branch `claude/peaceful-lamport-leak6p`. They are reference, not a base to build on. Where a skill names a path under `crates/`, `api/`, `infra/`, `scripts/` or `docs/` that is not on this branch, or quotes a `cargo` or `pnpm` command, it describes that earlier code:

- the `today.md` file in `optimization`, `context`, `infra` and `organization`;
- `stack`, `land` and `change`, which describe how that code is written, checked and changed;
- `ratchet`, which drives that code's CLI.

Use those for what was learned, such as the gaps each `today.md` lists. When code lands on this branch, rewrite them to describe it, in the same change.

## Checks

There is no code on this branch, so there is no build or test to run yet. For a change to the prototype, edit the parts in `design/web/src/` and run `bash design/web/src/assemble.sh`. Add the real checks here with the first code.

- Fix the code, not the check. Do not loosen a lint, test or snapshot to get green.

## Skills

Guidance lives in [`.claude/skills/`](.claude/skills), each a short router. Start from the question:

- `product`: what do we owe the customer, and what do we refuse to build?
- `optimization`: what does better mean, and how is a win decided?
- `context`: what may a proposer know?
- `infra`: where does work happen, and what may it cost?
- `organization`: who may do what, and how is it recorded?
- `design`: what does it look like and say?
- `change`: how does this kind of change travel through the layers?
- `stack`: how do I write this language here, and which library may I use?
- `land`: how do I prove it and ship it?
- `orchestrate`: how is multi-session work run? Only for a session that directs other sessions.
- `ratchet`: how to drive Ratchet from the shell.

## Conventions

- **Docs describe current state.** When a change makes a doc wrong, fix the doc in the same change. Target design is marked "(target)".
- **Product calls go to the owner.** When a choice is about what the product should be and the `product` skill does not settle it, ask rather than guess.
- **Mockups first.** Every visual or UX change starts in `design/web/`.
- **Vocabulary** follows [`docs/GLOSSARY.md`](docs/GLOSSARY.md): "optimization", never "automation"; "Return", never "ROI". The web client (target) says "experiment"; its words are in the glossary's target section.

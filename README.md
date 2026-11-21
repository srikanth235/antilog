# Antilog

Antilog is a chat with a coding agent in a cloud session, as Claude Code's cloud sessions are. A person states a goal in a project and talks to an agent that runs in a machine of its own with repos attached. The agent asks for setup (a repo, a credential, an environment change, more budget) through Antilog's tools, and a person approves each request. Spend is capped by the organization, the project and, optionally, the session. A project groups sessions.

Experiments and the judge (measuring a change, wins, receipts) are parked: the code is deleted from this branch and commit `daa445f` keeps it.

- [`AGENTS.md`](AGENTS.md): what Antilog is, how this branch is organised, and the index of the skills.
- [`docs/V0.md`](docs/V0.md): the owner's v0 design: cloud sessions only.
- [`.claude/skills/`](.claude/skills): the product's promises, the session and where an agent runs, the organization floor (projects, members, roles, approvals, budgets), the design rules, and how work is done.
- [`design/mockups/`](design/mockups/): the design boards, light and dark. Values are in `design/tokens.toml`; see [`design/README.md`](design/README.md).
- [`design/icon/`](design/icon): the logo.
- [`docs/GLOSSARY.md`](docs/GLOSSARY.md): the words.


## Licence

[FSL-1.1-ALv2](LICENSE.md), Copyright 2026 Duaility, LLC. Each release becomes Apache-2.0 two years after it is published.

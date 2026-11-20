---
name: stack
description: >-
  How do I write this language here, and which library may I use? Use for Rust, TypeScript and React, modern HTML, CSS and browser APIs (dialogs, popovers, forms, tables, theming, accessibility, performance, web security), Swift and SwiftUI for the macOS app, Postgres and SQL, security review (Rust, dependencies, API design, GitHub Actions), the MCP server (`antilog mcp` and the Antilog tools), driving a browser with Playwright, or adding a dependency: the repo's own constraints, the check command, general style references, and the chosen target libraries.
---

# Stack

Read the repo page for the file type first: it holds this repo's constraints, its common errors and its check. `general/` beside it holds digests of well-known sources, each starting with a `Source:` line; `licenses/` holds the upstream licence text beside what it covers. Which checks to run before you call a change done, and how to report them: [land](../land/SKILL.md).

**The repo page wins over `general/`.** If a digest says one thing and `antilog.md` says another, follow `antilog.md`. A digest is advice from outside; the repo page is a decision.

| File type | Read | Check |
| --- | --- | --- |
| `.rs`, `Cargo.toml` | [rust/antilog.md](rust/antilog.md) | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run`, `cargo deny check` |
| `.ts`, `.tsx`, `.css` in the web client | [typescript/antilog.md](typescript/antilog.md) | `pnpm build && pnpm typecheck && pnpm lint && pnpm test`, and `pnpm e2e` |
| HTML, CSS and browser APIs, in the web client or a board | [web/antilog.md](web/antilog.md), then the guide for the job in [web/general/INDEX.md](web/general/INDEX.md); for a Base UI component also [web/base-ui.md](web/base-ui.md) | the check of the file type; `python3 .claude/skills/stack/web/sync.py --check` after a sync |
| `.swift`, the macOS app (boards owed) | [swift/antilog.md](swift/antilog.md) | `xcodebuild ... test` on macOS (target); report it unrun elsewhere |
| SQL, migrations, `sqlx` queries, the job queue | [postgres/antilog.md](postgres/antilog.md) | `cargo sqlx prepare --workspace --check` and the storage tests |
| A security review: a Rust change at the trust boundary, a new dependency, an API or config design, `.github/workflows/`, a property test | [security/antilog.md](security/antilog.md), then the skill for the job in [security/general/INDEX.md](security/general/INDEX.md) | the check of the language; `python3 .claude/skills/stack/security/sync.py --check` after a sync |
| The MCP server (`antilog mcp`), the Antilog tools it serves and their descriptions | [mcp/antilog.md](mcp/antilog.md), then [mcp/general/INDEX.md](mcp/general/INDEX.md) | the Rust check; `python3 .claude/skills/stack/mcp/sync.py --check` after a sync |
| Driving the web app or a board in a browser while you work | [playwright/antilog.md](playwright/antilog.md), then [playwright/general/INDEX.md](playwright/general/INDEX.md) | `python3 .claude/skills/stack/playwright/sync.py --check` after a sync |
| Adding or changing a dependency | [libraries.md](libraries.md) | the check of the language |

## Every language

These hold in Rust, TypeScript, Swift and SQL alike, and win over any `general/` digest.

- **Almost no comments.** A comment that explains what the code does, or why it looks odd, invites the next reader to work around the code instead of fixing it. Make the code say it: a name, a type, a smaller function, a test. A comment is allowed only where nothing else can carry the fact: a `SAFETY:` note, a `reason = "..."` on an `#[expect]`, a link to an outside spec or bug the code must follow, why a `Result` is deliberately dropped, or a public API's doc comment that states its contract. No history, no restating the code, no commented-out code, no `TODO` without an issue link. A narrating comment is a finding.
- **APIs hide complexity from their callers, agents first.** Most callers will be agents, which read a signature and a one-line description and act. Give each module, endpoint and tool a small surface that does a whole job: one call that takes intent and returns a result or an error that says what to do next, not a sequence of low-level steps the caller must order, retry or clean up. Defaults that are right, no flag the caller must know to set, invariants enforced inside rather than documented for the caller. When a caller would need to read the implementation to use it safely, the API is wrong. This applies to the Rust core's traits, the HTTP API, the Antilog tools ([mcp/antilog.md](mcp/antilog.md)) and the CLI.

Nothing here is built on this branch yet: every check is (target) until code lands, and each page then gets its exact commands. Pages describe how the code is written.

## Folders

| Folder | Holds |
| --- | --- |
| `rust/` | `antilog.md` (the core, the server, the adapters and the cloud-agnostic rule); `general/` Rust style, API, patterns, Microsoft and Google guides, and async services (tokio, axum, tower, sqlx); `licenses/` |
| `typescript/` | `antilog.md`; `general/` TypeScript practices, type patterns, Vitest, React best practices, composition patterns; `licenses/` |
| `web/` | `antilog.md`; `base-ui.md`; `general/` 111 Modern Web Guidance guides with an index, copied by `sync.py`; `licenses/` |
| `swift/` | `antilog.md`; `general/` Swift, SwiftUI views, data, accessibility, diagnostics, performance, macOS; `licenses/` |
| `postgres/` | `antilog.md` (the only store, its schema and the job queue); `general/` Postgres best practices; `licenses/` |
| `security/` | `antilog.md` (when to use which review skill, and Antilog's trust rules as the bar); `general/` Trail of Bits (Rust review, supply chain, sharp edges, property-based testing) and Sentry (GitHub Actions security), copied by `sync.py`; `licenses/` |
| `mcp/` | `antilog.md` (our Rust server, the Antilog tools it serves and its rules); `general/` Anthropic's MCP builder skill, copied by `sync.py`; `licenses/` |
| `playwright/` | `antilog.md` (the CLI for working, `@playwright/test` for the suite); `general/` Microsoft's `playwright-cli` skill, copied by `sync.py`; `licenses/` |

The chosen target libraries and tool versions are in [libraries.md](libraries.md). Do not add a dependency for convenience: a new one is a product call.

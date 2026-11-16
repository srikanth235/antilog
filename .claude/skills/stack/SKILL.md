---
name: stack
description: >-
  How do I write this language here, and which library may I use? Use for Rust, TypeScript and React, modern HTML, CSS and browser APIs (dialogs, popovers, forms, tables, theming, accessibility, performance, web security), Swift and SwiftUI for the macOS app, Postgres and SQL, Cloudflare (Workers, Durable Objects, the cf CLI) for Ratchet cloud, OpenTofu, Packer, Helm and Kubernetes (earlier code only), security review (Rust, dependencies, API design, GitHub Actions), the MCP server, driving a browser with Playwright, or adding a dependency: the repo's own constraints, the check command, general style references, and the chosen target libraries.
---

# Stack

Read the repo page for the file type first: it holds this repo's constraints, its common errors and its check. `general/` beside it holds digests of well-known sources, each starting with a `Source:` line; `licenses/` holds the upstream licence text beside what it covers. Which checks to run before you call a change done, and how to report them: [land](../land/SKILL.md).

**The repo page wins over `general/`.** If a digest says one thing and `ratchet.md` says another, follow `ratchet.md`. A digest is advice from outside; the repo page is a decision.

| File type | Read | Check |
| --- | --- | --- |
| `.rs`, `Cargo.toml` | [rust/ratchet.md](rust/ratchet.md) | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run`, `cargo deny check`, `cargo build -p ratchet-core --target wasm32-unknown-unknown` (target) |
| `.ts`, `.tsx`, `.css` in the web client | [typescript/ratchet.md](typescript/ratchet.md) | `pnpm build && pnpm typecheck && pnpm test` (target) |
| HTML, CSS and browser APIs, in the web client or a board | [web/ratchet.md](web/ratchet.md), then the guide for the job in [web/general/INDEX.md](web/general/INDEX.md); for a Base UI component also [web/base-ui.md](web/base-ui.md) | the check of the file type; `python3 .claude/skills/stack/web/sync.py --check` after a sync |
| `.swift`, the macOS app | [swift/ratchet.md](swift/ratchet.md) | `xcodebuild ... test` on macOS (target); report it unrun elsewhere |
| SQL, migrations (Postgres and SQLite), `sqlx` queries, the job queue | [postgres/ratchet.md](postgres/ratchet.md) | `cargo sqlx prepare --check` and the storage tests against both stores (target) |
| The cloud shell `ratchet-cloud`, `cloudflare.config.ts`, Durable Objects, D1, the Secrets Store | [cloudflare/ratchet.md](cloudflare/ratchet.md), then the skill for the job in [cloudflare/general/INDEX.md](cloudflare/general/INDEX.md) | the Rust check; `python3 .claude/skills/stack/cloudflare/sync.py --check` after a sync |
| A security review: a Rust change at the trust boundary, a new dependency, an API or config design, `.github/workflows/`, a property test | [security/ratchet.md](security/ratchet.md), then the skill for the job in [security/general/INDEX.md](security/general/INDEX.md) | the check of the language; `python3 .claude/skills/stack/security/sync.py --check` after a sync |
| The MCP server (`ratchet mcp`), its tools and their descriptions | [mcp/ratchet.md](mcp/ratchet.md), then [mcp/general/INDEX.md](mcp/general/INDEX.md) | the Rust check; `python3 .claude/skills/stack/mcp/sync.py --check` after a sync |
| Driving the web app or a board in a browser while you work | [playwright/ratchet.md](playwright/ratchet.md), then [playwright/general/INDEX.md](playwright/general/INDEX.md) | `python3 .claude/skills/stack/playwright/sync.py --check` after a sync |
| `.tf`, `.tftest.hcl`, `.pkr.hcl`, Helm charts, Kubernetes manifests (earlier code only; none serves a sandbox in the target) | [infra/ratchet.md](infra/ratchet.md) | `tofu fmt -check`, `tofu validate`, `tofu test`, `packer validate`, `helm lint` (target) |
| Adding or changing a dependency | [libraries.md](libraries.md) | the check of the language |

Nothing here is built on this branch yet: every check is (target) until code lands, and each page then gets its exact commands. Pages describe how the code is written; the earlier daemon on `claude/peaceful-lamport-leak6p` is reference only.

## Folders

| Folder | Holds |
| --- | --- |
| `rust/` | `ratchet.md` (the core, the two shells and the portability rule); `general/` Rust style, API, patterns, Microsoft and Google guides, and async services (tokio, axum, tower, sqlx); `licenses/` |
| `typescript/` | `ratchet.md`; `general/` TypeScript practices, type patterns, Vitest, React best practices, composition patterns; `licenses/` |
| `web/` | `ratchet.md`; `base-ui.md`; `general/` 111 Modern Web Guidance guides with an index, copied by `sync.py`; `licenses/` |
| `swift/` | `ratchet.md`; `general/` Swift, SwiftUI views, data, accessibility, diagnostics, performance, macOS; `licenses/` |
| `postgres/` | `ratchet.md` (Postgres self-hosted, the shared schema, SQLite in the cloud); `general/` Postgres best practices, SQLite in a Durable Object; `licenses/` |
| `cloudflare/` | `ratchet.md` (what runs on Cloudflare and the rules that win); `general/` Cloudflare's own skills for Durable Objects, Workers, the `cf` CLI, Wrangler and Sandboxes (Sandboxes are not used by the target), with product references, copied by `sync.py`; `licenses/` |
| `security/` | `ratchet.md` (when to use which review skill, and Ratchet's trust rules as the bar); `general/` Trail of Bits (Rust review, supply chain, sharp edges, property-based testing) and Sentry (GitHub Actions security), copied by `sync.py`; `licenses/` |
| `mcp/` | `ratchet.md` (our Rust server and its rules); `general/` Anthropic's MCP builder skill, copied by `sync.py`; `licenses/` |
| `playwright/` | `ratchet.md` (the CLI for working, `@playwright/test` for the suite); `general/` Microsoft's `playwright-cli` skill, copied by `sync.py`; `licenses/` |
| `infra/` | `ratchet.md` (what ships the target, and the earlier OpenTofu, Packer and Helm notes); `general/` Terraform (for OpenTofu), Packer, Helm and Kubernetes; `licenses/` |

The chosen target libraries and tool versions are in [libraries.md](libraries.md). Do not add a dependency for convenience: a new one is a product call.

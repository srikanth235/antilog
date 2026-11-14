---
name: stack
description: >-
  How do I write this language here, and which library may I use? Use for Rust, TypeScript and React, Swift and SwiftUI for the macOS app, Postgres and SQL, OpenTofu, Packer, Helm and Kubernetes, or adding a dependency: the repo's own constraints, the check command, general style references, and the chosen target libraries.
---

# Stack

Read the repo page for the file type first: it holds this repo's constraints, its common errors and its check. `general/` beside it holds digests of well-known sources, each starting with a `Source:` line; `licenses/` holds the upstream licence text beside what it covers. Which checks to run before you call a change done, and how to report them: [land](../land/SKILL.md).

**The repo page wins over `general/`.** If a digest says one thing and `ratchet.md` says another, follow `ratchet.md`. A digest is advice from outside; the repo page is a decision.

| File type | Read | Check |
| --- | --- | --- |
| `.rs`, `Cargo.toml` | [rust/ratchet.md](rust/ratchet.md) | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run`, `cargo deny check` (target) |
| `.ts`, `.tsx`, `.css` in the web client | [typescript/ratchet.md](typescript/ratchet.md) | `pnpm build && pnpm typecheck && pnpm test` (target) |
| `.swift`, the macOS app | [swift/ratchet.md](swift/ratchet.md) | `xcodebuild ... test` on macOS (target); report it unrun elsewhere |
| SQL, migrations, `sqlx` queries, the job queue | [postgres/ratchet.md](postgres/ratchet.md) | `cargo sqlx prepare --check` and the database tests (target) |
| `.tf`, `.tftest.hcl`, `.pkr.hcl`, Helm charts, Kubernetes manifests | [infra/ratchet.md](infra/ratchet.md) | `tofu fmt -check`, `tofu validate`, `tofu test`, `packer validate`, `helm lint` (target) |
| Adding or changing a dependency | [libraries.md](libraries.md) | the check of the language |

Nothing here is built on this branch yet: every check is (target) until code lands, and each page then gets its exact commands. Pages describe how the code is written; the earlier daemon on `claude/peaceful-lamport-leak6p` is reference only.

## Folders

| Folder | Holds |
| --- | --- |
| `rust/` | `ratchet.md`; `general/` Rust style, API, patterns, Microsoft and Google guides, and async services (tokio, axum, tower, sqlx); `licenses/` |
| `typescript/` | `ratchet.md`; `general/` TypeScript practices, type patterns, Vitest, React best practices; `licenses/` |
| `swift/` | `ratchet.md`; `general/` Swift, SwiftUI views, data, accessibility, diagnostics, performance, macOS; `licenses/` |
| `postgres/` | `ratchet.md`; `general/` Postgres best practices; `licenses/` |
| `infra/` | `ratchet.md`; `general/` Terraform (for OpenTofu) and Packer; `licenses/` |

The chosen target libraries and tool versions are in [libraries.md](libraries.md). Do not add a dependency for convenience: a new one is a product call.

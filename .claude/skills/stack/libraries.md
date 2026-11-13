# Libraries

Do not add a dependency for convenience. A new one is a product call: ask the owner. Prefer what is already here.

## In use today

Read from the crate manifests under `crates/*/Cargo.toml`. The workspace manifest declares no shared dependency table, so check the crate you change.

| Library | What it is for |
| --- | --- |
| `tokio`, `tokio-util`, `tokio-stream`, `futures` | Async runtime, cancellation, streams |
| `async-trait` | Async methods on the port traits |
| `serde`, `serde_json`, `serde_norway` | JSON and YAML (the YAML crate reads `ratchet.yaml`) |
| `schemars` | JSON schema for config and tool arguments |
| `utoipa`, `utoipa-axum` | OpenAPI generated from Rust types |
| `axum` | The daemon's HTTP and server-sent events (feature `server`) |
| `hyper`, `hyper-util`, `http-body-util`, `tower` | HTTP client over the unix socket, and test calls |
| `reqwest` (rustls) | HTTPS to GitHub and the engine's outside calls |
| `clap` | Command line of `ratchet` and `ratchetd` |
| `sqlx` (SQLite) | The ledger and its migrations |
| `aws-config` and the `aws-sdk-*` crates (costexplorer, ec2, iam, pricing, s3, ssm, sts) | The AWS sandbox, prices, cost and artifacts |
| `thiserror`, `anyhow` | Errors in libraries and in binaries |
| `tracing`, `tracing-subscriber` | Logging |
| `uuid`, `rand`, `sha1`, `sha2`, `base64` | Ids, randomness, hashes, encoding |
| `regex`, `url`, `percent-encoding`, `indexmap`, `include_dir` | Parsing, ordered maps, embedded templates |
| `nix`, `libc` | Unix users, processes and signals (bench launcher, daemon) |
| `tempfile` | Temporary working trees and files |
| `security-framework` (macOS only) | The Keychain for secrets |
| `insta`, `proptest`, `aws-smithy-mocks` | Snapshot, property and AWS mock tests |

TypeScript in use: `typescript`, `vitest`, `yaml` (see [typescript.md](typescript.md)).

## Chosen for the target

(target) None of these is in the repository yet.

| Area | Libraries |
| --- | --- |
| Server | `tower-http`, `rust-embed`, `sqlx` with Postgres, `openidconnect`, `tower-sessions`, `jsonwebtoken`, `aws-lc-rs`, `lettre`, `opentelemetry` with `tracing-opentelemetry`, `metrics`, `cargo-deny` |
| Web app | React, TypeScript, Vite, pnpm, `openapi-typescript`, `openapi-fetch`, TanStack Query and Router, Radix primitives, CSS modules, Vitest, Playwright, Biome (see [typescript.md](typescript.md)) |
| Delivery | Docker with `cargo-chef`, Helm, Compose, `cosign`, `syft` |

Rules for any addition: self-hosting must keep working (no call to a service the administrator did not configure), the licence must suit a product we ship, and the crate must be maintained. A new outside system also needs a port with a real and a mock implementation; see [rust.md](rust.md).

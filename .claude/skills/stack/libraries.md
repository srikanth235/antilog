# Libraries

Do not add a dependency for convenience. A new one is a product call: ask the owner. Prefer what is already here. Before proposing one, review it with the supply-chain skill ([security/ratchet.md](security/ratchet.md)) and run `cargo deny check`; the review goes in the proposal.

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

TypeScript in use: `typescript`, `vitest`, `yaml` (see [typescript.md](typescript/ratchet.md)).

## Chosen for the target

(target) None of this is in the repository yet. Versions are the latest stable releases on 2026-11-14, read from crates.io, npm and each project's release feed. Pin these when the code lands; raise them on purpose, never by accident.

**Shape.** One Cargo workspace builds one core and two shells, with the runner and the CLI:

- `ratchet-core`: all logic and the API routes as an axum `Router`, behind one `Store` trait. It never imports tokio's runtime, `sqlx` or a Cloudflare crate, and builds for `wasm32-unknown-unknown`. It never connects to data.
- `ratchet-server`: the self-hosted shell around the core. The API, the embedded web app, Postgres. The launcher is built in.
- `ratchet-cloud`: the Ratchet cloud shell around the core, a Cloudflare Worker with one Durable Object (SQLite) per organization.
- `ratchet-runner`: the launcher for Ratchet cloud, the same code, installed in the customer's infrastructure, or run by us for Ratchet-managed organizations. It starts and removes sandboxes, mints scoped credentials and runs the egress proxy. It connects out over HTTPS: it pulls jobs and the server streams updates to it. No inbound port, no gRPC.
- `ratchet`: the CLI and the MCP server, built on the API's types.

Start fresh. Copy only proven pieces from the earlier code on `claude/peaceful-lamport-leak6p` (the AWS sandbox, the clean-up checks, the ledger tests); do not rebase on it.

**Toolchain**

| Tool | Version |
| --- | --- |
| Rust (stable) | 1.100.0 |
| Postgres | 18.7 |
| `wasm32-unknown-unknown` target (core build check, cloud shell) | the stable toolchain's |
| Node.js (LTS, web app build only) | 26.13.1 |
| pnpm | 12.18.0 |

**Rust crates**

| Need | Crate | Version |
| --- | --- | --- |
| Async runtime | `tokio` | 1.54.1 |
| HTTP server, live events | `axum` | 0.8.9 |
| Middleware | `tower` / `tower-http` | 0.5.3 / 0.7.1 |
| HTTP | `hyper` | 1.12.0 |
| TLS | `rustls` | 0.23.46 |
| HTTP client | `reqwest` | 0.13.5 |
| Serialization | `serde` / `serde_json` | 1.0.229 / 1.0.154 |
| Postgres, migrations (self-hosted store) | `sqlx` (+ `sqlx-cli`) | 0.9.0 |
| Cloud shell (Cloudflare Worker, runs the axum router with the `http` feature) | `worker` (workers-rs) | 0.8.7 |
| OpenAPI from Rust types | `utoipa` / `utoipa-axum` | 6.0.0 / 0.3.0 |
| Sign-in | `openidconnect` | 4.1.0 |
| Sessions | `tower-sessions` / `tower-sessions-sqlx-store` | 0.15.0 / 0.16.0 |
| Crypto, receipt signing (ed25519), token hashes | `aws-lc-rs` | 1.19.0 |
| Embedded web app | `rust-embed` | 8.14.0 |
| AWS | `aws-config` | 1.13.2 |
| | `aws-sdk-ec2` / `aws-sdk-s3` / `aws-sdk-sts` | 1.280.0 / 1.159.0 / 1.126.0 |
| | `aws-sdk-dynamodb` / `aws-sdk-secretsmanager` | 1.139.0 / 1.128.0 |
| GitHub | `octocrab` | 0.54.9 |
| Postgres connector | `tokio-postgres` | 0.7.18 |
| Egress proxy (TLS interception) | `hudsucker` / `rcgen` | 0.25.0 / 0.14.10 |
| MCP server | `rmcp` | 3.10.0 |
| Command line | `clap` | 4.6.7 |
| Ids, time | `uuid` / `jiff` | 1.29.2 / 0.2.43 |
| Errors | `thiserror` / `anyhow` | 2.0.21 / 1.0.104 |
| Logs | `tracing` / `tracing-subscriber` | 0.1.44 / 0.3.23 |
| Telemetry | `opentelemetry` / `opentelemetry-otlp` / `tracing-opentelemetry` | 0.33.1 / 0.33.1 / 0.34.0 |
| Tests | `insta` / `proptest` | 1.49.0 / 1.12.0 |
| Postgres in tests | `testcontainers` / `testcontainers-modules` | 0.28.0 / 0.15.0 |
| Test runner, licences | `cargo-nextest` / `cargo-deny` | 0.9.152 / 0.20.2 |

Snowflake, Databricks and BigQuery have no maintained Rust client we would ship; their connectors call each vendor's REST API through `reqwest`. Every connector sits behind one `Connector` trait, so its vocabulary stops there, and each knows how to mint a scoped, read-only, holdout-excluding credential. Sandbox backends sit behind one trait: Kubernetes first, then VMs on AWS, GCP and Azure. Provider crates stay inside each adapter.

**Cloud adapters**

| Need | Crate | Version |
| --- | --- | --- |
| Kubernetes sandboxes | `kube` | 5.1.0 |
| Kubernetes types | `k8s-openapi` | 0.29.0 |
| Object storage (S3, GCS, Azure Blob) | `object_store` | 0.14.2 |
| GCP compute | `google-cloud-compute-v1` | 4.2.0 |
| GCP secrets | `google-cloud-secretmanager-v1` | 2.0.0 |
| GCP auth | `google-cloud-auth` | 1.19.0 |
| Azure core | `azure_core` | 1.1.0 |
| Azure sign-in | `azure_identity` | 1.0.0 |
| Azure secrets | `azure_security_keyvault_secrets` | 1.0.1 |
| Secrets in Vault | `vaultrs` | 0.8.0 |

Azure compute goes through the ARM REST API with `azure_core`, because `azure_mgmt_compute` has not been updated since 2024. AWS uses the `aws-sdk-*` crates above.

Background jobs use a small queue in the store: Postgres with `FOR UPDATE SKIP LOCKED` self-hosted, the organization's Durable Object in the cloud. No Redis, no message bus.

**Web app (TypeScript)**: see [typescript.md](typescript/ratchet.md).

| Package | Version |
| --- | --- |
| `react` / `react-dom` | 19.3.1 |
| `typescript` | 7.0.2 |
| `vite` / `@vitejs/plugin-react` | 8.3.8 / 6.3.0 |
| `@tanstack/react-query` / `@tanstack/react-router` | 5.107.1 / 1.170.46 |
| `@base-ui/react` (pinned exactly) | 1.9.0 |
| `openapi-typescript` / `openapi-fetch` | 7.13.0 / 0.17.0 |
| `vitest` / `@playwright/test` | 5.1.0 / 1.65.1 |
| `@biomejs/biome` | 2.6.7 |

**macOS app (target)**

A proposal, not yet built. The app is SwiftUI and macOS only; Windows and Linux use the web app. See [swift.md](swift/ratchet.md).

| Tool | Version |
| --- | --- |
| Xcode | 27.2 |
| Swift | 6.4.2 |
| UI | SwiftUI |
| SDK | macOS 27 |
| Deployment target | macOS 26 (a proposal) |
| Sparkle (updates) | 2.10.0 |

- Sparkle is the update mechanism for the Mac app (decided 15 Nov). The app is a direct download and is not on the Mac App Store.
- The runner is the same Rust `ratchet-runner`, bundled as a helper inside the app. There is no second implementation in Swift.
- Colours and words are generated from `design/tokens.toml` and the words catalogue, never written by hand.

**Delivery**

| Tool | Version |
| --- | --- |
| `cargo-chef` (Docker builds) | 0.1.78 |
| `worker-build` (builds the cloud shell; downloads esbuild and `wasm-opt`, so CI caches or pins them) | 0.8.7 |
| Wrangler | 4.156.2 |
| Helm | 4.3.2 |
| OpenTofu | 1.13.9 |
| Packer | 1.16.2 |
| cosign | 3.1.4 |
| syft | 1.57.1 |

Rules for any addition: self-hosting must keep working (no call to a service the administrator did not configure), the licence must suit a product we ship, and the crate must be maintained. A new outside system also needs a port with a real and a mock implementation; see [rust.md](rust/ratchet.md).

# Libraries

Do not add a dependency for convenience. A new one is a product call: ask the owner. Prefer what is already here. Before proposing one, review it with the supply-chain skill ([security/ratchet.md](security/ratchet.md)) and run `cargo deny check`; the review goes in the proposal.

## Chosen for the target

(target) None of this is in the repository yet. Versions are the latest stable releases on 2026-11-14, read from crates.io, npm and each project's release feed. Pin these when the code lands; raise them on purpose, never by accident.

**Shape.** One Cargo workspace builds one core and two shells, with the CLI:

- `ratchet-core`: all logic and the API routes as an axum `Router`, behind one `Store` trait. It never imports tokio's runtime, `sqlx` or a Cloudflare crate, and builds for `wasm32-unknown-unknown`. It never connects to data.
- `ratchet-server`: the self-hosted shell around the core. The API, the embedded web app, Postgres. It talks out to the agent platforms over HTTPS.
- `ratchet-cloud`: the Ratchet cloud shell around the core, a Cloudflare Worker with one Durable Object (SQLite) per organization.
- The agent platform adapters (AWS AgentCore, then Claude Managed Agents, then the OpenAI Agents API, then Microsoft Foundry Agent Service, then Google Agent Runtime), one adapter each behind one platform trait, in their own crate or crates. They call the platforms over HTTPS from either shell. Ratchet runs no sandboxes and no runner. Which client crates the adapters use is not decided; a new dependency is a product call.
- `ratchet`: the CLI and the MCP server, built on the API's types.


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
| TLS (self-hosted server only; its `aws-lc-rs` provider never enters the core) | `rustls` | 0.23.46 |
| HTTP client | `reqwest` | 0.13.5 |
| Serialization | `serde` / `serde_json` | 1.0.229 / 1.0.154 |
| Postgres, migrations (self-hosted store) | `sqlx` (+ `sqlx-cli`) | 0.9.0 |
| Cloud shell (Cloudflare Worker, runs the axum router with the `http` feature) | `worker` (workers-rs) | 0.8.7 |
| OpenAPI from Rust types | `utoipa` / `utoipa-axum` | 6.0.0 / 0.3.0 |
| Sign-in: OpenID Connect, and GitHub's OAuth (default features off; each shell supplies the HTTP client) | `openidconnect` / `oauth2` | 4.1.0 / 5.0.0 |
| Receipt signing (ed25519), token and session hashes, constant-time compare (pure Rust, builds for wasm) | `ed25519-dalek` / `sha2` / `subtle` | 3.0.0 / 0.11.1 / 2.6.1 |
| Embedded web app | `rust-embed` | 8.14.0 |
| AWS (see the note under the table) | `aws-config` | 1.13.2 |
| | `aws-sdk-ec2` / `aws-sdk-s3` / `aws-sdk-sts` | 1.280.0 / 1.159.0 / 1.126.0 (not needed by the target) |
| | `aws-sdk-dynamodb` / `aws-sdk-secretsmanager` | 1.139.0 / 1.128.0 (not needed by the target) |
| GitHub | `octocrab` | 0.54.9 |
| Postgres connector (not needed by the target unless credential minting is ruled; the server never connects to data) | `tokio-postgres` | 0.7.18 |
| Egress proxy, TLS interception (not needed by the target: no egress proxy) | `hudsucker` / `rcgen` | 0.25.0 / 0.14.10 |
| MCP server | `rmcp` | 3.10.0 |
| Command line | `clap` | 4.6.7 |
| Ids, time | `uuid` / `jiff` | 1.29.2 / 0.2.43 |
| Errors | `thiserror` / `anyhow` | 2.0.21 / 1.0.104 |
| Logs | `tracing` / `tracing-subscriber` | 0.1.44 / 0.3.23 |
| Telemetry | `opentelemetry` / `opentelemetry-otlp` / `tracing-opentelemetry` | 0.33.1 / 0.33.1 / 0.34.0 |
| Tests | `insta` / `proptest` | 1.49.0 / 1.12.0 |
| Postgres in tests | `testcontainers` / `testcontainers-modules` | 0.28.0 / 0.15.0 |
| Test runner, licences | `cargo-nextest` / `cargo-deny` | 0.9.152 / 0.20.2 |

Sessions and access tokens need no library: each is 32 random bytes shown once, stored as a SHA-256 hash through the `Store` and compared in constant time, so both stores behave the same. `tower-sessions` and `aws-lc-rs` were dropped (November 2026): the first ties sessions to a `sqlx` store the cloud cannot use, the second is C code that does not build for wasm. Better Auth was considered and refused: it is TypeScript and keeps its own user, session and organization tables beside ours.

Snowflake, Databricks and BigQuery have no maintained Rust client we would ship; their connectors call each vendor's REST API through `reqwest`. Every connector sits behind one `Connector` trait, so its vocabulary stops there, and each says how to make a read-only, holdout-excluding credential for its system (whether Ratchet mints them per session is open). Agent platforms sit behind one agent trait, one adapter each; the judge is a separate trait. Platform crates stay inside each adapter.

AWS note: `aws-config` may serve the AgentCore adapter (request signing); the AWS SDK crate for AgentCore is not chosen, and that is a product call.

Background jobs (platform jobs, see [change/platform-job.md](../change/platform-job.md)) use a small queue in the store: Postgres with `FOR UPDATE SKIP LOCKED` self-hosted, the organization's Durable Object in the cloud. No Redis, no message bus.

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

- Sparkle is the update mechanism for the Mac app (decided 15 Nov; the app stays ruled in and its boards are owed). The app is a direct download and is not on the Mac App Store.
- The app is an API client only. It bundles no helper and runs no agent. "This machine" is deferred.
- Colours and words are generated from `design/tokens.toml` and the words catalogue, never written by hand.

**Delivery**

| Tool | Version |
| --- | --- |
| `cargo-chef` (Docker builds) | 0.1.78 |
| `worker-build` (builds the cloud shell; downloads esbuild and `wasm-opt`, so CI caches or pins them) | 0.8.7 |
| `cf` (the Cloudflare CLI, beta; agents use it) | 1.0.0-beta.32 |
| Wrangler (run by `cf` to build the Rust Worker, and `wrangler tail`) | 4.161.0 |
| cosign | 3.1.4 |
| syft | 1.57.1 |

Rules for any addition: self-hosting must keep working (no call to a service the administrator did not configure), the licence must suit a product we ship, and the crate must be maintained. A new outside system also needs a trait with a real and a mock implementation (an agent platform has the fake platform); see [rust.md](rust/ratchet.md).

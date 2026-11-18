# Libraries

Do not add a dependency for convenience. A new one is a product call: ask the owner. Prefer what is already here. Before proposing one, review it with the supply-chain skill ([security/antilog.md](security/antilog.md)) and run `cargo deny check`; the review goes in the proposal.

## Chosen for the target

(target) None of this is in the repository yet. Versions are the latest stable releases on 2026-11-14, read from crates.io, npm and each project's release feed. Pin these when the code lands; raise them on purpose, never by accident.

**Shape.** One Cargo workspace builds one core and one server, with the CLI:

- `antilog-core`: all logic and the API routes as an axum `Router`, behind one `Store` trait. It never imports `sqlx`, a cloud SDK or an adapter crate. It never connects to data.
- `antilog-server`: the one server, for Antilog cloud and self-hosted. The API, the embedded web app, Postgres, object storage. It ships as a container image and a Helm chart, and talks out to the agent platforms over HTTPS.
- The agent platform adapters (AWS AgentCore, then the Kubernetes sandbox adapter, then Microsoft Foundry Agent Service and Google Agent Runtime, then Claude Managed Agents and the OpenAI Agents API), one adapter each behind one platform trait, in their own crate or crates. They call the platforms over HTTPS. Antilog runs no sandboxes of its own outside the Kubernetes adapter, which runs them in the same cluster. Their client crates are in the table below.
- `antilog`: the CLI and the MCP server, built on the API's types.


**Toolchain**

| Tool | Version |
| --- | --- |
| Rust (stable) | 1.100.0 |
| Postgres | 18.7 |
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
| Postgres, migrations | `sqlx` (+ `sqlx-cli`) | 0.9.0 |
| Object storage (S3, Azure Blob, GCS; backend chosen by configuration) | `object_store` | 0.14.2 |
| OpenAPI from Rust types (paths listed in `ApiDoc`; `utoipa-axum` dropped: its `routes!` refuses generic handlers) | `utoipa` | 6.0.0 |
| Sign-in: OpenID Connect, and GitHub's OAuth  | `openidconnect` / `oauth2` | 4.1.0 / 5.0.0 |
| Receipt signing (ed25519), token and session hashes, constant-time compare (pure Rust) | `ed25519-dalek` / `sha2` / `subtle` | 3.0.0 / 0.11.1 / 2.6.1 |
| Embedded web app | `rust-embed` | 8.14.0 |
| AWS credentials (workload identity through the default chain) | `aws-config` | 1.13.2 |
| Server on Lambda: serves the axum `Router` / runs the handler | `lambda_http` / `lambda_runtime` | 1.3.2 / 1.5.0 |
| Timer adapter on AWS (one-off wake-ups) | `aws-sdk-scheduler` | 1.122.0 |
| AgentCore adapter (data plane / control plane) | `aws-sdk-bedrockagentcore` / `aws-sdk-bedrockagentcorecontrol` | 1.86.0 / 1.101.0 |
| Kubernetes sandbox adapter | `kube` / `k8s-openapi` | 5.1.0 / 0.29.0 |
| Google Agent Runtime adapter (when a GCP client signs) | `google-cloud-aiplatform-v1` | 1.19.0 |
| Foundry adapter sign-in (its REST API through `reqwest`; Azure has no Rust agents crate) | `azure_core` | 1.1.0 |
| GitHub | `octocrab` | 0.54.9 |
| Postgres connector (not needed by the target unless credential minting is ruled; the server never connects to data) | `tokio-postgres` | 0.7.18 |
| Egress proxy, TLS interception (not needed by the target: no egress proxy) | `hudsucker` / `rcgen` | 0.25.0 / 0.14.10 |
| MCP server | `rmcp` | 3.10.0 |
| Command line | `clap` | 4.6.7 |
| Ids, time | `uuid` / `jiff` | 1.29.2 / 0.2.43 |
| Randomness for tokens and keys (owner, 18 Nov) | `getrandom` | 0.4.3 |
| Errors | `thiserror` / `anyhow` | 2.0.21 / 1.0.104 |
| Logs | `tracing` / `tracing-subscriber` | 0.1.44 / 0.3.23 |
| Telemetry | `opentelemetry` / `opentelemetry-otlp` / `tracing-opentelemetry` | 0.33.1 / 0.33.1 / 0.34.0 |
| Tests | `insta` / `proptest` | 1.49.0 / 1.12.0 |
| Postgres in tests (`GenericImage`; `testcontainers-modules` 0.15.0 needs `testcontainers` 0.27, so it is dropped) | `testcontainers` | 0.28.0 |
| Test runner, licences | `cargo-nextest` / `cargo-deny` | 0.9.152 / 0.20.2 |

The three AWS SDK crates are used with `default-features = false` and `default-https-client`, `rt-tokio`: their default `rustls` feature is the legacy client (rustls 0.21 and h2 0.3) and fails `cargo deny`.

Sessions and access tokens need no library: each is 32 random bytes shown once, stored as a SHA-256 hash through the `Store` and compared in constant time. `tower-sessions` was dropped (November 2026): the hashed-token scheme is small and keeps sessions in our own tables. Better Auth was considered and refused: it is TypeScript and keeps its own user, session and organization tables beside ours.

Snowflake, Databricks and BigQuery have no maintained Rust client we would ship; their connectors call each vendor's REST API through `reqwest`. Every connector sits behind one `Connector` trait, so its vocabulary stops there, and each says how to make a read-only, holdout-excluding credential for its system (whether Antilog mints them per session is open). Agent platforms sit behind one agent trait, one adapter each; the judge is a separate trait. Platform crates stay inside each adapter.

Cloud SDKs stay inside their adapter crate. The server reads secrets from mounted files or the environment, so it needs no secrets-manager SDK on any cloud.

Background jobs (platform jobs, see [change/platform-job.md](../change/platform-job.md)) use a small queue in the store: Postgres with `FOR UPDATE SKIP LOCKED`. No Redis, no message bus, no cloud queue.

**Web app (TypeScript)**: see [typescript.md](typescript/antilog.md).

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

A proposal, not yet built. The app is SwiftUI and macOS only; Windows and Linux use the web app. See [swift.md](swift/antilog.md).

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
| `cargo-lambda` (builds and invokes the Lambda binary) | 1.9.2 |
| Helm (the chart) | pinned when the chart lands (target) |
| OpenTofu (one module per cloud) | 1.13.9, with the `aws` provider `~> 6.73` |
| k3s (the chart's install test in CI) | pinned when the chart lands (target) |
| cosign | 3.1.4 |
| syft | 1.57.1 |

Rules for any addition: self-hosting must keep working (no call to a service the administrator did not configure), the licence must suit a product we ship, and the crate must be maintained. A new outside system also needs a trait with a real and a mock implementation (an agent platform has the fake platform); see [rust.md](rust/antilog.md).

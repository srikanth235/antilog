# Libraries

Do not add a dependency for convenience. A new one is a product call: ask the owner. Prefer what is already here. Before proposing one, review it with the supply-chain skill ([security/antilog.md](security/antilog.md)) and run `cargo deny check`; the review goes in the proposal.

## Chosen for the target

(target) None of this is in the repository yet. Versions are the latest stable releases on 2026-11-14, read from crates.io, npm and each project's release feed. Pin these when the code lands; raise them on purpose, never by accident.

**Shape.** One Cargo workspace builds one core and one server, with the CLI:

- `antilog-core`: all logic and the API routes as an axum `Router`, behind one `Store` trait. It never imports `sqlx`, a cloud SDK or an adapter crate. It never connects to data.
- `antilog-server`: the one server, for Antilog cloud and self-hosted. The API, the embedded web app, Postgres, object storage. It ships as a container image.
- `antilog`: the CLI and the MCP server, built on the API's types.


**Toolchain**

| Tool | Version |
| --- | --- |
| Rust (stable) | 1.100.0 |
| Postgres | 18.7 |
| Node.js (LTS, web app build only; owner moved it from 26 on 18 Nov) | 22.23.5 |
| pnpm | 12.18.0 |

**Rust crates**

| Need | Crate | Version |
| --- | --- | --- |
| Async runtime | `tokio` (features include `time` for the live stream's sleeps) | 1.54.1 |
| Server-sent events for axum (`ReceiverStream`, `StreamExt::map`); already in the build through axum and sqlx, default features off | `tokio-stream` | 0.1.19 |
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
| Webhook signature (HMAC-SHA256, `X-Hub-Signature-256`) checked with `verify_slice`, which compares in constant time; chosen over a hand-written HMAC, and already in the lockfile | `hmac` | 0.13.0 (matches `sha2` 0.11) |
| Receipt signing (ed25519), token and session hashes, constant-time compare (pure Rust) | `ed25519-dalek` / `sha2` / `subtle` | 3.0.0 / 0.11.1 / 2.6.1 |
| Embedded web app | `rust-embed` | 8.14.0 |
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

Sessions and access tokens need no library: each is 32 random bytes shown once, stored as a SHA-256 hash through the `Store` and compared in constant time. `tower-sessions` was dropped (November 2026): the hashed-token scheme is small and keeps sessions in our own tables. Better Auth was considered and refused: it is TypeScript and keeps its own user, session and organization tables beside ours.

Snowflake, Databricks and BigQuery have no maintained Rust client we would ship; their connectors call each vendor's REST API through `reqwest`. Every connector sits behind one `Connector` trait, so its vocabulary stops there, and each says how to make a read-only, holdout-excluding credential for its system (whether Antilog mints them per session is open).

The server reads secrets from mounted files or the environment, so it needs no cloud SDK.

Background jobs use a small queue in the store: Postgres with `FOR UPDATE SKIP LOCKED`. No Redis, no message bus, no cloud queue.

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
| cosign | 3.1.4 |
| syft | 1.57.1 |

Rules for any addition: self-hosting must keep working (no call to a service the administrator did not configure), the licence must suit a product we ship, and the crate must be maintained. A new outside system also needs a trait with a real and a mock implementation; see [rust.md](rust/antilog.md).

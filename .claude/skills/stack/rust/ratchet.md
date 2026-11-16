# Rust in Ratchet

(target) No product Rust is on this branch yet. This page says how the code is written when it lands. Rules from the earlier daemon on `claude/peaceful-lamport-leak6p` that still hold are kept; its commands are marked as the earlier code's.

## Workflow and done criteria

Run these, in this order, and fix code rather than weakening a check:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace          # (target); `cargo test --doc` for doctests
cargo deny check                       # (target) licences, advisories, duplicate and banned crates
```

Set `export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0` for local builds when a cloud SDK makes full debug info huge (a lesson from the earlier code, which used the AWS SDK).

**Done** when all four are green and the tests for the kind of change are written ([land/tests.md](../../land/tests.md)). `cargo test --workspace` is the fallback while nextest is not set up. For a performance change, reproduce it, measure an optimized build, profile, fix, and measure again with the same inputs; put before and after in the commit message. Routine changes do not need this loop.

Fix the code, not the check. Never add `#[allow]` or loosen a lint to get green. When a lint is wrong for one item, use `#[expect(clippy::name, reason = "...")]` on the narrowest item: it warns when it goes stale.

## What gets built (target)

One Cargo workspace, one core with two shells, versions in [libraries.md](../libraries.md):

- `ratchet-core`: all logic. The accept rule, run states, budgets, permissions, the audit log and the API routes as an axum `Router`. It asks storage by name through one `Store` trait and never writes SQL. It orchestrates and records and **never connects to customer data**.
- `ratchet-server`: the self-hosted shell. tokio and axum's server, the Postgres `Store`, tokio timers, `NOTIFY` for live events, the job queue, the embedded web app. It reads each session's events from the agent platforms over HTTPS.
- `ratchet-cloud`: the Ratchet cloud shell. A Cloudflare Worker built with `worker` (workers-rs, `http` feature) that runs the core's `Router`, with one Durable Object per organization holding its SQLite `Store`, its live event streams, its job queue and its timers (alarms). It serves the static web app.
- Two `Store` implementations: Postgres through `sqlx` (in the self-hosted shell) and Durable Object SQLite (in the cloud shell). Each sits in its own crate, so the core depends on neither.
- `ratchet`: the CLI and the MCP server, built on the API's generated types.

Shared types (the API's request and response types, ids, states) live in a library crate that all of them use. No gRPC, no Redis: the queue lives in the store, which is Postgres self-hosted and the organization's Durable Object in the cloud ([../postgres/ratchet.md](../postgres/ratchet.md)). Every agent platform is one adapter behind the platform trait; the core does not change for a new one. The adapters live in their own crate or crates, so the core depends on none of them, and all connections go out over HTTPS.

### The core stays portable

- **`ratchet-core` never imports tokio's runtime, `sqlx` or a Cloudflare crate.** It opens no socket and starts no thread. Time, ids, timers and storage come in through traits that a shell implements. There is no `SystemTime` on wasm, so the shell passes the clock in and UUIDv7 ids are made from it.
- **`Store` futures are `Send`.** axum's handlers need `Send` futures, so the trait's methods return `impl Future<Output = ...> + Send`. The cloud shell meets that with workers-rs's own `worker::send::SendWrapper` and `SendFuture`, never with an `unsafe impl` of ours (the no-`unsafe` rule below holds in every crate). The local trial confirmed the pattern; see "Cloud shell trial" below.
- **It builds natively and for `wasm32-unknown-unknown`, and CI builds both on every commit** ([land/ci.md](../../land/ci.md)), so a slip fails at once. A dependency that does not build for wasm is a reason to put the code in a shell, not in the core.
- **The shells hold no logic.** A rule found in a shell moves to the core, so the two clouds cannot disagree.

### Cloud shell trial (local, November 2026)

The Rust cloud shell was tried locally with `wrangler dev`, no Cloudflare account, before committing to it. Result: **proceed with Rust**; the TypeScript fallback is not needed. Versions used: `worker` and `worker-build` 0.8.7, Wrangler 4.156.2, axum 0.8.9 with default features off in the core. It was repeated with `cf dev` (`cf` 1.0.0-beta.32 running Wrangler 4.161.0) with the same results.

| Question | Result |
| --- | --- |
| Core builds native and wasm | Works. `uuid` needs its `js` feature on `wasm32` only; axum in the core takes `default-features = false` (tokio's I/O does not build for wasm). Do not name a crate `core`: it shadows the standard library |
| Worker serves the core's `Router` | Works, through the `http` feature |
| Durable Object SQLite store | Works: migrations on start, `STRICT`, `returning`, append-only triggers, one object per organization isolated |
| Transactions | Works. workers-rs wraps the async `transaction`, which rolls back; `transactionSync` needs a small `wasm_bindgen` extern of our own |
| Foreign keys | On by default, and enforced |
| Integers above 2^53 | Need care: see [../postgres/general/sqlite.md](../postgres/general/sqlite.md) |
| Alarms | Work from a Rust `alarm` handler, and chain |
| Live events | WebSocket (with hibernation) and server-sent events both held for over two minutes and received pushes |
| D1 directory | Works from Rust |
| Size | 0.87 MB of wasm (0.30 MB gzipped) against a 64 MiB uncompressed limit; release build about 30 s |
| Point-in-time recovery | Not wrapped by workers-rs and not supported locally; needs an extern and a deployed test |

Still open, and only a deployed test can answer it: point-in-time restore, hibernation timing and cost, cold starts with migrations, real CPU limits, jurisdictions, throughput near 1,000 requests a second, `SQLITE_FULL` at 10 GB, and stream idle timeouts at Cloudflare's edge. That test needs a separate Cloudflare account or a scoped token, asked of the owner first.

## Constraints

- **No `unsafe`.** Every crate carries `#![forbid(unsafe_code)]` (or `[lints.rust] unsafe_code = "forbid"` at the workspace). Do not weaken it. A need for `unsafe` is a stop-and-ask, not a local decision. This was already so for the sandbox, bench and engine crates of the earlier code.
- **No `unwrap()` in product code.** `expect("...")` names the invariant that makes the failure impossible ("the regex is a literal"), not the symptom. Tests and `main` start-up may unwrap. Enable `clippy::unwrap_used` for non-test code so review does not have to.
- **Errors.** Libraries return typed errors with `thiserror` and keep the cause chain (`#[source]` or `#[from]`). Binaries (`ratchet-server`, `ratchet`) use `anyhow` at the edge, in `main` and CLI glue, to add context and print. Handlers map a typed error to one stable error code and status; the internal cause goes to the log with the trace id, never to the client.
- **Async is `tokio`; no blocking work on async tasks.** Blocking or CPU-heavy work goes through `spawn_blocking` or a thread; locks are not held across an `.await`; every queue and every concurrency is bounded; every call that leaves the process has a timeout; shutdown is graceful. Details and the reasons: [general/async-services.md](general/async-services.md). Async methods in traits use native `async fn in trait` where the trait is not used as `dyn`, and `async-trait` where it must be (`Box<dyn Backend>`).
- **Every outside system sits behind one trait** with a real implementation and a test double. Code depends on the trait, never on the real thing, and a new outside system is a new trait with both. This includes: agent platforms (Claude Managed Agents, then AgentCore, then the OpenAI Agents API, behind **one** platform trait; platform words and platform crates stay inside each adapter), connectors (each behind one `Connector` trait that says how to make a read-only, holdout-excluding credential for its system), git hosts, agents and model providers, clocks and id generators.
- **The server never connects to customer data.** `ratchet-core`, `ratchet-server` and `ratchet-cloud` have no connector client crates in their dependency trees: not `tokio-postgres`, not a warehouse or object-store client. Those belong to the agents' and the evaluator's sandboxes on the platforms. Check `cargo tree -p ratchet-server` (and the other two) when a dependency is added. The server stores ids, versions, numbers, states and audit rows only.
- **Secrets stay out of everything but their owner.** Never put a connector secret, token, model key or credential in a log, a tracing field, an error message, a response, a tag or state. Wrap secrets in a type whose `Debug` and `Display` print `[redacted]` (`secrecy` is not in [libraries.md](../libraries.md): write a small newtype, or ask before adding the crate). Do not derive `Debug` on a struct that holds one without checking.
- **Time and ids come from one place.** Use `jiff` for time and `uuid` for ids as in [libraries.md](../libraries.md), through a clock and id source that tests can fix. Store instants in UTC; format at the edge.
- **Dependencies.** Do not add one for convenience; a new one is a product call ([libraries.md](../libraries.md)). `cargo deny` (target) checks licences and advisories. Dependencies are declared once in `[workspace.dependencies]` and inherited (`dep.workspace = true`); features are additive.
- **Keep work whose cost grows with the data off request paths:** page by keyset cursor with `LIMIT`; do not load a whole table.
- **`Cargo.lock`:** (target) commit it. These are binaries, and `cargo deny` and reproducible builds need it. The earlier code did not commit it; that is not the rule here.
- **Edition and toolchain.** Edition 2024 and the stable toolchain pinned in `rust-toolchain.toml` to the version in [libraries.md](../libraries.md). `rustfmt` and `clippy` components installed. Do not use nightly features.

## Tests

How tests are written: [land/tests.md](../../land/tests.md). In Rust:

- Snapshots with `insta` (review with `cargo insta review`), properties with `proptest`, database tests against a real Postgres in a container with `testcontainers` ([../postgres/ratchet.md](../postgres/ratchet.md)), and one storage suite, written against the `Store` trait, that runs against both the Postgres store and the SQLite store, async time tests with `#[tokio::test(start_paused = true)]`.
- Test the public behaviour of a crate from `tests/`; unit-test pure logic in the module. A test never talks to a real cloud, a real agent or the network: use the doubles. A test that does is a real-cloud run and follows [land/real-cloud.md](../../land/real-cloud.md).
- Contract tests pin the API: the OpenAPI document generated from the Rust types (`utoipa`) is compared with a committed snapshot, and the web client and the CLI are generated from it. A change to a type changes the snapshot in the same change. How: [change/api-endpoint.md](../../change/api-endpoint.md).
- Tests are deterministic: fixed clock, seeded randomness, no sleeps. See the pitfalls.

## Compiler and clippy errors

Fix the first error first: later ones are often its echo. Clippy runs with `-D warnings`, so every warning is an error; fix the code, never add `#[allow]` to go green.

| Error | Fix |
| --- | --- |
| `E0382` use of moved value | Borrow it (`&x`), move the last use later, or restructure so one owner consumes it; clone only if two owners are really needed |
| `E0502` / `E0499` cannot borrow as mutable more than once | Shorten the first borrow (end it before the second), split the struct into disjoint fields, or collect indices first and mutate after |
| `E0505` move out while borrowed | End the borrow first, or move the data into the closure or task that needs it |
| `E0106` missing lifetime specifier | Return an owned value, or name one lifetime tying the output to the input it borrows from |
| `E0597` / `E0515` does not live long enough, returns reference to local | Return the owned value, or take the buffer from the caller |
| "future is not `Send`" | A non-`Send` value (`Rc`, `RefCell` guard, `std::sync::MutexGuard`) is held across an `.await`; drop it in a block before the await, or use `Arc` and, if it must be held across an await, `tokio::sync::Mutex` |
| `tokio::spawn` needs `'static` | The task borrows something local. Move an owned value or an `Arc` clone into it; do not leak with `'static` |
| `E0277` trait bound not satisfied | Read the "required by" note; add the bound to the generic, derive the trait, or convert the type |
| `E0308` mismatched types | Convert at the boundary (`.into()`, `?` with a `From` impl, `as_deref()`), not with a cast |
| `E0599` no method found | Import the trait that provides it, or the type lacks a derive or a feature flag |
| `clippy::unwrap_used`, `clippy::expect_used` | Use `?`, `if let`, `unwrap_or`, or return a typed error; `expect` only with a message naming the invariant (and `#[expect(clippy::expect_used, reason = "...")]` if the lint is on) |
| `clippy::needless_borrow`, `redundant_clone`, `clone_on_copy` | Delete the `&` or `.clone()` it points at |
| `clippy::too_many_arguments`, `type_complexity` | Group arguments in a struct; name the type with an alias |
| `clippy::await_holding_lock`, `await_holding_refcell_ref` | A guard is held across an `.await`: scope it before the await, or restructure so one task owns the state |
| `unused_must_use` / unused `Result` or `JoinHandle` | Handle it with `?`; `let _ =` only for a deliberate best-effort call, with a comment. An un-awaited `JoinHandle` hides a panic: keep it or use a `TaskTracker` |
| `sqlx` "set `DATABASE_URL`" or a query macro cannot connect | Build with the committed offline data (`SQLX_OFFLINE=true`) or point `DATABASE_URL` at a migrated database; regenerate the data with `cargo sqlx prepare` after a query changes |
| "usage of an `unsafe` block" under `#![forbid(unsafe_code)]` | Do not lift the forbid; use a safe API or a vetted crate and ask |

## Pitfalls

- **Never sort, key, hash or compare a `serde_json::Value` by its printed text.** `to_string` prints keys in insertion order when `preserve_order` is on and in name order when off. Cargo unifies that feature across the whole build and any dependency can turn it on, so a test can pass for one crate and fail for the workspace. Compare values with `==`, canonicalise (sorted keys, one number spelling) before hashing or signing, and re-run JSON-ordering claims across the whole workspace. Do not toggle `preserve_order` to fix it. This matters twice here: receipts are signed, and a verdict must be byte-for-byte reproducible.
- **A test that writes an executable and runs it is slow on macOS** (the first run of a new file waits on the system's code assessment, one file at a time, and a parallel runner makes it worse). Do not guess a fixed window for a fake tool: wait on a condition with a generous ceiling, wait for a line to be whole (ends in a newline) and not for the file to exist, and prefer a binary the system ships (`/bin/sh`) over a fresh one. This bites any test that starts a helper process.
- **Feature unification hides bugs.** A crate that builds alone can fail in the workspace, or the reverse. CI builds the workspace, and `cargo hack` (if adopted) checks feature combinations; features must be additive.
- **`select!` drops the losers.** A branch that is not cancel safe loses data on a dropped future; see [general/async-services.md](general/async-services.md).
- **Lock held across `.await`** is the classic server deadlock and shows as a "not `Send`" error or a hang. Take the lock, copy out, drop, then await.
- **Do not trust a string from outside** (a path, a branch name, a tag value) in a command or a path. Build commands with an argument array, never a shell string, and validate against an allowed root.

## General Rust guidance

[`general/general-practices.md`](general/general-practices.md) covers ownership, API design, errors and panics, concurrency, unsafe, modules, tests and idioms. Source digests: [`rust-api-guidelines.md`](general/rust-api-guidelines.md), [`effective-rust.md`](general/effective-rust.md), [`rust-design-patterns.md`](general/rust-design-patterns.md), [`microsoft-rust-guidelines.md`](general/microsoft-rust-guidelines.md), [`google-rust-style-guide.md`](general/google-rust-style-guide.md), and [`async-services.md`](general/async-services.md) for tokio, axum, tower and sqlx in a server.

The diagnostic table, measuring guidance and done criteria adapt [Bama](https://github.com/daveebbelaar/bama) by Dave Ebbelaar (MIT, [`licenses/Bama-LICENSE`](licenses/Bama-LICENSE)). [BeautifulRust](https://github.com/morganlinton/BeautifulRust) by Morgan Linton (MIT, [`licenses/BeautifulRust-LICENSE`](licenses/BeautifulRust-LICENSE)). Licences for the async digest: [tokio](licenses/tokio-LICENSE), [axum](licenses/axum-LICENSE), [sqlx](licenses/sqlx-LICENSE-MIT).

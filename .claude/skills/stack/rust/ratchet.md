# Rust in Ratchet

(target) No product Rust is on this branch yet. This page says how the code is written when it lands.

## Workflow and done criteria

Run these, in this order, and fix code rather than weakening a check:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace          # (target); `cargo test --doc` for doctests
cargo deny check                       # (target) licences, advisories, duplicate and banned crates
```

Set `export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0` for local builds when a cloud SDK makes full debug info huge.

**Done** when all four are green and the tests for the kind of change are written ([land/tests.md](../../land/tests.md)). `cargo test --workspace` is the fallback while nextest is not set up. For a performance change, reproduce it, measure an optimized build, profile, fix, and measure again with the same inputs; put before and after in the commit message. Routine changes do not need this loop.

Comments and API shape follow "Every language" in [../SKILL.md](../SKILL.md).

Fix the code, not the check. Never add `#[allow]` or loosen a lint to get green. When a lint is wrong for one item, use `#[expect(clippy::name, reason = "...")]` on the narrowest item: it warns when it goes stale.

## What gets built (target)

One Cargo workspace, versions in [libraries.md](../libraries.md):

- `ratchet-core`: all logic. The acceptance criteria (one pure function), the judge's queue, the budget meter, approvals and policy, permissions, the audit log and the API routes as an axum `Router`. It asks storage by name through one `Store` trait and never writes SQL. It orchestrates and records and **never connects to customer data**.
- `ratchet-server`: the one server, for Ratchet cloud and self-hosted alike, with two entry points over the same axum `Router`: a Lambda one through `lambda_http` (built with `cargo-lambda`, for pay-per-use) and a normal tokio listener (for Kubernetes). The Postgres `Store` through `sqlx`, the job queue, object storage through `object_store`, the embedded web app, and `NOTIFY` for live events on Kubernetes only. It reads each session's events from the agent platforms over HTTPS. It ships as one container image and a Helm chart ([../kubernetes/ratchet.md](../kubernetes/ratchet.md)). It is a native Linux binary, not WebAssembly, so tokio, sqlx, rustls and the AWS SDK all work.
- The platform adapters, each in its own crate: AgentCore first (the AWS SDK's `aws-sdk-bedrockagentcore` and `aws-sdk-bedrockagentcorecontrol`), then the Kubernetes sandbox adapter (`kube`), then Foundry (its REST API with `azure_core` for sign-in, since Azure has no Rust agents crate) and Google Agent Runtime (`google-cloud-aiplatform-v1`).
- `ratchet`: the CLI and the MCP server, built on the API's generated types.

Shared types (the API's request and response types, ids, states) live in a library crate that all of them use. No gRPC, no Redis: the queue lives in Postgres ([../postgres/ratchet.md](../postgres/ratchet.md)). Every agent platform is one adapter behind the platform trait; the core does not change for a new one, and depends on none of them. All connections go out over HTTPS.

### Two ways to run, one set of rules (target)

Scale-to-zero (Lambda on AWS; Cloud Run on GCP and Container Apps on Azure when a client needs them) freezes the process between requests. The core obeys these rules in both setups, so it never knows which one it is in:

- **Nothing runs after a response is sent.** No spawned tokio task, in-process timer or background loop outlives its request. Work that must happen later is a row in Postgres plus a wake-up.
- **Timers sit behind a trait.** Due jobs stay rows in Postgres. The Lambda adapter books each due time as a one-off wake-up with the cloud's scheduler (EventBridge Scheduler through `aws-sdk-scheduler`); the Kubernetes adapter runs the timer in the process. One adapter per cloud, and the core gets timers only through the trait. Wake-ups are at least once, so job handlers stay idempotent.
- **An invocation is at most 15 minutes.** Long work (relaying a session's events, waiting on a measurement or a watch) runs as resumable steps of about 10 minutes, each saving the platform's event cursor and booking its next wake-up. Agent sessions and judge runs run on AgentCore, never in the server. Lambda durable functions are not used: their SDK is TypeScript and Python only.
- **Event streams end before the cap.** The browser reconnects with `Last-Event-ID` and the stream replays from the events table. See [../../change/live-event.md](../../change/live-event.md).

### The core stays clean

- **`ratchet-core` never imports `sqlx`, a cloud SDK or an adapter crate.** Time, ids, storage and platforms come in through traits that the server implements, so the core is tested with doubles and fixed clocks.
- **Nothing is specific to one cloud outside an adapter or the deployment config.** Secrets come from mounted files or the environment, cloud access from workload identity through each SDK's default credential chain, and files through `object_store`, whose backend is chosen by configuration. A cloud name in the core or the server is a finding.

## Constraints

- **No `unsafe`.** Every crate carries `#![forbid(unsafe_code)]` (or `[lints.rust] unsafe_code = "forbid"` at the workspace). Do not weaken it. A need for `unsafe` is a stop-and-ask, not a local decision.
- **No `unwrap()` in product code.** `expect("...")` names the invariant that makes the failure impossible ("the regex is a literal"), not the symptom. Tests and `main` start-up may unwrap. Enable `clippy::unwrap_used`, `clippy::expect_used` (allowing it only with the named invariant), `clippy::indexing_slicing` and `clippy::panic` for non-test code so review does not have to. Use `.get(i)` and handle `None`. A panic in a request path drops that request's work half-done and can poison shared state for every organization the server holds.
- **Errors.** Libraries return typed errors with `thiserror` and keep the cause chain (`#[source]` or `#[from]`). Binaries (`ratchet-server`, `ratchet`) use `anyhow` at the edge, in `main` and CLI glue, to add context and print. Handlers map a typed error to one stable error code and status; the internal cause goes to the log with the trace id, never to the client.
- **Async is `tokio`; no blocking work on async tasks.** Blocking or CPU-heavy work goes through `spawn_blocking` or a thread; locks are not held across an `.await`; every queue and every concurrency is bounded; every call that leaves the process has a timeout; shutdown is graceful. Details and the reasons: [general/async-services.md](general/async-services.md). Async methods in traits use native `async fn in trait` where the trait is not used as `dyn`, and `async-trait` where it must be (`Box<dyn Backend>`).
- **Every outside system sits behind one trait** with a real implementation and a test double. Code depends on the trait, never on the real thing, and a new outside system is a new trait with both. This includes: agent platforms (Claude Managed Agents, then AgentCore, then the OpenAI Agents API, then Microsoft Foundry Agent Service, then Google Agent Runtime, behind **one** agent trait, with the judge as a separate trait; platform words and platform crates stay inside each adapter), connectors (each behind one `Connector` trait that says how to make a read-only, holdout-excluding credential for its system), git hosts, agents and model providers, clocks and id generators.
- **The server never connects to customer data.** `ratchet-core` and `ratchet-server` have no connector client crates in their dependency trees: not `tokio-postgres`, not a warehouse or object-store client. Those belong to the agents' sandboxes and the judge sandboxes on the platforms. Check `cargo tree -p ratchet-server` when a dependency is added. The server stores ids, versions, numbers, states, commit ids, the conversation's words, spend and audit rows only.
- **Secrets stay out of everything but their owner.** Never put a connector secret, token, model key or credential in a log, a tracing field, an error message, a response, a tag or state. Wrap secrets in a type whose `Debug` and `Display` print `[redacted]` (`secrecy` is not in [libraries.md](../libraries.md): write a small newtype, or ask before adding the crate). Do not derive `Debug` on a struct that holds one without checking.
- **Time and ids come from one place.** Use `jiff` for time and `uuid` for ids as in [libraries.md](../libraries.md), through a clock and id source that tests can fix. Store instants in UTC; format at the edge.
- **Dependencies.** Do not add one for convenience; a new one is a product call ([libraries.md](../libraries.md)). `cargo deny` (target) checks licences and advisories. Dependencies are declared once in `[workspace.dependencies]` and inherited (`dep.workspace = true`); features are additive.
- **Inputs from outside are strict.** Every type deserialized from a request, a tool call or a platform event carries `#[serde(deny_unknown_fields)]`, and a field the judge or the budget reads has no `#[serde(default)]`: an agent's misspelt field is an error, not a silent default. Defaults belong in the constructor that validates, once.
- **No lossy `as` casts.** Converting between integer types uses `try_from` (or `from` where it cannot fail); `as` is for the few float cases a comment cannot avoid. Budgets, token counts and money are where a silent wrap or truncation costs. Turn on `clippy::cast_possible_truncation`, `cast_sign_loss` and `cast_possible_wrap`.
- **Arithmetic on money and counts is checked.** `checked_add` and `checked_mul` (or saturating, when that is the rule) on spend, tokens and caps; overflow is an error, not a wrap.
- **Keep work whose cost grows with the data off request paths:** page by keyset cursor with `LIMIT`; do not load a whole table.
- **`Cargo.lock`:** (target) commit it. These are binaries, and `cargo deny` and reproducible builds need it.
- **Edition and toolchain.** Edition 2024 and the stable toolchain pinned in `rust-toolchain.toml` to the version in [libraries.md](../libraries.md). `rustfmt` and `clippy` components installed. Do not use nightly features.

## Tests

How tests are written: [land/tests.md](../../land/tests.md). In Rust:

- Snapshots with `insta` (review with `cargo insta review`), properties with `proptest`, database tests against a real Postgres in a container with `testcontainers` ([../postgres/ratchet.md](../postgres/ratchet.md)), and one storage suite, written against the `Store` trait, that runs against the Postgres store, async time tests with `#[tokio::test(start_paused = true)]`.
- Test the public behaviour of a crate from `tests/`; unit-test pure logic in the module. A test never talks to a real cloud, a real agent or the network: use the doubles. A test that does is a real-cloud run and follows [land/real-cloud.md](../../land/real-cloud.md).
- Schema tests pin the API: the OpenAPI document generated from the Rust types (`utoipa`) is compared with a committed snapshot, and the web client and the CLI are generated from it. A change to a type changes the snapshot in the same change. How: [change/api-endpoint.md](../../change/api-endpoint.md).
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

# Async services: tokio, axum, tower, sqlx (digest)

Sources, read on 14 Nov 2026:

- The Tokio tutorial and topics: [Shared state](https://tokio.rs/tokio/tutorial/shared-state), [Channels](https://tokio.rs/tokio/tutorial/channels), [Select](https://tokio.rs/tokio/tutorial/select), [Graceful shutdown](https://tokio.rs/tokio/topics/shutdown), [Bridging with sync code](https://tokio.rs/tokio/topics/bridging), [Getting started with tracing](https://tokio.rs/tokio/topics/tracing). Tokio project, MIT ([`../licenses/tokio-LICENSE`](../licenses/tokio-LICENSE)).
- The axum documentation: the [error handling](https://docs.rs/axum/latest/axum/error_handling/index.html) and middleware guides and the graceful-shutdown example, axum 0.8. axum project, MIT ([`../licenses/axum-LICENSE`](../licenses/axum-LICENSE)).
- The sqlx documentation for `Transaction` and the pool (sqlx project, MIT or Apache-2.0, [`../licenses/sqlx-LICENSE-MIT`](../licenses/sqlx-LICENSE-MIT)). The pool-sizing and compile-time-checking notes below are Antilog's own and not from a page above; see [../../postgres/antilog.md](../../postgres/antilog.md). Other points that are standard practice but not stated on those pages (timeouts, retries, error mapping, testing with paused time) are Antilog's own too.

Condensed in our own words. Versions are in [`../../libraries.md`](../../libraries.md); check an API against the version pinned there. [`../antilog.md`](../antilog.md) wins where they differ. Read this for a server that handles many requests and jobs at once: `antilog-server`, and the runner's supervisor loops.

## Model

- Async tasks are cooperative. A task runs until it reaches an `.await` that is not ready; between awaits it owns its thread. Anything that blocks a thread (a long computation, a blocking file or network call, a contended `std` lock, `std::thread::sleep`) stalls every other task scheduled on that thread, and so delays unrelated requests.
- Futures are lazy: nothing happens until a future is awaited or spawned. A loop calling `op()` without `.await` does nothing; with `.await` it runs one at a time. Concurrency and queueing exist only where you introduce them: `tokio::spawn`, `select!`, `join!`, channels.

## No blocking work on async tasks

- Short, non-blocking work is fine inline. Blocking or CPU-heavy work (hashing a large file, compressing, parsing a big document, a synchronous library such as a git or archive crate) goes to `tokio::task::spawn_blocking`, which runs it on a separate thread pool, and you `.await` the `JoinHandle`. For long-running CPU work that must not exhaust that pool, use a dedicated `std::thread` and report back through a `oneshot` channel.
- `spawn_blocking` work cannot be cancelled by dropping its future: the closure keeps running. Give it a way to stop (check an `AtomicBool` or a deadline) and keep what it does short.
- Use `tokio::fs`, `tokio::process` and `tokio::net`, not their `std` counterparts, inside async code. Use `tokio::time::sleep`, never `std::thread::sleep`.
- A synchronous program that needs a little async can own a `Runtime` and call `block_on` (the `current_thread` flavour suits a handful of tasks, but its spawned tasks only progress while `block_on` runs). Never call `block_on` from inside an async task.
- Detect the mistake: if latency spikes while CPU is low, look for a blocking call on a worker thread. `tokio-console` or a `tracing` span with a duration shows it.

## Shared state and locks

- A `std::sync::Mutex` is fine inside async code when the critical section is short, contention is low and the guard is **never held across an `.await`**. Its guard is not `Send`, so holding it across an await makes the future not `Send` and the compiler says so; it can also deadlock. Take the lock in a small block, copy what you need out, drop the guard, then await.
- `tokio::sync::Mutex` is for holding a lock across an await. It does not make a contended lock faster (it uses a synchronous mutex inside). If a lock is contended, the better fixes are: give the state to one task and talk to it by message passing, shard the mutex, or restructure to avoid it.
- Prefer ownership by one task plus channels over `Arc<Mutex<_>>` for anything with real logic. Share read-mostly data as `Arc<T>`; to swap it, send the new value over a `watch` channel.

## Channels, backpressure, bounded work

- Every queue is bounded. An unbounded queue turns overload into memory exhaustion at an unpredictable moment. Use `mpsc::channel(n)` with a chosen capacity (the sender awaits when full, which is the backpressure); use `try_send` when dropping or rejecting is the right answer, and return an overload error (HTTP 429 or 503) at the edge.
- Bound concurrency explicitly: a `tokio::sync::Semaphore` for the number of in-flight jobs or outbound calls; a limit on open connections in an accept loop; `buffer_unordered(n)` for streams of futures; a tower `ConcurrencyLimitLayer` or load-shed layer for requests.
- Channel kinds: `mpsc` many producers to one consumer; `oneshot` one reply (the usual way to answer a request sent to an actor task); `broadcast` every receiver gets every message (live events to many clients: receivers that lag get a `Lagged` error, so handle it); `watch` the latest value only (configuration, "is shutting down").
- If a `send` fails, the receiver is gone. Treat that as normal cancellation, not a panic.

## Cancellation and `select!`

- Cancelling a future means dropping it at an await point. Code after that await never runs. Do not rely on code running to completion unless the future is spawned or protected.
- `tokio::select!` runs several futures and drops the losers. Only use a future in `select!` if it is cancel safe: dropping it between polls loses nothing (`mpsc::Receiver::recv`, `sleep`, and `CancellationToken::cancelled` are; a future that has half-read a message into a local buffer is not). Re-create a future each loop turn only if that is harmless; otherwise pin it outside the loop.
- A drop is the hook for clean-up: implement `Drop` for guards, or let a `oneshot` receiver's drop signal the producer to stop. A `sqlx::Transaction` is rolled back on drop if it was not committed, so a request that is cancelled mid-transaction does not leave it half done, but the rollback is queued asynchronously, so do not rely on it for locks that must be released at once; end transactions explicitly.
- Do not hold a resource that other tasks wait on across an await that has no timeout.

## Timeouts

- Every call that leaves the process has a timeout: HTTP clients (`reqwest` connect and total timeouts), database acquires and statements, the runner's calls to the cloud, subprocesses. Use `tokio::time::timeout(dur, fut)` (it drops the future on expiry) or the client's own setting, and map the elapsed case to a typed error.
- Set a timeout on the server side too: a tower-http `TimeoutLayer` on the router so a request cannot hang forever, shorter than any proxy's timeout. The axum graceful-shutdown example adds one so shutdown does not wait on a stuck request.
- Pair timeouts with a retry only for idempotent operations, with backoff and a cap, and never retry inside a transaction.

## Tasks and shutdown

- `tokio::spawn` returns a `JoinHandle`; a panic in the task is reported only when you await it, and an un-awaited handle hides it. Keep handles, or spawn into a `tokio_util::task::TaskTracker` and `wait` on it. Do not spawn a detached task that can outlive what it borrows or that nobody supervises.
- Graceful shutdown has three parts: decide when to stop (a signal: `ctrl_c`, and `SIGTERM` on Unix, which is what Kubernetes sends; or an internal request), tell every part (a `tokio_util::sync::CancellationToken` cloned into each task, which each task selects on and then runs its own wind-down such as flushing), and wait for them to finish (the `TaskTracker`, with an overall deadline shorter than the platform's grace period).
- With axum: `axum::serve(listener, app).with_graceful_shutdown(signal_future)` stops accepting new connections and lets in-flight requests finish. Stop background workers with the same token. A worker holding a queue claim should finish or release it; with a Postgres queue, a lease that expires covers a worker that dies (see [../../postgres/antilog.md](../../postgres/antilog.md)).
- Start order matters: register the tracing subscriber first, then open the pool, run migrations, bind the listener, and only then report ready. A health endpoint should say "ready" only when the dependencies are.

## axum and tower

- axum has no middleware system of its own; it uses tower. Everything in tower and tower-http works: `TraceLayer` for request spans, `TimeoutLayer`, `CorsLayer`, `CompressionLayer`, request-id layers, concurrency and load-shed layers.
- Add several layers with `tower::ServiceBuilder`: they then run top to bottom (the first one added sees the request first). Chained `.layer()` calls wrap the earlier ones, so the last one added runs first. Put request-id and tracing outermost, then timeout and limits, then authentication, then the handler. `route_layer` applies a layer only to matched routes (so unmatched paths still 404 without running it); use it for auth.
- Handlers cannot fail at the service level: axum requires an infallible error type, and a handler's `Err(e)` is just another response, through `IntoResponse`. Define one application error enum (`thiserror`) that implements `IntoResponse`, mapping each variant to a status and a problem body, so handlers can use `?`. Log the internal cause with its trace id, and send the client a stable code and message, never the raw error. For middleware that can error (timeouts), convert the error to a response with `HandleErrorLayer`, or use the layers that already produce a status code.
- Extractors reject with their own response before the handler runs: decide on purpose how `Json`, `Path` and `Query` rejections look (your own extractor wrapping `Json`, or a `#[from_request(rejection(...))]` derive) so clients get the same error shape everywhere. Put a body size limit (`DefaultBodyLimit`) in front of anything that reads a body.
- Share state with `State<AppState>`, where `AppState` is `Clone` and cheap (an `Arc` inside, or fields that are themselves handles such as `PgPool`). Do not put per-request data in it.
- Keep handlers thin: extract, call a function that returns a typed result, map to a response. Business logic lives in plain functions that tests can call without HTTP. Streaming and live events use `Sse` over a `broadcast` or `watch` receiver; make each stream end when the client goes away or shutdown starts.

## sqlx in a server

- One `PgPool` for the process, created at start-up, cloned into state. Configure `max_connections`, `acquire_timeout` (so a saturated pool returns an error instead of hanging every request), `idle_timeout` and `max_lifetime`. Size it from the database's limit divided by the replicas (see [../../postgres/antilog.md](../../postgres/antilog.md)).
- Do not hold a pooled connection while doing slow non-database work: acquire late, release early. A transaction ends with `commit()` or `rollback()` explicitly; never hold one across a call to another service.
- Prefer the checked macros (`query!`, `query_as!`) with the offline query data committed, so the build needs no database and a wrong column fails at compile time.
- Map `sqlx::Error` into your own error type at the boundary: not found, unique violation (a conflict), serialization failure or deadlock (retry the whole transaction), pool timeout (overload), everything else (internal).

## Tracing

- Register one subscriber at the start of `main` (`tracing_subscriber` with an `EnvFilter` so `RUST_LOG` controls it; JSON output in production, compact output in development). Libraries emit events; they never install a subscriber.
- Put `#[tracing::instrument(skip_all, fields(...))]` on the functions that matter, and `skip` large or secret arguments (tokens, credentials, request bodies). A span follows the request across awaits, so logs from interleaved tasks stay attributable; attach the request id and organisation id to the root span.
- Log with structured fields (`tracing::info!(run_id = %id, "claimed")`), not formatted strings. Do not log secrets or customer data; ids, versions, numbers and states only. Use `Instrument`/`in_current_span` to carry a span into a spawned task, because `tokio::spawn` does not inherit it.
- Export spans with OpenTelemetry (`tracing-opentelemetry`) when an administrator configures a collector; the server must run correctly with none.

## Testing async code

- `#[tokio::test]` for async tests; `#[tokio::test(start_paused = true)]` plus `tokio::time::advance` to test timeouts and retries without real waiting. Wait on a condition or a channel message, never on a fixed sleep. Test a handler by calling the `Router` as a tower `Service` (`oneshot`), no socket needed.

# V0 contracts

The interfaces the build sessions share, fixed before they start so that none of them invents its own. Decisions are in [V0.md](V0.md). OpenCode facts below were checked against opencode-ai 1.19.6 with a mock provider (20 Nov 2026).

## Ownership

| Session | Owns | Uses |
| :- | :- | :- |
| A, runner | the queue and leases, `/runner/*`, the `antilog runner` command, the OpenCode driver, the runner's `SessionMachine` (trait in `antilog-core/src/machine.rs`), resume of a reclaimed session by `opencode export` and `opencode import` | `SessionCredentials` |
| B, proxy | the `antilog-proxy` binary, the `session_credentials` table, `SessionCredentials`, model keys per organization, metering and the budget stop | spend rows and budgets |
| C, reach | the per-session MCP endpoint, Customize backends for connectors, skills and plugins | `SessionCredentials` |

The `SessionCredentials` trait and its fake are on the base branch, so all three build against it. B writes the Postgres store.

## SessionCredentials (core)

Built: `antilog-core/src/credentials.rs`, with `FakeCredentials` in `fakes/credentials.rs` and the Postgres store on `PgStore` (table `session_credentials`, only the token's SHA-256 is kept; checking and revoking go through security-definer functions because a machine presents a token and no organization). `Purpose` is `Model`, `Mcp` or `Runner`. `mint(session, purpose, ttl)` returns a fresh opaque token and stores only its hash. `verify(token, purpose)` returns the session if the token is live. `revoke_all(session)` ends every token the session holds. The same suite (`gateway_suite::credentials_suite`) runs against the fake and Postgres.

## Queue and runner (A)

- A session's machine state: `queued`, `claimed`, `running`, `idle`, `released`, `done`, `failed`. A claim holds a lease of 60 seconds. Each poll renews it. A lapsed lease puts the session back in `queued`.
- The runner authenticates with a runner key (Antilog cloud: ours; self-hosted later: the organization's environment key) and gets a runner token.
- `POST /runner/poll` is the heartbeat. It renews every lease the runner holds and returns new work plus commands for held sessions: a person's message, an approval reply, abort, release, and a `SessionMachine` command to run.
- A claim returns the session spec: the repos and short-lived clone credentials, the environment, the skill versions with download paths, the generated OpenCode config, the model and MCP tokens, and the model.
- `POST /runner/sessions/{id}/events` takes a batch, idempotent by OpenCode message id and part id.
- `POST /runner/sessions/{id}/results` returns the outcome of a `SessionMachine` command.
- `POST /runner/sessions/{id}/release` marks the session idle, done or failed.
- None of these routes go in the public OpenAPI.

## Driving OpenCode (A)

- Start: `opencode serve --hostname 127.0.0.1 --port <free>` in the clone, with a random `OPENCODE_SERVER_PASSWORD`, `OPENCODE_DISABLE_MODELS_FETCH=1` and `autoupdate: false`.
- Config: the server generates it and the runner writes it to `/etc/opencode/opencode.json` as root. That managed path beats a repo's `opencode.json`. `OPENCODE_CONFIG` does not, so never rely on it. A also checks `OPENCODE_DISABLE_PROJECT_CONFIG` and `--pure` and reports what they block.
- Prompt with `POST /session/{id}/prompt_async`, and interrupt with `POST /session/{id}/abort`.
- Live updates come from `GET /event` (SSE). It has no replay and no event id. After every reconnect, rebuild from `GET /session/{id}/message` (paginated by `before`, cursor in `x-next-cursor`) and de-duplicate by message id and part id.
- Approvals: a `permission.asked` event (also listed at `GET /permission`) raises an approval card. The reply goes to `POST /permission/{id}/reply` with `once` or `reject`.
- A repo may still add MCP servers, skills and plugins of its own inside the machine, as a repo's `.mcp.json` can in Claude Code. They run in the machine and reach out only through the egress allowlist.

## Proxy (B)

Built: `crates/antilog-proxy`, one binary on one listener (`ANTILOG_PROXY_LISTEN`, default `0.0.0.0:8081`).

- Routes `/anthropic/v1/*` (key in `x-api-key`) `/openai/v1/*` (key in `Authorization: Bearer`) and `/opencode/v1/*` (OpenCode Zen, key in `Authorization: Bearer`, upstream `ANTILOG_PROXY_OPENCODE_URL`, default `https://opencode.ai/zen`; every other client header, `x-opencode-*` included, is forwarded; usage is read as Anthropic shape for a `message` reply or `message_*` event and as OpenAI shape otherwise; Zen models are priced from Zen's own list, see Prices below). The proxy verifies the session token with `Purpose::Model`, swaps in the organization's key for that provider and forwards without translation, streaming included. One exception: OpenAI sends a chat stream's usage only when the request sets `stream_options.include_usage`, so the proxy sets that one field when it is absent (otherwise the stream could not be metered). Every other request goes through byte for byte.
- Model keys are per organization per provider (`anthropic`, `openai`, `opencode`), sealed with `Sealer` under the row `model_key:<org>:<provider>`, in the table `model_keys`. The master key is the secret `model-key-master` (64 hex characters, `ANTILOG_SECRET_MODEL_KEY_MASTER` or a mounted file); without it the server and the proxy start but refuse to save or use a key. Organization routes: `GET /model-keys` (any member; providers and when set, never a key), `PUT` and `DELETE /model-keys/{provider}` (owners; audited as `model_key.set` and `model_key.remove`).
- Spend is booked only by the proxy, from the usage in each response: a plain reply's usage object, or a stream's events (Anthropic: input from `message_start`, output from the last `message_delta`; OpenAI: the final usage chunk, or `response.completed`). OpenCode's own counts are not authoritative: an aborted turn reports zero. A stream the session abandons books what was seen, and is logged. Calls that carry no bill (a model list, a token count) pass unmetered.
- Zen prices: `price::ZenPrices` is loaded at start and every hour from `GET <ANTILOG_PROXY_OPENCODE_URL>/v1/models` (`cost.text.input.price`, `.cache.read`, `.cache.write`, `output.price`, US dollars per million tokens), fetched without a key (the list is the same for any key; if the endpoint ever demands one the fetch fails and every Zen model is booked at the highest price). A missing cache price is the input price; `multipliers` (time-of-day surcharges) are ignored in v0, so a surcharged hour is booked at the base rate. A failed fetch keeps the last table and logs once per run of failures; a model not in the table, or any model before the first load, is booked at the highest price in the vendored table and logged.
- Anthropic and OpenAI prices are LiteLLM's price list (MIT), trimmed to Anthropic and OpenAI chat models and vendored in `crates/antilog-proxy/data/prices.json` with its licence and source commit in `prices.header.md`; `price.rs` loads it with `include_str!` (thousandths of a dollar per million tokens, the model as the key). `scripts/update-prices.py` refreshes it (see the crate's README). A dated snapshot takes its model's price. An unknown model is booked at the highest price in the table, part by part, and logged. A call books whole cents, rounded up.
- Before forwarding, the proxy checks the experiment's remaining budget with `budget::ensure_room` (experiment, project cap and organization cap). Once it is spent it answers 402 in the provider's own error shape (`billing_error` for Anthropic, `insufficient_quota` for OpenAI) with a message that says the budget is spent and to call `request_budget`. A session whose organization has no key for the provider gets a 403 that says so; a bad, expired or wrong-purpose token gets a 401. Nothing is forwarded in any of these cases.
- MCP pass-through: `/mcp/*` on the proxy is passed unchanged to the server (`ANTILOG_PROXY_SERVER_URL`), streaming the answer, so a session's machine needs to reach only the proxy. The server checks the session's MCP token. Point `ANTILOG_PUBLIC_URL` (the MCP URL in the generated OpenCode config) at the proxy when the server is not reachable over HTTPS from the machine, as in `scripts/local-run.sh`.
- Egress: a CONNECT forward proxy on the same listener, with the environment's allowlist, without decrypting traffic. The machine's proxy login is the same `Purpose::Model` token as the password (any user name); a missing or bad token gets 407. Allowed: the hosts the environment lists (exact names) and, when it allows them, the package registries in `egress.rs`. Refused (403) and logged: anything else, and any name that leads only to a private, loopback, link-local or metadata address. The allowlist is the environment version the session was started on, read from Postgres through that token (the session's machine row names the id and version) and kept for at most 60 seconds per session. `none` refuses every CONNECT while the model gateway routes keep working; `limited` allows the listed hosts and, when switched on, the package registries; `full` allows any host except a private, loopback or metadata address, which stay refused. A session with no machine, or whose version cannot be read, gets no network. At every level, including `none`, the hosts in `ANTILOG_PROXY_PLATFORM_HOSTS` (comma-separated: the proxy's and the server's public names and the code host) are allowed, because the machine reaches the model gateway, the MCP endpoint and git through this same CONNECT; the private-address check still applies to them. Network secrets need decryption and come after v0; in v0 a service key is a plain environment variable (owner, 20 Nov).
- No key or token is written to a log line; a test reads every line.

## OpenCode config the server generates

```json
{
  "autoupdate": false,
  "small_model": "<the session's model>",
  "provider": {
    "anthropic": {"options": {"baseURL": "<proxy>/anthropic/v1", "apiKey": "<model token>"}},
    "openai":    {"options": {"baseURL": "<proxy>/openai/v1",    "apiKey": "<model token>"}},
    "opencode":  {"options": {"baseURL": "<proxy>/opencode/v1",  "apiKey": "<model token>"}}
  },
  "mcp": {"antilog": {"type": "remote", "url": "<server>/mcp/sessions/<id>", "headers": {"Authorization": "Bearer <mcp token>"}}},
  "skills": {"paths": ["/opt/antilog/skills"]},
  "permission": {"bash": "allow", "edit": "allow"}
}
```

## MCP endpoint and skills (C)

- `/mcp/sessions/{id}`: streamable HTTP with a bearer token for `Purpose::Mcp`. It serves the judge's tools and the switched-on connectors' tools. Each tool is on, off or ask first.
- Skills are downloaded per version from `GET /runner/sessions/{id}/skills/{name}/{version}`. A ships the route, C supplies the content. The runner unpacks them into `/opt/antilog/skills/<name>/`.
- Built (C): the endpoint is `antilog_server::api::mcp`, mounted with `api::router_with_reach(app, credentials, Backend)`, which also mounts the Customize routes. The skills handler is `antilog_server::api::customize::session_skill(deps, session, name, version)`: it answers only for a version the session's experiment selected, with `SkillVersion { files: [{path, content}] }` (JSON text files; no zip in v0), and `session_skill_refs(deps, session)` lists the selected `{name, version}` for the claim. A authenticates the runner and mounts both. Connector credentials are sealed with `Sealer` under `connector:<id>`; the server needs `ANTILOG_SECRET_MASTER_KEY` (64 hex) to hold one. `ANTILOG_CONNECTORS_ALLOW_PRIVATE=1` lets a connector sit on a loopback address, for local development and tests only.

## The sealed test command (judge)

The judge runs the sealed command through `SessionMachine::run`. A run on evaluation data happens in the session's machine. A run with `data.holdout` true happens in a second machine (`runner/machine.rs::run`): the runner starts it through the same `Sandbox` from the session's `EnvironmentSpec` (same image and setup snapshot, same limits, same proxy login and network rules), named `antilog-<session>-<command id>` with its own network, clones the session's repos from the code host (never copied from the agent's machine), restores the protected files, runs the command, and removes the machine and its network on success, failure or a clone that fails. Only that machine gets the holdout run; the live machine sees no exec of it. The machine belongs to the session, so it is not a second session for any cap. Its last output line is the measurement:

```json
{"metric": 123.4, "guardrails": {"p99_ms": 210, "errors": 0}}
```

Base and trial runs are interleaved by the judge (`judge/paired.rs`). The number comes from this line, never from the agent's report.

## As built (session A)

- A machine row is its own table (`machines`), not columns on `sessions`. Commands wait in `machine_commands` until a runner takes them; a run's outcome is stored on its command; an export is replaced on each release (`machine_exports`).
- `POST /runner/poll` also takes `held` (the sessions the runner runs now, answered by `lost`) and `done` (command ids it finished). Release outcomes `idle`, `done` and `failed` leave the machine `released`, `done` and `failed`.
- `GET /runner/sessions/{id}/export` serves the last export for `opencode import`. `GET /runner/sessions/{id}/skills/{name}/{version}` serves the skill versions the session's experiment selected (Customize's `session_skill`; 404 for any other); a claim's skills list is that selection; a pack is JSON, `{files: [{path, content}]}` (session C's `SkillVersion`), written to `/opt/antilog/skills/<name>/<path>`; a path that escapes the folder skips the pack.
- An OpenCode permission ask becomes an Ask first card (`ask_first`), added to the card kinds. Bash and edit are allowed in the config (the machine is isolated), so only another tool asks.
- A decided card reaches the agent: `settle` records its outcome as an agent-kind note in the conversation (key `card:<approval id>`, the same kind as the machine-time note) and queues a `message` command with the same words, waking a released machine first. Both callers of `settle` (a person's decision, policy approval) therefore tell it, once, because `mark_applied` guards each card.
- An approved Attach repo adds the repo to the machine's seed (`RunnerStore::add_seed_repo`, so a later claim or resume clones it) and, while a runner holds the machine, queues `attach_repo`. The queue holds only the repo name; `POST /runner/poll` fills in the clone address from the server's clone base. The runner clones it beside the others if it is not there; a failed clone is logged, not fatal.
- The judge's data (`ANTILOG_DATA_KIND`, `ANTILOG_DATASET_ID`, `ANTILOG_DATASET_VERSION`) is named to the command; delivering the rows is not built.
- `--pure` stops a repo's plugin code from running; `OPENCODE_DISABLE_PROJECT_CONFIG` also stops its MCP servers, skills and agents. Neither is needed to keep a repo from overriding the provider, permissions or autoupdate: the managed config wins. See [../.claude/skills/session/runner.md](../.claude/skills/session/runner.md).

## Stored secrets (B and C)

Customer model keys and connector credentials are encrypted in Postgres with `chacha20poly1305` (XChaCha20-Poly1305 with a random nonce for each value). The master key comes from the existing `Secrets` store, never from the database. Built: `Sealer` in `antilog-core/src/sealed.rs` (model keys use it through `PgStore::with_sealer`, from the secret `model-key-master`) (`seal(plain, row)` and `open(stored, row)`; `row` is associated data that binds a value to its row, such as `model_key:<org>:anthropic`). B and C both use it (owner, 20 Nov).

## MCP transport (C)

The per-session endpoint uses rmcp's streamable HTTP server feature on the existing `rmcp` dependency (owner, 20 Nov).

## Environments and the session machine (D and E)

`EnvironmentSpec` and `NetworkLevel` are in `antilog-types/src/runner.rs`. Every change to an environment makes a new version.

- **E owns (built):** environments as data (types, store, migration `20261120700000_environments.sql`, routes `/environments`, Settings, Connections with the warning that variables are readable by the agent), filling `SessionSpec.environment: EnvironmentSpec` from the version the session started on (`Seed.environment` and `Seed.environment_version`, pinned by `session::start`; none is the built-in default), and the proxy reading each session's allowlist from Postgres through its `Purpose::Model` token. `ANTILOG_PROXY_ALLOWED_HOSTS` is gone.
- **D owns:** the runner's `Sandbox` trait (start, stop, run a command), with the plain-process backend kept for tests and a container backend that drives the `docker` command line (no new crate). The container gets the environment's CPU and memory limits, and its only route out is the proxy (an internal Docker network, with `HTTPS_PROXY` set). The setup script runs once per (`id`, `version`) and the result is kept as an image tagged with both, dropped after 7 days unused. D also owns the organization lock: a runner locks to the organization of its first claim, enforced by the server at claim time.
- **Built (D):** `runner/sandbox.rs`, `runner/container.rs`, `session-image/Dockerfile` and the organization lock (migration `20261120600000_runner_org_lock.sql`, `runner_locks`, `RunnerStore::unlock_runner`, `PollRequest.exiting`); see `.claude/skills/session/runner.md`.
- **Built (limits, migration `20261120800000_session_machine_limits.sql`):** the runner lock records `polled_at` and lapses after 10 minutes without a poll; `--disk-gb` (20 GiB default) per container where the storage driver allows; a claim holds a machine for at most 24 hours (`runner::Settings.max_lifetime`, `ANTILOG_MAX_SESSION_HOURS`), then the poll sends `release` `idle` and says why in the conversation; `organizations.max_running_sessions` (10) caps claimed, running and idle machines, set by `PUT /organization/limits` (`maxRunningSessions`, owners). `RunnerStore::mark_overdue` is the one new store call.
- E has landed: `SessionSpec.environment` is an `EnvironmentSpec`; D's `SandboxSpec` takes the whole environment from it (limits, network, variables, setup script), with no second set of fields.

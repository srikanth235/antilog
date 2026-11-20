# V0 contracts

The interfaces the build sessions share, fixed before they start so that none of them invents its own. Decisions are in [V0.md](V0.md). OpenCode facts below were checked against opencode-ai 1.19.6 with a mock provider (20 Nov 2026).

## Ownership

| Session | Owns | Uses |
| :- | :- | :- |
| A, runner | the queue and leases, `/runner/*`, the `antilog runner` command, the OpenCode driver, the runner's `SessionMachine` (trait in `antilog-core/src/machine.rs`), resume of a reclaimed session by `opencode export` and `opencode import` | `SessionCredentials` |
| B, proxy | the `antilog-proxy` binary, the `session_credentials` table, `SessionCredentials`, model keys per organization, metering and the budget stop | spend rows and budgets |
| C, reach | the per-session MCP endpoint, Customize backends for connectors, skills and plugins | `SessionCredentials` |

The `SessionCredentials` trait and its fake land on the base branch before A, B and C start, so all three build against it. B writes the Postgres store.

## SessionCredentials (core)

```rust
pub enum Purpose { Model, Mcp, Runner }
#[async_trait]
pub trait SessionCredentials: Send + Sync {
    /// A fresh opaque token; only its hash is stored.
    async fn mint(&self, session: SessionId, purpose: Purpose, ttl: Duration) -> Result<String>;
    /// The session the token belongs to, if it is live and for this purpose.
    async fn verify(&self, token: &str, purpose: Purpose) -> Result<Option<SessionId>>;
    async fn revoke_all(&self, session: SessionId) -> Result<()>;
}
```

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

- Routes `/anthropic/v1/*` (key in `x-api-key`) and `/openai/v1/*` (key in `Authorization: Bearer`). The proxy verifies the session token with `Purpose::Model`, swaps in the organization's key for that provider and forwards without translation.
- Spend is booked only by the proxy, from the usage in each response, including the last event of a stream. OpenCode's own counts are not authoritative: an aborted turn reports zero.
- Before forwarding, the proxy checks the experiment's remaining budget. Once it is spent, it refuses with an error the agent can read.
- Egress: a CONNECT forward proxy with the environment's allowlist, without decrypting traffic. Network secrets need decryption and come after v0; in v0 a service key is a plain environment variable (owner, 20 Nov).

## OpenCode config the server generates

```json
{
  "autoupdate": false,
  "provider": {
    "anthropic": {"options": {"baseURL": "<proxy>/anthropic/v1", "apiKey": "<model token>"}},
    "openai":    {"options": {"baseURL": "<proxy>/openai/v1",    "apiKey": "<model token>"}}
  },
  "mcp": {"antilog": {"type": "remote", "url": "<server>/mcp/sessions/<id>", "headers": {"Authorization": "Bearer <mcp token>"}}},
  "skills": {"paths": ["/opt/antilog/skills"]},
  "permission": {"bash": "ask", "edit": "allow"}
}
```

## MCP endpoint and skills (C)

- `/mcp/sessions/{id}`: streamable HTTP with a bearer token for `Purpose::Mcp`. It serves the judge's tools and the switched-on connectors' tools. Each tool is on, off or ask first.
- Skills are downloaded per version from `GET /runner/sessions/{id}/skills/{name}/{version}`. A ships the route, C supplies the content. The runner unpacks them into `/opt/antilog/skills/<name>/`.

## The sealed test command (judge)

The judge runs the sealed command through `SessionMachine::run` in the session's machine. Its last output line is the measurement:

```json
{"metric": 123.4, "guardrails": {"p99_ms": 210, "errors": 0}}
```

Base and trial runs are interleaved by the judge (`judge/paired.rs`). The number comes from this line, never from the agent's report.

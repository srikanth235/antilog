# Organization: what exists today, and what is target

Today Ratchet has one user: the person on the machine that runs the daemon. There is no organization, project, member, role, sign-in provider, access-token store, service account or audit log in the repo. Every path below was checked to exist.

## How the daemon authenticates now

- The daemon `ratchetd` serves HTTP on a unix socket created with mode 0600 inside a private directory, and removes a stale socket but not a live daemon's (`crates/ratchetd/src/server.rs`, `bind`).
- At every start it generates a fresh 256-bit token and saves it through a `TokenStore` (`crates/ratchetd/src/auth.rs`). The only store is `FileTokenStore`, a file with mode 0600 next to the socket. A Keychain store is a TODO in that file.
- Every route needs `Authorization: Bearer <token>`. A missing, wrong or almost-right token gets 401 with `WWW-Authenticate: Bearer`. Comparison is constant-time (`constant_time_eq`).
- The token is stored as plaintext in that file. It is a per-boot owner credential, not a hashed, revocable access token.
- MCP session tokens are a second kind. They open `/v1/mcp/session` and `/v1/mcp/call` and nothing else, are bound to one tool set and to one repo or optimization, are issued per chat or per local agent, and are revoked when it ends. The owner's token does not open those two routes (`crates/ratchetd/src/mcp/sessions.rs`, `crates/ratchetd/src/mcp/routes.rs`, `docs/ratchet-mcp.md`).
- Tests: `crates/ratchetd/tests/api/auth.rs` (every route needs the token, scheme checked, socket and token file private, a second daemon cannot take a live socket, shutdown cleans up) and `crates/ratchetd/tests/api/mcp.rs`.
- The wire contract is `api/openapi.yaml`, checked by `crates/ratchetd/tests/api/openapi.rs` and described in `api/CONTRACT.md`.

## Other pieces that exist

| Piece | Where |
| --- | --- |
| Secrets kept out of settings files; Keychain service `com.ratchet.app`; refusal to store in plaintext | `crates/ratchet-connectors/src/secrets.rs` |
| Responses report secrets as set or not, never the value | `crates/ratchetd/tests/api/contract.rs` |
| Live events: server-sent events with ids and resume by `Last-Event-ID`, `hello` and `resync` | `crates/ratchetd/src/api/events.rs`, `crates/ratchetd/tests/api/sse.rs` |
| GitHub sign-in by device flow, for access to repos, not for signing in to Ratchet | `crates/ratchet-github/src/signin.rs` |
| The ledger, the one store; no organization column | `crates/ratchet-ledger/migrations/0001_init.sql` |
| The CLI as a client of the daemon | `crates/ratchet-cli/src/` |
| CI for the daemon and the CLI | `.github/workflows/native.yml` |

## Gaps in what exists

- No role: whoever holds the owner token may call every route.
- No actor on writes. The ledger records what happened, not who did it.
- Writes are not funneled through an action layer. Handlers in `crates/ratchetd/src/api/` call the service directly.
- No audit log. Spend events and ratchet moves are append-only by trigger, but they are not an audit log.

## Target, not built

- Organizations and projects, with an organization on every row.
- Members and the roles owner, editor and viewer on a project.
- OpenID Connect sign-in: GitHub and Google on the hosted service, the customer's identity provider when self-hosted. No passwords.
- Access tokens and service accounts, shown once and stored as a hash.
- One action layer for permission, audit and events.
- The audit log.
- Notifications. The native Mac app that showed them was removed. The target is [notifications.md](notifications.md).
- A test that a self-hosted server contacts only configured endpoints.

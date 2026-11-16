# Organization: what the earlier code did, and what is target

The earlier daemon and CLI (branch `claude/peaceful-lamport-leak6p`) had one user: the person on the machine that ran the daemon. They are reference, not a base. There was no organization, member, role, sign-in provider, access-token store, service account, approval or audit log. Nothing here exists on this branch. The paths are in that earlier code.

## How the daemon authenticated

- The daemon `ratchetd` served HTTP on a unix socket created with mode 0600 inside a private directory, and removed a stale socket but not a live daemon's (`crates/ratchetd/src/server.rs`, `bind`).
- At every start it generated a fresh 256-bit token and saved it through a `TokenStore` (`crates/ratchetd/src/auth.rs`). The only store was `FileTokenStore`, a file with mode 0600 next to the socket. A Keychain store was a TODO in that file.
- Every route needed `Authorization: Bearer <token>`. A missing, wrong or almost-right token got 401 with `WWW-Authenticate: Bearer`. Comparison was constant-time (`constant_time_eq`).
- The token was stored as plaintext in that file. It was a per-boot owner credential, not a hashed, revocable access token.
- MCP session tokens were a second kind. They opened `/v1/mcp/session` and `/v1/mcp/call` and nothing else, were bound to one tool set and to one repo, were issued per chat or per local agent, and were revoked when it ended. The owner's token did not open those two routes (`crates/ratchetd/src/mcp/sessions.rs`, `crates/ratchetd/src/mcp/routes.rs`, `docs/ratchet-mcp.md`).
- Tests: `crates/ratchetd/tests/api/auth.rs` (every route needs the token, scheme checked, socket and token file private, a second daemon cannot take a live socket, shutdown cleans up) and `crates/ratchetd/tests/api/mcp.rs`.
- The wire contract was `api/openapi.yaml`, checked by `crates/ratchetd/tests/api/openapi.rs` and described in `api/CONTRACT.md`.

## Other pieces the earlier code had

| Piece | Where |
| --- | --- |
| Secrets kept out of settings files; Keychain service `com.ratchet.app`; refusal to store in plaintext | `crates/ratchet-connectors/src/secrets.rs` |
| Responses reported secrets as set or not, never the value | `crates/ratchetd/tests/api/contract.rs` |
| Live events: server-sent events with ids and resume by `Last-Event-ID`, `hello` and `resync` | `crates/ratchetd/src/api/events.rs`, `crates/ratchetd/tests/api/sse.rs` |
| GitHub sign-in by device flow, for access to repos, not for signing in to Ratchet | `crates/ratchet-github/src/signin.rs` |
| The ledger, the one store; no organization column | `crates/ratchet-ledger/migrations/0001_init.sql` |
| The CLI as a client of the daemon | `crates/ratchet-cli/src/` |
| CI for the daemon and the CLI | `.github/workflows/native.yml` |

## What it taught

- Whoever held the owner token could call every route. A role check needs its own table and a route-walk test.
- The ledger recorded what happened, not who did it. An actor must be on every write from the start.
- Handlers in `crates/ratchetd/src/api/` called the service directly. One action layer is cheaper than retrofitting one.
- Spend events and ledger moves were append-only by trigger, but they were not an audit log.
- Secrets reported as set or not, never as a value, held up under test. Keep that.
- The earlier code set agents up through settings and wizards. The target replaces that with approval cards in the chat ([approvals.md](approvals.md)).

## Target, not built

- Organizations with an organization on every row, roles owner, editor and viewer that are the default in every project, and projects that can set a member's role for themselves.
- OpenID Connect and GitHub sign-in: GitHub and Google on Ratchet cloud, the customer's identity provider when self-hosted. No passwords.
- Access tokens and service accounts, shown once and stored as a hash.
- Approvals, policy, budgets and the monthly cap ([approvals.md](approvals.md)).
- One action layer for permission, audit and events.
- The audit log.
- Notifications ([notifications.md](notifications.md)). The earlier native Mac app that showed them was removed.
- A test that a self-hosted server contacts only configured endpoints.

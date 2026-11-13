# An API endpoint

`api/openapi.yaml` is generated from the Rust types and is the one description of the wire format. The full contract and its checks are in `api/CONTRACT.md` (on the earlier code branch, see `AGENTS.md`). Read it first.

## Steps today

1. **Edit the Rust types.** Request and response types live in `crates/ratchetd/src/api/`. The handler lives in `crates/ratchetd/src/routes/` and carries its `#[utoipa::path]`. The handler is thin: it decodes, calls one method of the `AppService` trait (`crates/ratchetd/src/service.rs`) and encodes the answer. Implement the method for the simulation (`crates/ratchetd/src/sim/`) and for the real service (`crates/ratchetd/src/real/`).
2. **Regenerate the contract.** `cargo run -p ratchetd -- openapi` rewrites `api/openapi.yaml` (`--check` exits 1 when it differs).
3. **Contract tests and snapshots.** `cargo test -p ratchetd` fails when the committed file is stale and round-trips every route (`crates/ratchetd/tests/api/`). Review the diff of `api/openapi.yaml` and of the insta snapshots with `cargo insta review`.
4. **CLI.** Add the command in `crates/ratchet-cli/src/cli.rs` and `crates/ratchet-cli/src/commands/`. Every command takes `--json`. A command that spends or changes things needs `--yes` and previews without it. Add the command name to the list in `crates/ratchet-cli/tests/contract.sh`.
5. **MCP tool.** `ratchet mcp` generates one tool per CLI command from the clap definition (`crates/ratchet-cli/src/mcp.rs`), so step 4 normally creates the tool. The chat's own tools are in `crates/ratchetd/src/mcp/tools.rs` and call the same `AppService` methods.
6. **TypeScript client (target).** The web app's client is generated from the same `api/openapi.yaml` with `openapi-typescript` and `openapi-fetch`. Nothing generates it today.
7. **Docs.** Fix `api/CONTRACT.md` if a convention changed, and the CLI help text.

## Wire conventions

Summarised from `api/CONTRACT.md`; that file wins.

- Property names are camelCase. Ids are strings. Times are epoch milliseconds unless a field says otherwise.
- Nullable response fields are required and carry `null` (`#[schema(required = true)]` on every `Option` in a response). Inputs and query structs stay optional.
- Errors are `{ "code", "message", "fields" }` with the usual statuses.
- Live updates are server-sent events on `GET /v1/events` (see [live-event.md](live-event.md)).
- Every route needs the bearer token (`crates/ratchetd/src/auth.rs`).
- No response, event or error carries a secret (see [product promises](../product/promises.md)).

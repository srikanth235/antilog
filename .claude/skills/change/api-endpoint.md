# An API endpoint

The OpenAPI document is generated from the Rust types in `ratchet-server` and is the one description of the wire format. The web client, the CLI and the contract tests all read it. Never edit it by hand. (target: nothing here is built yet.)

## Steps (target)

1. **Decide the product call first.** An endpoint that exposes a new thing is a product change; see [product](../product/SKILL.md). An endpoint never returns data from a customer's systems: the server stores only ids, versions, numbers, states and audit rows.
2. **Write the types.** Request and response types live in `crates/ratchet-server/src/api/` and derive their schema with utoipa. Follow [stack/rust/ratchet.md](../stack/rust/ratchet.md).
3. **Write the handler.** `crates/ratchet-server/src/routes/`, with `#[utoipa::path]`. The handler is thin: it decodes, calls the action layer (a write, see [write-action.md](write-action.md)) or one read function, and encodes the answer. It tests no role and writes no audit row.
4. **Regenerate the document.** `cargo xtask openapi` writes the committed document; `cargo xtask openapi --check` fails when it differs (target commands).
5. **Regenerate the TypeScript client.** The web app's client is generated from the document (`openapi-typescript` and `openapi-fetch`, see [stack/libraries.md](../stack/libraries.md)). Never edit the output. See [stack/typescript/ratchet.md](../stack/typescript/ratchet.md).
6. **CLI command.** Add it in the `ratchet` program. Every command takes `--json`. A command that spends or changes things needs `--yes` and previews without it. Add it to the CLI contract list.
7. **MCP tool.** `ratchet mcp` makes one tool per CLI command, so step 6 normally creates the tool. A tool that needs `--yes` takes `confirm: true`.
8. **The macOS app.** If the Mac app shows it, it reads the same document (see [stack/swift/ratchet.md](../stack/swift/ratchet.md)).
9. **Tests.** A contract test that every route in the document has a case and its answer parses into its type; insta snapshots of the document and of the event union; a test that the route is refused without a token and as a role the matrix denies. See [../land/tests.md](../land/tests.md).
10. **Docs.** Fix the wire conventions below if one changed, the CLI help text and the glossary if a word is new.

## Wire conventions (target)

These are the conventions the earlier code settled on; keep them unless the owner rules otherwise.

- Property names are camelCase. Ids are strings. Times are epoch milliseconds unless a field says otherwise.
- Nullable response fields are required and carry `null`. Inputs and query structs stay optional.
- Errors are `{ "code", "message", "fields" }` with the usual statuses.
- Live updates are server-sent events (see [live-event.md](live-event.md)).
- Every route needs a signed-in member or an access token, and every route is permission-checked.
- No response, event or error carries a secret or customer data (see [product promises](../product/promises.md)).

Facts from the earlier daemon's contract are in [today.md](today.md).

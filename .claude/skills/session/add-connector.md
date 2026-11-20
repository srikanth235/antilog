# Add a connector

A connector is a remote MCP server a session may call: a data system's MCP server, an internal tool, a SaaS server. In v0 it is only that ([docs/V0.md](../../../docs/V0.md)). The code host (GitHub, GitLab) is how repos attach and pull requests open, and is added as a code host, not a connector. A cloud account is not a connector.

Connectors live in Customize, Connectors (boards `Connectors`, `ConnectorsEmpty`, `ConnectorDetail`, `AddConnector`, `AddConnectorMcp`, `ConnectorError`; layout in [../design/behaviour.md](../design/behaviour.md)). Built: `antilog-core/src/customize/connectors.rs` (and `reach.rs` for what a session sees), the routes in `antilog-server/src/api/customize.rs`, the client in `antilog-server/src/connectors.rs`.

1. **The server calls it, the machine never does.** A session has one MCP endpoint, `/mcp/sessions/{id}`. It serves the Antilog tools and the tools of every connector switched on for the experiment, each named `<connector>__<tool>`. The server forwards each call to the connector and returns the text. The machine holds no address and no credential of the connector.
2. **Add it with a URL and an optional credential.** The URL is https and names a public host: a literal private address, `localhost`, an internal name or a redirect is refused (`check_url`). The credential is sent to the connector as a bearer token.
3. **The credential is sealed.** It is stored with `Sealer` (XChaCha20-Poly1305, bound to the row `connector:<id>`), opened only for the one call or check, and never returned by any route, event or log. Without a master key (`ANTILOG_SECRET_MASTER_KEY`) a connector can be added without a credential only.
4. **Adding runs a check.** The server asks the connector for its tools. The status is Connected, Reconnect or Not checked; an unchecked connector or one that did not answer is never shown as healthy, and keeps the tools it had. `ConnectorError` says "Can't read the credential for NAME." and offers Check again and Update credential.
5. **Each tool is On, Off or Ask first.** A tool seen for the first time asks first. Off is not offered to a session and a call to it is refused and audited. Ask first raises a Use a connector tool card (`use_connector_tool`, through `approval.rs`), and one approval pays for one call with exactly the input approved; policy may pre-approve the card kind like any other except Add credential rules. On calls go straight through.
6. **Chosen per experiment.** An editor or owner of the project switches connectors on for an experiment, on the experiment page. A connector an experiment has switched on cannot be removed.
7. **Audit.** Adding, checking, changing a tool's mode, replacing the credential, removing and every call (`connector.use`, Done or Denied) are audit rows with an actor. The conversation keeps only the tool's name, never its input or answer.
8. **Roles.** Owners add, check, change and remove. Every member reads (never a credential).
9. **A plugin may bring one.** It arrives without a credential and Not checked; an owner adds the credential on the connector.
10. **Tests.** Against a mock MCP server (`antilog-server/tests/reach.rs`) and a scripted fake (`FakeConnectors`): the credential never leaves the server, an Off tool is never forwarded, an Ask first tool waits for a card and then runs once, a down connector is Reconnect, a private address is refused.

## Not in v0

The earlier design of connectors as data systems with an agent credential and a judge credential, datasets under them and a holdout marking is not built, and nothing above relies on it. A connector reaches an agent only through the server, so what it returns can still end up in the agent's branch or messages: say so where a connector is added ([threat-model.md](threat-model.md)). Network secrets that the proxy attaches without the key entering the machine come after v0.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

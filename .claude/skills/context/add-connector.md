# Add a connector

A connector is a connection to an outside system that an agent's code reads through: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, S3, GitHub, GitLab or an MCP server. Today the nearest thing is a signed-in account connector (`crates/ratchet-connectors/src/model.rs`, `ConnectorKind`).

1. **One translator.** Write one module that turns the system's vocabulary into datasets and nothing else. The system's words (schema names, project ids, query dialect) stop in that module. (target)
2. **Say how to make a scoped credential.** The connector says how to make a read-only credential for its system that leaves the holdout out (a read-only role, view or policy), and a separate one for the evaluator that includes it. It reaches a session only through the platform's credential store. A credential with write rights is refused. Whether credentials are minted per run or registered once by the customer is open (see [rulings](../product/rulings.md)); write the connector so either works. (target)
3. **No secret in a response or log.** A credential lives in the platform's credential store, never in a prompt, a repo file, settings, a response, an event or a log. Whether Ratchet ever holds a connector admin secret is open (see [rulings](../product/rulings.md)); on Ratchet cloud it may not. Today the secret is in the Keychain or a 0600 file (`crates/ratchet-connectors/src/secrets.rs`).
4. **Probe.** Connect runs a probe and reports signed out, expired, invalid or ready, as connectors do today (`ConnectorStatus` in `crates/ratchet-connectors/src/model.rs`, `crates/ratchet-connectors/src/validate.rs`).
5. **Name datasets, never read them.** The connector's output to Ratchet is a dataset or a virtual dataset with an id and versions. The agent's code reads through the connector with the scoped credential. The server never calls the connector. (target)
6. **Data stays put.** Reads run in a session's sandbox on an agent platform, or in an evaluator sandbox, removed when the session ends (see [../infra/SKILL.md](../infra/SKILL.md)), never through the Ratchet server. (target)
7. **No transform.** The connector reads tables and files as they are. The customer prepares data in their own warehouse.
8. **Write-back is separate.** If the system can be written to (a code host, a ticket system), writing is a named write-back action requested by optimization, not a method of the connector. See the pull request path in `crates/ratchet-github/src/github_host.rs`.
9. **Audit.** Connect, disconnect, credential and grant changes are named actions with an actor (see [../organization/SKILL.md](../organization/SKILL.md)). (target)
10. **Tests.** Add a mock connector and a contract test in the style of `crates/ratchet-connectors/tests/connectors.rs`: probe states, secret never returned, scoped credential read-only, holdout read refused.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

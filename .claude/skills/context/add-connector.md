# Add a connector

A connector is a connection to an outside system that an agent's code reads through: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, S3, GitHub, GitLab or an MCP server. Today the nearest thing is a signed-in account connector (`crates/ratchet-connectors/src/model.rs`, `ConnectorKind`).

1. **One translator.** Write one module that turns the system's vocabulary into datasets and nothing else. The system's words (schema names, project ids, query dialect) stop in that module. (target)
2. **Mint a scoped credential.** The connector says how to mint, for one run, a short-lived, read-only credential for its system that leaves the holdout out (a read-only role, view or policy), and a separate one for the evaluator that includes it. A credential with write rights is refused. (target)
3. **Admin secret stays out of the server.** The secret the launcher uses to mint credentials lives in the customer's secret manager (Ratchet-managed: our encrypted secret store, held by the managed runner) and never in settings, a response, an event, a log or the server. Today the secret is in the Keychain or a 0600 file (`crates/ratchet-connectors/src/secrets.rs`).
4. **Probe.** Connect runs a probe and reports signed out, expired, invalid or ready, as connectors do today (`ConnectorStatus` in `crates/ratchet-connectors/src/model.rs`, `crates/ratchet-connectors/src/validate.rs`).
5. **Name datasets, never read them.** The connector's output to Ratchet is a dataset or a virtual dataset with an id and versions. The agent's code reads through the connector with the scoped credential. The server never calls the connector. (target)
6. **Data stays put.** Reads run in a sandbox, in the customer's infrastructure or, if an owner chooses Ratchet-managed, in a Ratchet sandbox removed at the end of the run (see [../infra/SKILL.md](../infra/SKILL.md)), never through the Ratchet server. (target)
7. **No transform.** The connector reads tables and files as they are. The customer prepares data in their own warehouse.
8. **Write-back is separate.** If the system can be written to (a code host, a ticket system), writing is a named write-back action requested by optimization, not a method of the connector. See the pull request path in `crates/ratchet-github/src/github_host.rs`.
9. **Audit.** Connect, disconnect, mint and grant changes are named actions with an actor (see [../organization/SKILL.md](../organization/SKILL.md)). (target)
10. **Tests.** Add a mock connector and a contract test in the style of `crates/ratchet-connectors/tests/connectors.rs`: probe states, secret never returned, minted credential read-only, holdout read refused.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

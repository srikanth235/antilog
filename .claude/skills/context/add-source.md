# Add a source

A source is a read-only connection to an outside system: Snowflake, BigQuery, Postgres, object storage, GitHub, GitLab, monitoring or an MCP server. Today the nearest thing is a connector (`crates/ratchet-connectors/src/model.rs`, `ConnectorKind`).

1. **One translator.** Write one module that turns the system's vocabulary into datasets and nothing else. The system's words (schema names, project ids, query dialect) stop in that module. (target)
2. **Read-only credential.** The source asks the customer for a credential that can only read. A credential with write rights is refused at connect time. (target)
3. **Credential storage.** The secret lives in the Keychain or a 0600 file and never in settings, a response, an event or a log. Reuse `crates/ratchet-connectors/src/secrets.rs`.
4. **Probe.** Connect runs a probe and reports signed out, expired, invalid or ready, as connectors do today (`ConnectorStatus` in `crates/ratchet-connectors/src/model.rs`, `crates/ratchet-connectors/src/validate.rs`).
5. **Produce datasets, not reads.** The source's only output is a dataset or a virtual dataset with versions. An experiment never calls the source. (target)
6. **Data stays put.** Reads run on a machine in the customer's account (see [../infra/SKILL.md](../infra/SKILL.md)), never through the Ratchet server. (target)
7. **No transform.** The source reads tables and files as they are. The customer prepares data in their own warehouse.
8. **Write-back is separate.** If the system can be written to (a code host, a ticket system), writing is a named write-back action requested by optimization, not a method of the source. See the pull request path in `crates/ratchet-github/src/github_host.rs`.
9. **Audit.** Connect, disconnect and grant changes are named actions with an actor (see [../organization/SKILL.md](../organization/SKILL.md)). (target)
10. **Tests.** Add a mock source and a contract test in the style of `crates/ratchet-connectors/tests/connectors.rs`: probe states, secret never returned, read-only credential refused.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

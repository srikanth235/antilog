# Add a connector

A connector is a data system or MCP server an agent's code may read: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, Amazon S3 or any MCP server. The code host (GitHub, GitLab) is how repos attach and pull requests open, and is added as a code host, not a connector. Nothing like this is built on this branch; the nearest earlier thing is in [today.md](today.md). All steps are target.

1. **Say how to make a scoped credential.** The connector says how to make a read-only credential for its system that leaves the holdout out (a read-only role, view or policy), and a separate one for the judge that includes it. The customer makes both and registers them once; the connector page tells them how. A credential with write rights is refused.
2. **A card, not a page.** A session asks for a credential with `request_credential`. A person types the value into the secure form. It goes straight to the platform's credential store, and Ratchet keeps only a credential reference (see [tools.md](tools.md)).
3. **No secret in a response or log.** A credential lives in the platform's credential store, never in a prompt, a chat message, a repo file, a response, an event or a log. Ratchet holds no connector admin secret.
4. **Probe.** Adding it runs a probe and reports signed out, expired, invalid, ready or not checked, and never shows an unchecked one as healthy.
5. **Name the data, never read it.** A contract names its data (the evaluation data and the holdout) by contract; see [../judge/model.md](../judge/model.md). The agent's code reads through the connector with the scoped credential. The server never calls the connector and stores no rows.
6. **Data stays put.** Reads run in a session's sandbox on an agent platform, or in a judge sandbox, removed when it ends, never through the Ratchet server. The network policy lets a session reach only its model, its connectors and the code host.
7. **No transform.** The connector reads tables and files as they are. The customer prepares data in their own warehouse.
8. **Revoke.** Settings, Connections lists the approved credentials with Revoke. Revoking removes the value from the platform's credential store.
9. **Audit.** Adding, using and revoking a credential are named actions with an actor (see [../organization/SKILL.md](../organization/SKILL.md)).
10. **Tests.** Add a mock connector and a contract test: probe states, secret never returned, scoped credential read-only, holdout read refused for an agent's credential.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

# Add a connector

A connector is a data system or MCP server an agent's code may read: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, Amazon S3 or any MCP server. The code host (GitHub, GitLab) is how repos attach and pull requests open, and is added as a code host, not a connector. A cloud account is not a connector. Nothing like this is built on this branch; the nearest earlier thing is in [today.md](today.md). All steps are target.

Connectors live in Settings, Connectors (boards `SettingsConnectors`, `SettingsConnectorsEmpty`, `ConnectorDetail`, `AddConnector`, `AddConnectorMcp`, `ConnectorError`, `ConnectorCatalogue`, `DatasetHoldout`). A connector is added from there with "Add connector", or from the Add credential card in a chat.

1. **One translator.** Write one module that turns the system's vocabulary into datasets and nothing else. The system's words (schema names, project ids, query dialect) stop in that module. A second translator for the same system forks the meaning of a column.
2. **Give a recipe for each of two credentials.** The connector says how to make, in its own system:
   - the **agent credential**: read-only, on the views the person lists, holdout tables not granted;
   - the **judge credential**: a separate read-only credential that also covers the holdout.

   The customer makes both and registers them once. A credential with write rights is refused. The recipes are short and per system, shown on the Add connector page and on the Add credential card:

   | System | Agent credential | Judge credential |
   | --- | --- | --- |
   | Snowflake | A read-only role granted SELECT on the views you list; holdout tables not granted | A separate read-only role that also covers the holdout |
   | Databricks | A read-only service principal with SELECT on the views you list | A separate service principal that also covers the holdout |
   | BigQuery | A service account with the data viewer role on the views you list, not on the holdout dataset | A separate service account that also reads the holdout dataset |
   | Postgres | A role with SELECT on the views you list; no grant on the holdout schema | A separate role that also reads the holdout schema |
   | DynamoDB | A role allowed read actions on the tables you list only | A separate role that also reads the holdout table |
   | Amazon S3 | A role allowed to read the prefixes you list; the holdout prefix denied | A separate role that also reads the holdout prefix |
   | MCP server | A token that may call only the read-only tools you list | A separate token that may also call the tools that reach the holdout |

3. **The Add connector page.** It asks for the per-system fields (for example account and warehouse for Snowflake; the server address and the tools to allow for an MCP server), then two credentials, each with its recipe, a secure field and a "What each one gets" block: agents read the views listed, holdout excluded; the judge reads those and the holdout. Values go straight to the platform's credential store.
4. **A card in the chat.** A session asks for a credential with `request_credential` (see [tools.md](tools.md)). The Add credential card is connector-aware: it shows the connector's logo, the recipe for the read-only, holdout-excluding credential, "What each one gets: agents / the judge", and a "Browse connectors" link that opens the catalogue. The value is typed into the secure form, never the chat. Ratchet keeps only a credential reference.
5. **The catalogue.** `ConnectorCatalogue` is a picker dialog with a grid of the connectors and their logos (Snowflake, Databricks, BigQuery, Postgres, DynamoDB, Amazon S3, MCP server), opened from "Add connector" and from the Add credential card. No "Featured" and no "For you".
6. **No secret in a response or log.** A credential lives in the platform's credential store, never in a prompt, a chat message, a repo file, a response, an event or a log. Ratchet holds no connector admin secret. Credentials are short-lived and minted per session.
7. **Check.** Adding a connector runs a check and reports Connected, Reconnect or Not checked. An unchecked connector is never shown as healthy. When a credential cannot be read, `ConnectorError` says "Can't read the credential for NAME.", what to do, shows its datasets as "Not checked" (never empty), names the experiments that cannot measure until it is fixed, and offers "Check again" and "Update credential".
8. **Name the data, never read it.** An experiment names its data (the evaluation data and the holdout) by dataset and version; see [../judge/model.md](../judge/model.md). The agent's code reads through the connector with its credential. The server never calls the connector and stores no rows.
9. **Data stays put.** Reads run in a session's sandbox on an agent platform, or in a judge sandbox, removed when it ends, never through the Ratchet server. A session contacts only the endpoints its experiment and environment allow.
10. **No transform.** The connector reads tables and files as they are. Ratchet never reshapes or pipes data. The customer prepares it in their own warehouse.
11. **Datasets and the holdout.** The connector's datasets appear in the Datasets table on its page (name, version, size, marking, Agents "Can read" or "No access", Judge "Can read"). The holdout row reads "Restricted to the judge" and has no grant control. A dataset version never changes after it is created. See [model.md](model.md).
12. **Row actions.** Check connection, Update credential and Remove. Remove deletes the values from the platform's credential store, after a confirm that names the experiments that can no longer measure.
13. **Audit.** Adding, checking, updating, using and removing a credential are named actions with an actor (see [../organization/SKILL.md](../organization/SKILL.md)). Settings, Connections lists the repos, credentials and environments that were approved and links to Connectors for data systems.
14. **Tests.** Add a mock connector and a test: check states, secret never returned, agent credential read-only, holdout read refused for the agent credential and allowed for the judge's, no rows through the server, a dataset version unchanged after a new one is made.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

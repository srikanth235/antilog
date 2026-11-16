# Add a connector

A connector is a data system or MCP server an agent's code may read: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, Amazon S3 or any MCP server. The code host (GitHub, GitLab) is how repos attach and pull requests open, and is added as a code host, not a connector. A cloud account is not a connector. All steps are target.

Connectors live in Customize, Connectors (Yours and Discover; boards `Connectors`, `ConnectorsEmpty`, `ConnectorsDiscover`, `ConnectorDetail`, `AddConnector`, `AddConnectorMcp`, `ConnectorError`, `DatasetHoldout`; layout in [../design/behaviour.md](../design/behaviour.md)). A connector is added there with "Add connector", or from the Add credential card in a chat; what a card adds appears in Customize.

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

3. **The Add connector page.** The per-system fields (for example account and warehouse for Snowflake; the server address and the tools to allow for an MCP server), then the two credentials, each with its recipe, a secure field and a "What each one gets" block: agents read the views listed, holdout excluded; the judge reads those and the holdout. Values go straight to the platform's credential store.
4. **A card in the chat.** A session asks with `request_credential` ([tools.md](tools.md)). The Add credential card shows the same recipe and "What each one gets", and a "Browse connectors" link to Customize, Connectors, Discover. The value is typed into the secure form, never the chat; Ratchet keeps only a credential reference.
5. **Discover.** A grid of the connectors Ratchet can connect to (Snowflake, Databricks, BigQuery, Postgres, DynamoDB, Amazon S3, MCP server), reached from "Add connector" and from the Add credential card.
6. **No secret in a response or log.** A credential lives only in the platform's credential store, is short-lived and is minted per session. Ratchet holds no connector admin secret.
7. **Check.** Adding a connector runs a check and reports Connected, Reconnect or Not checked; an unchecked connector is never shown as healthy. When a credential cannot be read, `ConnectorError` says "Can't read the credential for NAME.", shows its datasets as "Not checked" (never empty), names the experiments that cannot measure until it is fixed, and offers "Check again" and "Update credential".
8. **Name the data, never read it.** An experiment names its data by dataset and version ([../judge/model.md](../judge/model.md)). The agent's code reads through the connector with its credential. The server never calls the connector and stores no rows.
9. **Data stays put.** Reads run in a session's sandbox or a judge sandbox, never through the Ratchet server, and a session contacts only the endpoints its experiment and environment allow.
10. **No transform.** The connector reads tables and files as they are; the customer prepares data in their own warehouse.
11. **Datasets and the holdout.** The Datasets table on the connector's page shows name, version, size, marking, Agents ("Can read" or "No access") and Judge ("Can read"). The holdout row reads "Restricted to the judge" and has no grant control. See [model.md](model.md).
12. **Row actions.** Check connection, Update credential and Remove. Remove deletes the values from the platform's credential store, after a confirm that names the experiments that can no longer measure.
13. **Audit.** Adding, checking, updating, using and removing a credential are named actions with an actor (see [../organization/SKILL.md](../organization/SKILL.md)). Settings, Connections lists the repos, credential references and environments approved in chats and links here for data systems.
14. **Tests.** Add a mock connector and a test: check states, secret never returned, agent credential read-only, holdout read refused for the agent credential and allowed for the judge's, no rows through the server, a dataset version unchanged after a new one is made.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

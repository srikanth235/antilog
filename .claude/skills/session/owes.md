# What session owes

| Promise | The rule |
| --- | --- |
| Nothing asked first | Before the first message a person chooses at most a project, repos, an agent, an environment and a budget, each with a default. No settings page, no form, no plan step |
| Credentials never in chat | A credential value is typed into a secure form and goes straight to the server's sealed store. It is never in the conversation, a prompt or a repo file |
| Clean up | Every session's machine is released when idle or stopped and its disk dropped; a dead runner's lease lapses |
| Spend recorded honestly | Every dollar is tied to a session and its agent, read from the usage the proxy sees in each model reply. The proxy refuses a model call with a 402 once the session's cap, its project's monthly cap or the organization's monthly cap is spent (built) |
| Failure never shown as empty | A conversation, a tool output or a log that cannot be fetched says so. It is never shown as blank or as healthy |
| Least access | Antilog holds only the customer's model keys, saved per organization per provider, sealed in Postgres and used only by the proxy (built). No cross-account role, no cloud keys, no connector admin secrets. A connector's own credential is held sealed and never leaves the server |

## How each is checked

**Nothing asked first.** Check: a session starts from a composer with one message and the chips (Project, Repos, Agent, Environment, Budget), and no step blocks it. A missing repo, credential, environment or budget becomes a card at the moment it is needed.

**Credentials never in chat.** Check: no message, event, log or tool argument carries a credential value, a scan of stored conversations finds none, and the form posts only to the server.

**Clean up.** Check: after a session ends, its machine is released by the runner and its cost stops counting.

**Spend recorded honestly.** Check: the gateway refuses a call at each of the three caps (`antilog-proxy` tests, `budget::ensure_room`), every spend row carries the session and the agent, and the session's spend (`GET /sessions/{id}/spend`) sums its rows.

**Failure never shown as empty.** Check: a runner error shows as failed, and a conversation that cannot be fetched says so.

**Least access.** Check: no path stores a cloud key, a cross-account role or a connector admin secret, and no response, event or log carries a model key or connector credential.

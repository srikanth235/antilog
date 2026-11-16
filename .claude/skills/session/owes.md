# What session owes

| Promise | The rule |
| --- | --- |
| Nothing asked first | Before the first message a person chooses at most repos, an agent, an environment and a budget, each with a default. No settings page, no form, no plan step (target). |
| Credentials never in chat | A credential value is typed into a secure form and goes straight to the platform's credential store. It is never in the conversation, a prompt, a repo file or Ratchet's database (target). |
| Clean up | Every session and sandbox Ratchet starts is ended, deleted on its platform and shown as removed (target). |
| Spend recorded honestly | Nothing is spent without a budget. Every dollar is tied to an experiment, a session and its agent, read from the platform's usage. The experiment's budget covers all its sessions and the judge time (target). |
| Failure never shown as empty | A conversation, a tool output or a log that cannot be fetched says so. It is never shown as blank or as healthy (target). |
| Least access | Ratchet holds only agent platform credentials: the customer's when self-hosted, its own on Ratchet cloud. No cross-account role, no cloud keys, no connector admin secrets (target). |
| Any platform | The same promises hold on every agent platform (target). |

## How each is checked

All are owed: nothing on this branch checks them. What the earlier code had is in [today.md](today.md).

**Nothing asked first.** Check: an experiment starts from a composer with one message and the chips (Repos, Agent, Environment, Budget), and no step blocks it. A missing repo, credential, environment or budget becomes a card at the moment it is needed.

**Credentials never in chat.** Check: no message, event, log or tool argument carries a credential value, a scan of stored conversations finds none, and the form posts only to the platform's credential store.

**Clean up.** Check: after a session ends, listing the platform's sessions and sandboxes tagged with it finds nothing, and the page shows removed.

**Spend recorded honestly.** Check: the budget meter stops a session at its limit, every spend row carries the session and the agent, and rows reconcile against the platform's bill.

**Failure never shown as empty.** Check: a test fetches an expired tool output and expects "expired", not an empty block; a platform error shows as failed.

**Least access.** Check: no path stores a cloud key, a cross-account role or a connector admin secret, and no response, event or log carries a platform credential.

**Any platform.** Check: one test suite every adapter must pass, run against a recorded fake in CI and against each real platform with the owner's go-ahead (see [../land/real-cloud.md](../land/real-cloud.md)).

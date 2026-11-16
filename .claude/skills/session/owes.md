# What session owes

| Promise | The rule |
| --- | --- |
| Nothing asked first | Before the first message a person chooses at most a project, repos, an agent, an environment and a budget, each with a default. No settings page, no form, no plan step (target). |
| Credentials never in chat | A credential value is typed into a secure form and goes straight to the platform's credential store. It is never in the conversation, a prompt, a repo file or Ratchet's database (target). |
| Clean up | Every session and sandbox Ratchet starts is ended, deleted on its platform and shown as Removed. One that is not deleted shows Not removed, is retried, and its cost counts until it is gone (target). |
| Spend recorded honestly | Nothing is spent without a budget. Every dollar is tied to an experiment, a session and its agent, read from the platform's usage, and shown per session and per experiment. The budget covers all its sessions and the judge time (target). |
| Spend well | Cost per trial is shown and derived from spend records, the Seal experiment card estimates a measurement before it is bought, agents read `history` before they propose, spawned sessions default to the lower-cost agent, and Where the spend went accounts for every dollar (target). |
| No tie to one provider | Auto chooses by measured cost per win among the agents turned on and says why, a session can be handed off to another agent, and a platform that fails hands the session to a Continue on another agent? card. No agent is preselected by brand (target). |
| Failure never shown as empty | A conversation, a tool output or a log that cannot be fetched says so. It is never shown as blank or as healthy (target). |
| Least access | Ratchet holds only agent platform credentials: the customer's when self-hosted, its own on Ratchet cloud. No cross-account role, no cloud keys, no connector admin secrets (target). |
| Any platform | The same promises hold on every agent platform (target). |

## How each is checked

Nothing on this branch checks them yet.

**Nothing asked first.** Check: an experiment starts from a composer with one message and the chips (Project, Repos, Agent, Environment, Budget), and no step blocks it. A missing repo, credential, environment or budget becomes a card at the moment it is needed.

**Credentials never in chat.** Check: no message, event, log or tool argument carries a credential value, a scan of stored conversations finds none, and the form posts only to the platform's credential store.

**Clean up.** Check: after a session ends, listing the platform's sessions and sandboxes tagged with it finds nothing and the sessions table shows Removed. A session whose deletion fails shows Not removed and keeps counting its cost.

**Spend recorded honestly.** Check: the budget meter stops a session at its limit, every spend row carries the session and the agent, rows reconcile against the platform's bill, and the sessions table and Settings, Budgets sum to the experiment's spend.

**Spend well.** Check: cost per trial equals the spend rows since the session's previous trial plus the judge time, and the Where the spend went categories sum to the experiment's spend, with Not measured the remainder. A test that `spawn` with no agent named picks the organization's lower-cost agent, and that the brief tells agents to call `history` first.

**No tie to one provider.** Check: a test that Auto picks the best cost per win among enabled agents only and records its reason, that with no data it picks the organization's default and with none set asks, that a handed-off session starts on the same branch with a summary and the old one is Done, and that a platform failure raises the Continue on another agent? card (or follows policy and records it).

**Failure never shown as empty.** Check: a test fetches an expired tool output and expects "expired", not an empty block; a platform error shows as failed.

**Least access.** Check: no path stores a cloud key, a cross-account role or a connector admin secret, and no response, event or log carries a platform credential.

**Any platform.** Check: one test suite every adapter must pass, run against a recorded fake in CI and against each real platform with the owner's go-ahead (see [../land/real-cloud.md](../land/real-cloud.md)).

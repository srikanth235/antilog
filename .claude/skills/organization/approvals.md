# Approvals and budgets

Built: raising a card, policy, deciding and who may decide (`antilog-core/src/approval.rs`), budgets, caps and `ensure_room` for the gateway (`budget.rs`), and their endpoints. The cards' chat UI, Mode on the composer, the Settings pages and notifications are web work or target.

A person approves the tool calls that matter, in the chat, when they are needed. Settings pages list what was approved and revoke it ([rulings](../product/rulings.md)).

## The cards

An approval card has a title, what would happen, what it may cost, and Approve and Decline. The six kinds:

| Card | Raised by | Shows | Default policy |
| --- | --- | --- | --- |
| Raise budget (`raise_budget`) | `request_budget` | The amount asked, the session's budget now, the reason | Allow up to a limit: asks above the limit set in Settings, Approvals. Approving raises that session's cap |
| Add credential (`add_credential`) | `request_credential` | The connector, the scope and why. A secure form: the value is typed there, never into the chat, and goes straight to the server's secret store. The agent never sees it | Always ask, and can never be pre-approved |
| Attach repo (`attach_repo`) | `attach_repo` | The repo and the access asked. Approving clones it into the live machine | Always ask |
| Change environment (`change_environment`) | `set_environment` | The environment it changes and the versions, then a diff against what the session runs: network and hosts (apply now), variables (new commands now), setup script (runs now), default repos (cloned now). Approving makes the environment's next version, which other sessions on it also get, and moves the session to it | Always ask |
| Allow this tool? (`ask_first`) | The harness asked to use a tool that is set to ask first | The tool and what it would do, as the harness described it. Actions: Allow once, Decline. The answer goes back to the machine that asked | Always ask |
| Use a connector tool (`use_connector_tool`) | A connector tool set to Ask first | The tool and the exact input. One approval pays for one call | Always ask |

The tools are in [../session/tools.md](../session/tools.md). A decided card reaches the agent at once: its outcome is sent into the session as a message from Antilog.

## Policy

Settings, Approvals sets one value per card kind: **Always ask**, **Allow up to a limit** or **Allow**. A card that policy approves still writes its audit row and shows in the chat as approved by policy. Add credential has no pre-approval, because a person must type the value. A card nobody may approve in time stays open and the session is Waiting for you.

## Mode

Each session has a Mode on its composer (`followPolicy` on the session): **Ask** (every card asks a person) or **Follow policy** (cards that policy allows go through without asking, and each still writes its audit row and shows in the chat as approved by policy). Policy always caps Follow policy: a card kind set to Always ask still asks, and a limit still holds.

## Who may approve

Roles are checked at project scope ([model.md](model.md)). Owners and editors of the card's project approve its cards. A viewer reads them and approves nothing. `GET /approvals/pending` lists the cards the caller may decide, by their role in each card's project. Changing a session's budget needs an editor or owner of its project. Only an organization owner changes policy.

## Recorded

Every approval is an audit row (`approval.decide`): who, which card, what it would do, when, the outcome (approved, declined, approved by policy). So is every tool call that changes something (`tool.call`). Audit names include `session.set_environment` (a person's move or an approved Change environment card; the new version itself is `environment.edit` or `environment.create`), `session.start`, `session.message`, `session.stop`, `session.set_budget`, `project.create`, `project.set_role`, `project.set_limits`, `organization.set_limits`, `policy.set`, `model_key.set` and `connector.set`.

## Settings pages

Settings, Approvals holds policy. Settings, Connections holds the environments sessions run on: everyone reads them, owners and editors add, change (a new version) and archive them. Budgets holds the organization's monthly cap and default budget per session, and the projects with their caps and spend (`GET /budgets`). These pages are read-only for non-owners, not hidden. Two things are set up in Settings because no session can run without them: model keys (Settings, Model keys: an owner sets, replaces or removes one per provider; every member sees which providers have one and never the key; built), and members and sign-in.

## Budgets

Caps nest: a session's own optional cap is at most its project's monthly cap, which is at most the organization's monthly cap. A write that would break the order is refused with 409 `over_cap`.

- **Spend** is model tokens, booked by the proxy from each reply, and machine time. The proxy refuses a model call with a 402 once a cap is spent; the meter is in [../session/model.md](../session/model.md). Starting a session when the project's or the organization's monthly cap is already reached is refused with 409 `budget_spent`.
- **A session's cap** is optional. A person sets it when starting. It defaults to the project's default budget per session (`defaultBudgetCents`), which falls back to the organization's default. An editor or owner can change it at any time (`PUT /sessions/{id}/budget`); each change is an audit row (`session.set_budget`).
- **`request_budget`** lets the agent ask for more through a Raise budget card, and works only for a session with its own cap (otherwise it says so and points to Settings). An approved card raises that session's cap.
- **A project's monthly cap** and its default per session are set by a project owner (`PUT /projects/{id}/limits`), and cannot pass the organization's.
- **The organization's monthly cap** is changed only by an organization owner (`PUT /organization/limits`). No session cap, project cap or card passes it.
- **The gateway check** (`budget::ensure_room`) looks at the session's cap, then the project's, then the organization's.
- When a cap is spent the session is Waiting for you. A notification says so ([notifications.md](notifications.md)).

**Card fields.** Each card kind carries a typed payload (`CardPayload` in `antilog-types/src/ledger.rs`): Raise budget the budget now, the amount asked and why; Add credential the connector, scope and why; and so on. A tool raises the card with its payload, the web card renders from it and the action after approval reads from it. Never put what a person must read only in the summary text, and never parse the summary.

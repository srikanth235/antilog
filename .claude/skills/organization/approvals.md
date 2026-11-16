# Approvals and budgets (target)

Settings are tools. A person approves the tool calls that matter, in the chat, when they are needed. Settings pages list what was approved and revoke it. Nobody visits a settings page before the first message.

## The cards

An approval card has a title, what would happen, what it may cost, and Approve and Decline. The six kinds:

| Card | Raised by | Shows | Default policy |
| --- | --- | --- | --- |
| Seal experiment | `seal` | The definition, the seal check results and a Budget row | Always ask |
| Raise budget | `request_budget` | The amount asked, spent so far, the budget now | Allow up to a limit: asks above the limit set in Settings, Approvals |
| Add credential | `request_credential` | The host and why. A secure form: the value is typed there, never into the chat, and goes straight to the platform's credential store. The agent never sees it | Always ask, and can never be pre-approved |
| Attach repo | `attach_repo` | The repo and the access asked | Always ask |
| Change environment | `set_environment` | Image, setup script, size and network, as a change from now | Always ask |
| Open pull request | `deliver` | One pull request per repo changed, and the receipt | Always ask |

The tools are in [../session/tools.md](../session/tools.md) and [../judge/tools.md](../judge/tools.md).

## Policy

Settings, Approvals sets one value per card kind: **Always ask**, **Allow up to a limit** or **Allow**. A card that policy approves still writes its audit row and shows in the chat as approved by policy. Add credential has no pre-approval, because a person must type the value. A card nobody may approve in time stays open and the session is Waiting for you.

## Mode

Each experiment has a Mode on its composer: **Ask** (every card asks a person) or **Auto** (cards that policy allows go through without asking, and each still writes its audit row and shows in the chat as approved by policy). Policy always caps Auto: a card kind set to Always ask still asks, and a limit still holds.

## Who may approve

Roles are checked at project scope ([model.md](model.md)). A member's role in a project is their organization role unless a project owner set another for that project. Owners and editors of the project approve every card for its experiments. A viewer reads them and approves nothing. Visibility does not follow repo access ([../product/rulings.md](../product/rulings.md)). Changing an experiment's budget needs an editor or owner of its project. A project owner manages the project's members and its budget. Only an organization owner changes policy.

## Recorded

Every approval is an audit row: who, which card, what it would do, when, the outcome (approved, declined, approved by policy). So is every tool call that changes something. Revoking is an audited action too.

## Settings pages

Settings, Approvals holds policy. Settings, Connections lists the repos, credentials and environments that were approved, each with Revoke. Budgets holds the organization's default budget per experiment, the monthly cap, and the list of projects and experiments with their budgets and spend. These pages are read-only for non-owners, not hidden. Two things are still set up in Settings because no experiment can run without them: agent platform accounts (self-hosted), and members and sign-in.

## Budgets

- One budget per experiment, covering all its sessions and their judge time. Spend is model tokens, sandbox time and judge time. The meter is in [../session/model.md](../session/model.md).
- A person sets it when starting, from the budget on the composer's row. It defaults to the project's default budget per experiment, which falls back to the organization's default in Settings, Budgets (boards show $10 as the example).
- An editor or owner can change it at any time: from the scoreboard strip ("Change budget"), from the experiment detail page, or in a row on the Seal experiment card. Lowering it below what is spent stops new measurements. Each change is an audit row. An experiment's budget sits inside its project's cap, and a raise that would pass the cap is refused.
- `request_budget` lets the agent ask for more through a Raise budget card; policy for it stays in Settings, Approvals.
- A project may have a monthly cap on all its experiments. A project owner changes it, and it cannot pass the organization's. The project page shows spent this month against it.
- An organization has a monthly cap. No budget, no project cap and no card passes it. Only an organization owner changes the cap.
- When a budget is spent, the judge refuses `measure`, and the experiment's sessions are Waiting for you with a Raise budget card. A notification says so ([notifications.md](notifications.md)).
- When an experiment is sealed and the budget allows, nothing asks anything.

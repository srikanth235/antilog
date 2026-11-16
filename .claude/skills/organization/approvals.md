# Approvals and budgets (target)

Settings are tools. A person approves the tool calls that matter, in the chat, when they are needed. Settings pages list what was approved and revoke it. Nobody visits a settings page before the first message.

## The cards

An approval card has a title, what would happen, what it may cost, and Approve and Decline. The six kinds:

| Card | Raised by | Shows | Default policy |
| --- | --- | --- | --- |
| Seal contract | `seal` | The contract and the seal check results | Always ask |
| Raise budget | `request_budget` | The amount asked, spent so far, the budget now | Allow up to a limit: asks above the standing allowance per session |
| Add credential | `request_credential` | The host and why. A secure form: the value is typed there, never into the chat, and goes straight to the platform's credential store. The agent never sees it | Always ask, and can never be pre-approved |
| Attach repo | `attach_repo` | The repo and the access asked | Always ask |
| Change environment | `set_environment` | Image, setup script, size and network, as a change from now | Always ask |
| Open pull request | `deliver` | One pull request per repo changed, and the receipt | Always ask |

The tools are in [../session/tools.md](../session/tools.md) and [../judge/tools.md](../judge/tools.md).

## Policy

Settings, Approvals sets one value per card kind: **Always ask**, **Allow up to a limit** or **Allow**. A card that policy approves still writes its audit row and shows in the chat as approved by policy. Add credential has no pre-approval, because a person must type the value. A card nobody may approve in time stays open and the session is Waiting for you.

## Who may approve

Roles are organization-wide. Owners and editors approve every card. A viewer reads sessions and contracts and approves nothing. Whether a person sees only some repos' sessions is an open owner call ([../product/rulings.md](../product/rulings.md)); until it is decided, a role covers the whole organization. Only an owner changes policy.

## Recorded

Every approval is an audit row: who, which card, what it would do, when, the outcome (approved, declined, approved by policy). So is every tool call that changes something. Revoking is an audited action too.

## Settings pages

Settings, Approvals holds policy. Settings, Connections lists the repos, credentials and environments that were approved, each with Revoke. Budgets lists budgets and the monthly cap. These pages are read-only for non-owners, not hidden. Two things are still set up in Settings because no session can run without them: agent platform accounts (self-hosted), and members and sign-in.

## Budgets

- One budget per session and the sessions it spawns. Spend is model tokens, sandbox time and judge time. The meter is in [../session/model.md](../session/model.md).
- A standing allowance per session comes from policy. The figure is an open owner call; boards show $10 as an example.
- `request_budget` raises a budget through a Raise budget card.
- An organization has a monthly cap. No card raises a session past it. Only an owner changes the cap.
- When a budget is spent, the judge refuses `measure`, and the session is Waiting for you with a Raise budget card. A notification says so ([notifications.md](notifications.md)).
- When a contract exists and the budget allows, nothing asks anything.

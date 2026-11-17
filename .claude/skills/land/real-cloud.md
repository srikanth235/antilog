# Real cloud

A run that spends money needs the owner's go-ahead. That covers a real agent platform account, a judge sandbox on one, an agent session, and the agent canary. Do not start one on your own judgment.

Nothing here exists yet.

## Rules

- Use a dedicated test account on each platform, never a shared or production one. The account is the owner's.
- State the cap before you start. A run with no budget does not start; the judge refuses `measure` when the budget is spent.
- Never put a key, a token or a connector secret in a file, a log or a report. Secrets go to the platform's credential store. No model key enters a sandbox: model access belongs to the platform account.
- Check clean-up after every run: listing the platform's sessions and sandboxes by the run tag finds nothing. Report what was left, and remove it.
- Unit tests do not prove a platform's credential store, its sandbox's reach or the holdout split. Only a real account does, and the judge sandbox on a real platform has never been exercised. Say that in the report.
- Say what you ran and what it cost.

## Order (target)

1. The spike: one tiny experiment measured end to end on all five platforms (Claude Managed Agents, AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service and Google Agent Runtime), to prove the agent trait, the Antilog tools served to a session over its event stream (asynchronous `measure` sidesteps the undocumented stream on Google and the 10-minute expiry on Foundry), and a judge sandbox that holds a credential no agent sees, with a tiny cap.
2. Fakes only: the platform test suite against the fake platform, and the server against it.
3. One real platform in a dedicated account, in build order (AgentCore first), with a tiny cap, confirming tags, the budget stop (the judge refuses `measure`), that no route runs from an agent session to the judge sandbox, and clean-up.
4. A connector credential registered for a real connector: confirm it is read-only and cannot see the holdout; confirm the judge's credential can.
5. A real agent on a real run, with a stated budget.
6. A full session end to end: a sealed experiment, trials, a Kept win and its pull request.

The command names are (target) and belong to the CLI; add them here when they exist.

In fakes and in CI nothing here applies; say in the report that real-cloud behaviour is unverified. See [report.md](report.md).

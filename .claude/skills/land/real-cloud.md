# Real cloud

A run that spends money needs the owner's go-ahead. That covers a real session machine, a model key, an agent session, and the agent canary. The v0 design is in [docs/V0.md](../../../docs/V0.md). Do not start one on your own judgment.

Nothing here exists yet.

## Rules

- Use a dedicated test account, never a shared or production one. The account is the owner's.
- State the cap before you start. A run with no budget does not start; the judge refuses `measure` when the budget is spent.
- Never put a key, a token or a connector secret in a file, a log or a report. Secrets stay on the server. No model key enters a session machine; the proxy swaps the session's token for the key.
- Check clean-up after every run: listing the machines by the run tag finds nothing. Report what was left, and remove it.
- Unit tests do not prove a machine's network reach or the holdout split. Only a real account does, and none has been exercised. Say that in the report.
- Say what you ran and what it cost.

## Order (target)

1. A connector credential registered for a real connector: confirm it is read-only and cannot see the holdout; confirm the judge's credential can.
2. A real agent on a real run, with a stated budget.
3. A full session end to end: a sealed experiment, trials, a Kept win and its pull request.

The command names are (target) and belong to the CLI; add them here when they exist.

In fakes and in CI nothing here applies; say in the report that real-cloud behaviour is unverified. See [report.md](report.md).

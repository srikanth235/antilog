# Real cloud

A run that spends money needs the owner's go-ahead. That covers a real session machine, a model key, an agent session, and the agent canary. The v0 design is in [docs/V0.md](../../../docs/V0.md). Do not start one on your own judgment.

## Rules

- Use a dedicated test account, never a shared or production one. The account is the owner's.
- State the cap before you start. A run with no cap does not start; the proxy refuses model calls when a cap is spent.
- Never put a key, a token or a connector secret in a file, a log or a report. Secrets stay on the server. No model key enters a session machine; the proxy swaps the session's token for the key.
- Check clean-up after every run: listing the machines by the run tag finds nothing. Report what was left, and remove it.
- Unit tests do not prove a machine's network reach. Only a real host does; a real container run needs a Docker daemon and the built image. Say that in the report.
- Say what you ran and what it cost.

## Order (target)

1. A real runner host and a real model key through the proxy, with a stated budget.
2. A full session end to end: start it in a project, let the agent clone a repo, make a change and stop, then check the spend.

The command names are (target) and belong to the CLI; add them here when they exist.

In fakes and in CI nothing here applies; say in the report that real-cloud behaviour is unverified. See [report.md](report.md).

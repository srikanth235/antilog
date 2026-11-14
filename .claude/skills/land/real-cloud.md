# Real cloud

A run that spends money needs the owner's go-ahead. That covers a real cloud account, a Kubernetes cluster that costs money, a Packer build, an agent run on a subscription or key, and the agent canary. Do not start one on your own judgment.

Nothing here exists yet. The earlier code's notes (on the other branch) are in [today.md](today.md); their lessons still apply.

## Rules

- Use a dedicated sandbox account or cluster, never a shared or production one. It is the customer's infrastructure, so it is the owner's.
- State the cap before you start. A run with no budget does not start; the runner stops it at the cap.
- Never put a key, a token or a connector secret in a file, a log or a report. Secrets go to the customer's secret manager. Model keys reach a sandbox only through the egress proxy.
- Check clean-up after every run: a listing by the run tag finds nothing. Report what was left, and remove it.
- Unit tests do not prove IAM, networking, NetworkPolicy or the holdout split. Only a real account does, and the evaluator on a real account has never been exercised. Say that in the report.
- Say what you ran and what it cost.

## Order (target)

1. Fakes only: the backend contract suite against the fake backend, and the fake runner against the server.
2. One real backend in a dedicated account: `ratchet-runner` against the real backend with a tiny cap, confirming tags, the cap stop, the no-route check between workspace and evaluator, and clean-up.
3. A scoped credential minted from a real connector: confirm it is read-only and cannot see the holdout; confirm the evaluator's credential can.
4. A real agent on a real run, with a stated budget.
5. A full experiment end to end.

The command names are (target) and belong to the runner and the CLI; add them here when they exist.

In fakes and in CI nothing here applies; say in the report that real-cloud behaviour is unverified. See [report.md](report.md).

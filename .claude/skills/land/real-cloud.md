# Real cloud

Source: `docs/REAL_ENV_TESTING.md` (on the earlier code branch, see `AGENTS.md`) and the area notes in `docs/real-env/` (`aws.md`, `bench.md`, `agents.md`, `engine.md`, `github.md`). Read the one for your area first. Some of these were written for the retired TypeScript engine; the notes say so at the top, and what to verify still applies.

## Rules

- **A real-cloud run spends money and needs the owner's go-ahead.** Do not run `tofu apply`, a Packer build, a live run (`ratchetd serve --mode real` with a real account), the harness canary or anything with provider keys on your own judgment.
- Use a dedicated sandbox account, never a shared or production account.
- Nothing in the evaluator sandbox has been exercised on a real AWS account yet, per `docs/REAL_ENV_TESTING.md`. The unit tests do not prove IAM, networking or the holdout split. Only a real account does.
- The order is in the doc: local agents without AWS, then `aws.md` (apply, destroy, kill switch, reaper), `bench.md`, `agents.md`, `engine.md`.
- Check clean-up after every run: the sandbox is destroyed after the idle window and nothing is left. Report what was left.
- Keys go through the app or `ratchet`, never into a file, a log or a report.
- State what you ran and what it cost.

In simulation and in CI nothing here applies; say in the report that real-cloud behaviour is unverified. See [report.md](report.md).

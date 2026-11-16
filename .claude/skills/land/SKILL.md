---
name: land
description: >-
  How do I prove it and ship it? Use before calling a change done: which checks to run for which kind of change (Rust, TypeScript, Swift, Postgres, infra, design), what CI runs, how tests are written, what a real-cloud run needs, git and push rules, and how to report what ran and what did not.
---

# Land

Prove the change at the layer it lives in, report what ran and what did not.

The only real check on this branch today is `python3 design/tools/tokens.py --check`. There is no code and no CI here yet. The pages describe the target checks, with each command that does not exist yet marked (target). [today.md](today.md) keeps the earlier code's commands and CI facts, for reference.

| If you are doing | Read |
| --- | --- |
| Choosing the checks for a change, before calling it done | [checks.md](checks.md) |
| A CI failure, or waiting on a run | [ci.md](ci.md) |
| Writing a test | [tests.md](tests.md) |
| Anything that touches a real cloud account, real agents or spends money | [real-cloud.md](real-cloud.md) |
| Committing, branching, pushing | [git.md](git.md) |
| Writing the final report | [report.md](report.md) |
| What the earlier code ran | [today.md](today.md) |

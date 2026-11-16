# Judge: what the earlier code did, and what it taught

The earlier Rust daemon and CLI are on the branch `claude/peaceful-lamport-leak6p`. They are reference, not a base. Nothing here exists on this branch. Paths below are on that branch. The earlier code used other words: "evaluator" for the evaluation and its sandbox, "objective" for the metric, "guardrail" for a limit, "candidate" for a trial, "verdict" for an outcome, "ratchet move" for a recorded win, "experiment" and "run" for the work as a whole.

## What it did

| Piece | Where |
| --- | --- |
| Config: `objective` (metric, direction, `min_gain`, value model), `evaluator` (kind, ref, command, repeats, noise factor, hardware, holdout), `guardrails`, `protected` | `crates/ratchet-core/src/config/schema.rs`, `crates/ratchet-core/src/config/guardrails.rs` |
| Evaluation kinds: benchmark, backtest, eval, simulation | `EvaluatorKind` in `crates/ratchet-core/src/enums.rs` |
| Starter benchmarks | `templates/` (build-size, cli-cold-start, forecast-backtest, http-latency, llm-eval, memory, test-suite-time); docs in `docs/templates.md` and `docs/packs/` |
| Statistics and noise floor | `crates/ratchet-core/src/bench/stats.rs`, `crates/ratchet-core/src/bench/calibrate.rs` |
| The acceptance rule, a pure function | `evaluate_ratchet` in `crates/ratchet-core/src/ratchet.rs` |
| Verdicts | `Verdict` and `CandidateVerdict` in `crates/ratchet-core/src/enums.rs` |
| Protected paths (a candidate that touches one is blocked) | `crates/ratchet-core/src/protected.rs`, `crates/ratchet-core/src/protected_set.rs` |
| The control loop and its stops | `crates/ratchet-engine/src/tick.rs`, `crates/ratchet-engine/src/engine.rs` |
| Measuring, overlay of protected files from the evaluator ref, confirm against the holdout | `crates/ratchet-bench/src/pipeline.rs`, `crates/ratchet-bench/src/overlay.rs`, `crates/ratchet-bench/src/evaluate.rs` |
| Ledger: candidates, bench results, baselines, ratchet moves and spend events (append-only by trigger) | `crates/ratchet-ledger/src/store.rs`, `crates/ratchet-ledger/migrations/0001_init.sql` |
| Value model, estimate and Return | `crates/ratchet-core/src/value.rs` (`estimate_value`, `return_on_spend`) |
| Delivered and pending split by pull request state | `crates/ratchetd/src/views/overview.rs` |
| Pull request body with the result report | `crates/ratchet-github/src/report.rs`, `crates/ratchet-github/src/squash.rs` |
| What an agent could call: history, steering notes, submit candidate; no tool wrote the evaluator, protected set or budgets | `crates/ratchet-mcp/src/sets.rs`, `docs/ratchet-mcp.md` |
| Simulated end-to-end runs of three demo repositories | `examples/demo-search`, `examples/demo-forecast`, `examples/demo-cli` |
| Tests | `crates/ratchet-engine/tests/engine/core_loop.rs`, `crates/ratchet-engine/tests/engine/runtime_bench_contract.rs`, `crates/ratchet-core/tests/` |

## What it taught

- The acceptance rule as one pure function worked. Keep it, and keep its tests on each side of every boundary.
- The ledger tables for ratchet moves and spend were append-only by trigger, but the candidate verdict was an `UPDATE`. The judge's outcomes need the same database guard.
- The holdout was optional in the config, and the confirm step ran only when one was named. A holdout that may be skipped is not a guarantee. In the contract it is required.
- Protected files came back from the evaluator ref by overlay. The seal replaces that ref with a hash over pinned commits.
- The earlier code had no cheat checks and no signed receipt. Dataset versions, the machine type and the credential scopes were not recorded, and nothing signed the numbers.
- Nothing measured a merged win again. That gap is the watch.
- The only change it knew was a commit on one repo's branch, and the only data was what was in the repo or the sealed bucket.
- Spend caps were set and enforced by the engine's own tick. Now the budget refuses `measure`.
- The earlier rule that an agent never reads the evaluation's code is replaced: it may read it and never change it.

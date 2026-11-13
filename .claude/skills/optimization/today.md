# Optimization: what exists today, and what is target

Every path below was checked to exist. Names in the code differ from the target words: the code says "evaluator" for the benchmark, "objective" for the goal, "guardrail" for a limit, "ratchet move" for a recorded win.

## Exists

| Piece | Where |
| --- | --- |
| Config: `objective` (metric, direction, `min_gain`, value model), `evaluator` (kind, ref, run, repeats, noise factor, hardware, holdout), `guardrails`, `protected`, `agent` | `crates/ratchet-core/src/config/schema.rs`, `crates/ratchet-core/src/config/guardrails.rs` |
| Evaluator kinds: benchmark, backtest, eval, simulation | `EvaluatorKind` in `crates/ratchet-core/src/enums.rs` |
| Starter benchmarks | `templates/` (build-size, cli-cold-start, forecast-backtest, http-latency, llm-eval, memory, test-suite-time), loaded by `crates/ratchet-core/src/templates/`; docs in `docs/templates.md` and `docs/packs/` |
| Statistics and noise floor | `crates/ratchet-core/src/bench/stats.rs`, `crates/ratchet-core/src/bench/calibrate.rs` |
| The accept rule, a pure function | `evaluate_ratchet` in `crates/ratchet-core/src/ratchet.rs` |
| Verdicts | `Verdict` and `CandidateVerdict` in `crates/ratchet-core/src/enums.rs` |
| Protected paths (a candidate that touches one is blocked) | `crates/ratchet-core/src/protected.rs`, `crates/ratchet-core/src/protected_set.rs` |
| The control loop: candidates, bench queue, verdicts, budget, deadline and plateau stops | `crates/ratchet-engine/src/tick.rs` (budget stop near `budget_usd`), `crates/ratchet-engine/src/engine.rs` |
| The measuring pipeline, overlay of protected files from the evaluator ref, confirm against the holdout | `crates/ratchet-bench/src/pipeline.rs`, `crates/ratchet-bench/src/overlay.rs`, `crates/ratchet-bench/src/evaluate.rs` |
| Ledger: candidates, bench results, baselines, ratchet moves (append-only by trigger), spend events (append-only by trigger) | `crates/ratchet-ledger/src/store.rs`, `crates/ratchet-ledger/migrations/0001_init.sql` |
| Value model, estimate and Return | `crates/ratchet-core/src/value.rs` (`estimate_value`, `return_on_spend`) |
| Delivered and pending split by pull request state | `crates/ratchetd/src/views/overview.rs` |
| Pull request body with the result report | `crates/ratchet-github/src/report.rs`, `crates/ratchet-github/src/squash.rs` |
| What an agent may call: history, steering notes, submit candidate; no tool writes the evaluator, protected set or budgets | `crates/ratchet-mcp/src/sets.rs`, `docs/ratchet-mcp.md` |
| Simulated end-to-end runs of three demo repositories | `examples/demo-search`, `examples/demo-forecast`, `examples/demo-cli`; `crates/ratchet-engine/tests/engine/runtime_simulation.rs`; job `demos` in `.github/workflows/ci.yml` |
| Engine and rule tests | `crates/ratchet-engine/tests/engine/core_loop.rs`, `crates/ratchet-engine/tests/engine/runtime_bench_contract.rs`, `crates/ratchet-core/tests/` |

## Gaps in what exists

- A verdict is not immutable in the database. `set_verdict` in `crates/ratchet-ledger/src/store.rs` runs an `UPDATE`, and only `ratchet_moves` and `spend_events` have append-only triggers. The engine writes a verdict once, but the table does not enforce it.
- The holdout is optional in the config. The confirm step runs only when one is named.
- The only subject today is a git repository, changed by commits on an agent branch. The only data a benchmark reaches is what is in the repo or the sealed bucket.
- Spend caps (`budget_usd`) are set on the optimization and enforced by the engine's own tick, not by an infra layer.

## Target, not built

- A receipt record per win with dataset versions and machines.
- Re-checking a merged win on a schedule ("Keep").
- Subjects other than a git repository (a prompt, a query, a config with a version and a diff).
- Benchmarks that name dataset versions and an environment id, as in [model.md](model.md).
- A database-level rule that a verdict never changes.
- Making the holdout required for a win to count.

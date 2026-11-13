# Infra: what exists today, and what is target

Every path below was checked to exist.

## Exists

| Piece | Where |
| --- | --- |
| The sandbox contract: inputs, outputs and invariants every cloud module must satisfy | `infra/contract.md` |
| The AWS module (OpenTofu): VPC, buckets (artifacts and sealed), agent permission boundary, bench and confirm identities, DNS firewall, optional egress proxy | `infra/aws/` (`main.tf`, `iam.tf`, `network.tf`, `s3.tf`, `proxy.tf`, `dns_firewall.tf`, policies in `infra/aws/policies/`) |
| The environment module: a drafted environment's machines, tagged `ratchet:purpose=environment` and `ratchet:managed=opentofu` | `infra/modules/sandbox/` (test in `infra/modules/sandbox/tests/machines.tftest.hcl`) |
| Module tests (`tofu test`) and policy simulation | `infra/aws/tests/`, `infra/tests/src/` |
| Machine images (Packer): runner and bench AMIs, install scripts | `infra/images/` (`runner.pkr.hcl`, `bench.pkr.hcl`, `scripts/`), `infra/images/README.md` |
| Sandbox lifecycle: plan, apply, destroy, status; mock provider | `crates/ratchet-sandbox/src/sandbox/` (`tofu.rs`, `mock.rs`, `environment.rs`) |
| Compute: launch, list by tag, terminate; AWS and mock | `crates/ratchet-sandbox/src/compute/` |
| Tags: `ratchet:opt`, `ratchet:agent`, `ratchet:role`, `ratchet:sandbox`, `ratchet:purpose` | `crates/ratchet-sandbox/src/util.rs` |
| The reaper: lifetime and per-agent instance limits, clean-up of ended optimizations | `crates/ratchet-sandbox/src/reaper.rs`, `crates/ratchet-engine/src/reaper.rs` |
| Spend: estimate, ledger events, billing reconcile | `crates/ratchet-core/src/cost.rs`, `crates/ratchet-sandbox/src/cost.rs`, `crates/ratchet-sandbox/src/cost_explorer.rs`, `crates/ratchet-sandbox/src/pricing.rs` |
| Budget stop and 80 percent style warnings | `crates/ratchet-engine/src/tick.rs` |
| Dynamic environments: a setup agent drafts an environment, it is validated, costed and locked on approval | `crates/ratchet-env/src/`, `docs/SIMPLIFIED_MODEL.md` |
| Remote benchmark execution on the bench machines | `crates/ratchet-bench/src/remote.rs`, `crates/ratchet-bench/src/remote_session.rs` |
| Agent adapters (Claude Code, Codex, Grok), local detection, worktrees, the runner binary | `crates/ratchet-agents/src/` (`claude_code.rs`, `codex.rs`, `grok.rs`, `detect.rs`, `local_agents.rs`, `runner_main.rs`) |
| Fallback order across agents | `crates/ratchet-agents/src/fallback.rs`, `crates/ratchet-core/src/agents_policy.rs` |
| Ledger of sandboxes and spend | `crates/ratchet-ledger/src/store.rs`, `crates/ratchet-ledger/tests/sandbox_lifecycle.rs` |
| Settings and AWS connector with a monthly cap | `crates/ratchet-connectors/src/model.rs` (`monthly_cap_usd`) |
| CI | `.github/workflows/ci.yml` (job `infra`: fmt, validate, `tofu test`), `.github/workflows/images.yml` (launcher, runner, `packer validate`) |
| Real-environment testing notes | `docs/REAL_ENV_TESTING.md`, `docs/real-env` |

## Gaps in what exists

- Provider is a two-value enum, `aws` and `mock` (`SandboxProviderKind` in `crates/ratchet-core/src/enums.rs`).
- Tags carry the optimization and agent, not a run.
- "Where" is This machine or Cloud. A local run proposes on the user's machine; measuring is always in the cloud sandbox.
- Spend is tied to an optimization and agent, and the cap is enforced by the engine tick.

## Target, not built

- GCP and Azure modules and a provider-neutral contract test that every module passes.
- A run tag on every resource and every spend row.
- A verified clean-up state per run, shown as removed.
- A refusal to start a run with no stated budget.
- Model providers as a stored object, separate from the agent.
- Environments as named, versioned objects referenced by id from a benchmark.

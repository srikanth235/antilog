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

The target replaces most of the above (decided 16 Nov): Ratchet runs no sandboxes, OpenTofu modules, images or runner. Agents run as sessions on agent platforms behind one trait; see [model.md](model.md). What carries over is the shape of spend (append-only rows, a budget stop), tagging everything with its run, and a verified clean-up.

- The platform trait and adapters: Claude Managed Agents, then AWS AgentCore, then the OpenAI Agents API, then Microsoft Foundry Agent Service, then Google Agent Runtime.
- The `measure` tool, answered over the session's event stream, and the evaluator sandbox on the measuring platform.
- A run tag on every session, sandbox and spend row, and a verified clean-up state per run.
- A refusal to start a run with no stated budget.
- Environments as named objects on a platform, referenced by id from a benchmark.

## Adding a backend, as the earlier code did it

Reference for the earlier code on the other branch. The target has no sandbox backends; adding an agent platform is in [add-platform.md](add-platform.md).

- The module lived under `infra/<cloud>/` and satisfied `infra/contract.md`. The AWS module in `infra/aws/` is the model.
- Sealed isolation was a holdout bucket: the evaluator identity sat outside any agent identity's reach, and the bucket denied agents twice, on the store and in the agent boundary (`infra/aws/policies/sealed_bucket_policy.json.tftpl`, `infra/aws/policies/agent_boundary.json.tftpl`). The target replaces this with the credential model: the evaluator's own credential includes the holdout, and agents never get it.
- Tag keys were in `crates/ratchet-sandbox/src/util.rs`. The compute port (launch, list by tag, terminate) was in `crates/ratchet-sandbox/src/compute/mod.rs`, AWS in `compute/aws.rs`, the double in `compute/mock.rs`. Launch failed closed on a missing tag.
- The reaper was `crates/ratchet-sandbox/src/reaper.rs`. Price table and billing reconcile were `crates/ratchet-sandbox/src/pricing.rs` and `cost_explorer.rs`. Images were in `infra/images/`.
- Providers were the two-value `SandboxProviderKind` in `crates/ratchet-core/src/enums.rs`; widen it only after the contract tests pass.
- Tests were `infra/tests/src/` and `infra/aws/tests/`, plus a job beside `infra` in `.github/workflows/ci.yml`.

# Add a cloud

A cloud is a new sandbox backend behind the launcher's one trait. The order is Kubernetes first (separate node pools for workspace and evaluator, a NetworkPolicy for "no route"), then VMs on AWS, GCP and Azure. On-prem Kubernetes is the customer's infrastructure, and a laptop is not a backend. All of it is target. Today only `aws` and `mock` exist (`SandboxProviderKind` in `crates/ratchet-core/src/enums.rs`).

1. **Satisfy the contract.** Write a module under `infra/<cloud>/` that accepts the inputs and produces the outputs in `infra/contract.md`, and satisfies every invariant there. A module that cannot satisfy one is not a valid sandbox. The AWS module in `infra/aws/` is the model.
2. **Sealed isolation.** The evaluator identity is outside any agent identity's reach, and the holdout store denies agents twice, once on the store and once in the agent boundary. Match `infra/aws/policies/sealed_bucket_policy.json.tftpl` and `infra/aws/policies/agent_boundary.json.tftpl`.
3. **Tagging.** Every resource carries the run tag (a label on Kubernetes), written at creation, before anything else happens to it. Use the key names in `crates/ratchet-sandbox/src/util.rs`. The run tag itself is (target).
4. **Compute.** Implement the compute port: launch, list by tag, terminate (`crates/ratchet-sandbox/src/compute/mod.rs`, AWS in `crates/ratchet-sandbox/src/compute/aws.rs`, test double in `crates/ratchet-sandbox/src/compute/mock.rs`). Launch fails closed on a missing tag.
5. **Clean-up.** The reaper must find and remove everything the provider can leave behind, and the destroy path must report anything it could not remove. Model it on `crates/ratchet-sandbox/src/reaper.rs`.
6. **Spend.** Provide a price table and a billing reconcile for the provider (`crates/ratchet-sandbox/src/pricing.rs`, `crates/ratchet-sandbox/src/cost_explorer.rs`). Spend rows are append-only and corrected, never edited.
7. **Images.** Build the runner and bench machine images for the provider (the AWS definitions are in `infra/images/`).
8. **Provider words stay here.** Regions, project ids, instance ids and node pools appear in no other skill and no other crate's public types. Widen `SandboxProviderKind` only after the contract tests pass.
9. **Credentials.** The backend hands the workspace only the scoped, read-only credential minted for the run, and the evaluator its own. It stores no connector admin secret and gives Ratchet no access to the account.
10. **Same promises.** Run the checks in [owes.md](owes.md) against the new provider. A promise that holds on one cloud and not another is a defect in the new module.
11. **One benchmark, one provider.** A benchmark never spans providers. Do not add cross-provider comparison.
12. **Tests.** Port `infra/tests/src/` and `infra/aws/tests/` to the new module, and add a CI job beside `infra` in `.github/workflows/ci.yml`.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

# Add a sandbox backend

A backend is an adapter behind the launcher's one trait. It makes sandboxes, lists them by run tag and removes them. The order is fixed (all target):

1. Kubernetes first: separate node pools for workspace and evaluator, a NetworkPolicy for "no route". On-prem Kubernetes is the customer's infrastructure.
2. VMs on AWS, then GCP, then Azure.
3. This machine: the macOS app as a backend where agents propose (target). Agents only. It is never an evaluator, never holds the holdout credential and never produces a number that counts.

Today only the earlier code's AWS and mock providers exist; see [today.md](today.md).

## Steps

1. **Satisfy the contract.** The backend accepts the inputs and gives the outputs in the backend contract (target: `infra/contract.md`) and keeps every invariant in it. A backend that cannot keep one is not a valid sandbox backend. The contract is a suite every backend must pass, the same for all of them.
2. **Two sides, no route between.** Workspace sandboxes and evaluator sandboxes are different machines (different node pools on Kubernetes). A workspace sandbox has no network route to an evaluator, enforced by the platform (a NetworkPolicy, a security group, a firewall rule) and not by the agent's good manners. This machine has only the workspace side.
3. **Credentials, not buckets.** The backend hands a sandbox only what the launcher minted for the run:
   - An agent sandbox gets the run's scoped credential: short-lived, read-only, leaving the holdout out. It never sees the evaluator's credential.
   - The evaluator gets its own credential, which includes the holdout. It exists only on evaluator machines.
   - The backend stores no connector admin secret and gives Ratchet no access to the account. Connector admin secrets stay in the customer's secret manager, and only the launcher reads them.
4. **Tag every resource with its run.** Written at creation, before anything else happens to it (a label on Kubernetes). Launch fails closed on a missing tag.
5. **Implement the backend trait.** Launch, list by tag, remove. Plus a test double (a fake backend) the whole suite can use.
6. **Clean-up.** The launcher must find and remove everything the provider can leave behind, and report what it could not remove. A run is not finished until clean-up is verified.
7. **Spend.** Provide a price table and a billing reconcile for the provider. Spend rows are append-only and corrected, never edited. The launcher stops a run at its cap.
8. **Images.** Build the sandbox and evaluator images for the provider (Packer, or container images for Kubernetes). See [../stack/infra/ratchet.md](../stack/infra/ratchet.md).
9. **Provider words stay here.** Regions, project ids, instance ids and node pools appear in no other skill and in no public type outside the adapter.
10. **Same promises.** Run the checks in [owes.md](owes.md) against the new backend. A promise that holds on one backend and not another is a defect in the new one.
11. **One benchmark, one provider.** A benchmark never spans backends. Do not add cross-provider comparison.
12. **Tests.** Run the shared backend contract suite against the new adapter and add a CI job for it. A run on a real account needs the owner's go-ahead; see [../land/real-cloud.md](../land/real-cloud.md).

## Isolation is required

- A backend that cannot isolate the agent side from the evaluator side at the platform level (a NetworkPolicy or its equivalent) is not a valid backend and is not added.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

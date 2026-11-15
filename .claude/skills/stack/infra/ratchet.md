# Infrastructure code in Ratchet

OpenTofu, Packer, Helm and Kubernetes manifests. (target) None of this is on this branch. The earlier code had OpenTofu for an AWS evaluator sandbox and Packer images under `infra/`; those are reference. Versions are in [libraries.md](../libraries.md): OpenTofu 1.13.9, Packer 1.16.2, Helm 4.3.2, `cosign` and `syft` for signing and software bills of materials. Where work happens and what it may cost: [infra skill](../../infra/SKILL.md) and [infra/rules.md](../../infra/rules.md), which win over this page on any rule about sandboxes, credentials and clean-up.

## What each tool is for (target)

| Tool | Used for |
| --- | --- |
| OpenTofu | Cloud resources the customer's side needs: the runner's own machines or cluster pieces, buckets, queues, roles, networks. Modules are small and typed. |
| Packer | Machine images: the runner image and the evaluator and agent workspace images for VM backends. |
| Helm | Packaging the server and the runner for Kubernetes (the first sandbox backend), with `values.yaml` for the customer's choices. |
| Kubernetes manifests | What the chart renders: workspace and evaluator pods in separate node pools, a `NetworkPolicy` for "no route", service accounts, resource limits. |
| Docker Compose | The self-hosted server and Postgres. The only thing that needs no chart. |
| Cloudflare config (`cloudflare.config.ts`, used by `cf`) | The Ratchet cloud Worker and its Durable Object classes, deployed by us (target). See [../cloudflare/ratchet.md](../cloudflare/ratchet.md). |
| Cloudflare Containers | The Ratchet-managed sandboxes, run through a runner we operate (target). See [../cloudflare/ratchet.md](../cloudflare/ratchet.md). |

Ratchet runs in the customer's infrastructure by default. Nothing here creates resources in an account Ratchet operates, except two things we run ourselves: Ratchet cloud (a Cloudflare Worker, deployed by us) and the Ratchet-managed sandboxes on Cloudflare Containers, which an owner chooses per organization ("Where work runs"). The rules below for the customer's account hold for Your infrastructure.

## Workflow and done criteria

```bash
# OpenTofu, in each module or root directory
tofu fmt -check -recursive
tofu init -backend=false -input=false
tofu validate
tofu test                                  # plan-mode tests with mocked providers; no cloud calls

# Packer, from the image directory
packer fmt -check -diff .
packer init .
packer validate -var image_version=ci .    # no cloud calls

# Helm
helm lint charts/<chart>                   # (target)
helm template charts/<chart> -f <values> > /dev/null
```

CI pins the tool versions in [libraries.md](../libraries.md). **Done** when the commands above for what you changed are green, new behaviour has a `tofu test` (or a rendered-chart assertion), and nothing that costs money was run. **Building an image, applying a module, or installing a chart into a cluster spends money or changes real systems**: that is a real-cloud run and follows [land/real-cloud.md](../../land/real-cloud.md); never do it as a side effect of a check. Say what you did not run.

Fix the code, not the check. Do not widen a policy or loosen an assertion to get a test green.

## Rules for the code

- **No cross-account role, no stored cloud keys.** Ratchet never holds access to the customer's cloud. The runner is installed in the customer's account or cluster and connects **out** to the server over HTTPS; it opens no inbound port. Do not write a module that creates a role the server can assume, an access key, or a trust policy naming an account Ratchet operates. No serverless compute in Ratchet's account for Your infrastructure either; the Ratchet cloud Worker and Ratchet-managed sandboxes are ours and the rules in [infra/rules.md](../../infra/rules.md) apply to them.
- **The module never sees model keys or tokens.** Model keys reach a sandbox only through the egress proxy, which adds them on the way out; they are never a variable, a tag, an output, an environment variable of a machine, or in state. Connector admin secrets stay in the customer's secret manager. **State holds no secrets**: use a secret-manager reference or an ephemeral value, mark any sensitive variable and output `sensitive`, and never print state ([general/terraform.md](general/terraform.md)).
- **Credentials are scoped, short-lived and minted per run** by the launcher. A role or policy in code grants the least privilege and is written once in a policy template that tests render and check; do not duplicate it by hand. The sealed bucket (holdout) denies agent roles. **Do not widen a policy to get a test green.** The evaluator gets a separate credential that includes the holdout; an agent's does not.
- **Tag every created resource with its run before anything else happens to it** (a resource without its tags cannot be found or removed). Tags are `ratchet:*` keys, and the run is always among them. The earlier code used `ratchet:opt`, `ratchet:agent`, `ratchet:role` (workspace or evaluator), `ratchet:sandbox`, `ratchet:purpose` and `ratchet:managed=opentofu`; keep names like these. Put them in the provider's `default_tags` or one merged `local`, so no resource can skip them. **Fail closed:** a machine that lacks its run tag is denied everything. A key never goes in a tag value.
- **Clean-up is verified, not assumed.** A run is not finished until its resources are confirmed gone. Provide the means to list by tag, to terminate, and to sweep leftovers older than their lifetime; the launcher and the reaper use them. Each created thing has a lifetime and a cap. A failed `apply`, a failed Packer build or a crashed run can leave resources: the tag is how they are found.
- **Measuring and proposing run on different machines**, in separate node pools or accounts, and a benchmark stays on one provider and one machine type. Provider words (regions, instance ids, node pools) stay inside the module or adapter for that provider.
- **A new cloud or sandbox backend is a product-level change**: follow [infra/add-backend.md](../../infra/add-backend.md) and ask. On-prem Kubernetes counts as the customer's infrastructure.
- **Kubernetes manifests:** every pod has resource requests and limits, a non-root user, a read-only root filesystem where possible, no host mounts, no service account token unless needed, dropped capabilities, and a `NetworkPolicy` that denies by default (the "no route" for an agent workspace) and allows only what is named. Secrets come from the cluster's secret store, not from `values.yaml`.
- **Helm:** details in [general/helm-kubernetes.md](general/helm-kubernetes.md). Values are documented and have safe defaults; nothing in a template depends on a cloud-specific field outside a clearly named block; images are pinned by digest in release values; a chart upgrade is tested from the previous release's values. Sign images with `cosign` and publish a bill of materials with `syft` (target).
- **Pin everything.** Provider versions with `~>`, module versions, OpenTofu's `required_version`, Packer plugin versions, base-image filters with `owners`, chart dependencies. Commit `.terraform.lock.hcl` (OpenTofu uses the same lock file name). Never commit state, `.terraform/`, plan files or `.tfvars` with secrets.
- **OpenTofu is not Terraform.** Use `tofu`, and check a feature against OpenTofu's release notes before using it (the HashiCorp digests are written for Terraform: [general/terraform.md](general/terraform.md)).

## Layout (target)

Written with the first code. The earlier code's layout is a useful shape:

| Folder | What |
| --- | --- |
| `infra/<cloud>/` | An OpenTofu root or module per cloud, with `policies/` (templates for roles and bucket policies, the one source of truth) and `tests/` (`tofu test`, mocked provider) |
| `infra/modules/sandbox/` | A sandbox module with its own `tests/` |
| `infra/images/` | Packer definitions of the runner and workspace images, with a README |
| `charts/` | Helm charts for the server and the runner |
| `infra/contract.md` | What every sandbox module must accept, produce and guarantee |

A TypeScript test suite that renders the policies and simulates the allow and deny decisions was used by the earlier code (vitest). If that returns, it runs with the web checks ([../typescript/ratchet.md](../typescript/ratchet.md)); prefer a `tofu test` where the tool can do it.

## Pitfalls

- **A test that "passes" because the mock is permissive.** Assert the denials: an agent role cannot read the sealed bucket; a machine without its tag cannot do anything; the evaluator cannot be reached from a workspace.
- **Count versus `for_each`.** Removing an item from a `count` list renumbers the rest and replaces resources; key by a stable name ([general/terraform.md](general/terraform.md)).
- **Interpolating into a policy as a string.** Build policy documents from the rendered template or typed data sources, and test them, so a variable cannot widen a statement.
- **A leaked build machine.** A failed Packer build can leave an instance and snapshots. Tag the build machine's run (`run_tags`) and sweep.
- **Provider drift in tests.** Pin versions and use mocked providers so `tofu test` does not depend on a cloud or on today's provider release.
- **Secrets in `local-exec` or `user_data`.** They land in state or a console. Pass a reference and let the machine fetch the value with its own scoped identity.

## General guidance

[`general/terraform.md`](general/terraform.md): HashiCorp's Terraform style guide, tests and module refactoring, written for Terraform and applying to OpenTofu. [`general/packer.md`](general/packer.md): Packer image builds. Both digest [hashicorp/agent-skills](https://github.com/hashicorp/agent-skills) (MPL-2.0, [`licenses/hashicorp-agent-skills-LICENSE`](licenses/hashicorp-agent-skills-LICENSE)). [`general/helm-kubernetes.md`](general/helm-kubernetes.md): our own notes on charts, values, pod security, network policy and permissions, from Helm's and Kubernetes' documentation.

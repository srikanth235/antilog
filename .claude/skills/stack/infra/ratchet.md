# Infrastructure code in Ratchet

OpenTofu, Packer, Helm and Kubernetes manifests. (target) None of this is on this branch. **In the target, none of these tools serves a sandbox.** Ratchet runs no sandboxes: agent platforms do (see [product rulings](../../product/rulings.md), "Architecture (target)"). The OpenTofu module, the Packer images and the Kubernetes sandbox backend are earlier code only: the earlier code had OpenTofu for an AWS evaluator sandbox and Packer images under `infra/`, and they are reference on `claude/peaceful-lamport-leak6p`. Do not write new modules, images or charts for sandboxes. Where work happens and what it may cost: [infra skill](../../infra/SKILL.md) and [infra/rules.md](../../infra/rules.md), which win over this page.

What the target ships, and with what:

| Thing | Shipped with |
| --- | --- |
| Self-hosted server and Postgres | Docker Compose. No chart needed. |
| Ratchet cloud Worker and its Durable Object classes | Cloudflare config (`cloudflare.config.ts`, used by `cf`), deployed by us from CI (target). See [../cloudflare/ratchet.md](../cloudflare/ratchet.md). |
| OpenTofu, Packer, Helm, Kubernetes manifests | Nothing in the target. Whether Helm packages the self-hosted server for Kubernetes, or OpenTofu manages Ratchet's own accounts, is not ruled; ask the owner before adding either. |

Versions of the tools are still in [libraries.md](../libraries.md), marked as not needed by the target. The rest of this page is for earlier code and for the case the owner rules a use for these tools. Where it names a sandbox, a runner or a launcher, it describes the earlier design.

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

- **No cross-account role, no stored cloud keys.** Ratchet never holds access to the customer's cloud. Do not write a module that creates a role the server can assume, an access key, or a trust policy naming an account Ratchet operates. Ratchet holds only agent platform credentials ([infra/rules.md](../../infra/rules.md)).
- **The module never sees model keys or tokens.** Model access belongs to the platform account, so a model key is never a variable, a tag, an output, an environment variable of a machine, or in state. Connector admin secrets are never in Ratchet. **State holds no secrets**: use a secret-manager reference or an ephemeral value, mark any sensitive variable and output `sensitive`, and never print state ([general/terraform.md](general/terraform.md)).
- **Credentials are scoped and read-only for an agent.** (Earlier design: minted per run by the launcher. Today whether connector credentials are minted per run or registered once is open.) A role or policy in code grants the least privilege and is written once in a policy template that tests render and check; do not duplicate it by hand. The sealed bucket (holdout) denies agent roles. **Do not widen a policy to get a test green.** The evaluator gets a separate credential that includes the holdout; an agent's does not.
- **Tag every created resource with its run before anything else happens to it** (a resource without its tags cannot be found or removed). Tags are `ratchet:*` keys, and the run is always among them. The earlier code used `ratchet:opt`, `ratchet:agent`, `ratchet:role` (workspace or evaluator), `ratchet:sandbox`, `ratchet:purpose` and `ratchet:managed=opentofu`; keep names like these. Put them in the provider's `default_tags` or one merged `local`, so no resource can skip them. **Fail closed:** a machine that lacks its run tag is denied everything. A key never goes in a tag value.
- **Clean-up is verified, not assumed.** A run is not finished until its resources are confirmed gone. Provide the means to list by tag, to terminate, and to sweep leftovers older than their lifetime; the earlier launcher and reaper used them. Each created thing has a lifetime and a cap. A failed `apply`, a failed Packer build or a crashed run can leave resources: the tag is how they are found.
- **Measuring and proposing run on different machines**, and a benchmark stays on one provider and one machine type. Provider words (regions, instance ids, node pools) stay inside the module or adapter for that provider. In the target this is the measuring platform and its evaluator sandbox, not a module.
- **A new way to run sandboxes is not a module.** A new agent platform is a product-level change: follow [infra/add-platform.md](../../infra/add-platform.md) and ask.
- **Kubernetes manifests:** every pod has resource requests and limits, a non-root user, a read-only root filesystem where possible, no host mounts, no service account token unless needed, dropped capabilities, and a `NetworkPolicy` that denies by default (the "no route" for an agent workspace in the earlier design) and allows only what is named. Secrets come from the cluster's secret store, not from `values.yaml`.
- **Helm:** details in [general/helm-kubernetes.md](general/helm-kubernetes.md). Values are documented and have safe defaults; nothing in a template depends on a cloud-specific field outside a clearly named block; images are pinned by digest in release values; a chart upgrade is tested from the previous release's values. Sign images with `cosign` and publish a bill of materials with `syft` (target).
- **Pin everything.** Provider versions with `~>`, module versions, OpenTofu's `required_version`, Packer plugin versions, base-image filters with `owners`, chart dependencies. Commit `.terraform.lock.hcl` (OpenTofu uses the same lock file name). Never commit state, `.terraform/`, plan files or `.tfvars` with secrets.
- **OpenTofu is not Terraform.** Use `tofu`, and check a feature against OpenTofu's release notes before using it (the HashiCorp digests are written for Terraform: [general/terraform.md](general/terraform.md)).

## Layout (earlier code)

The earlier code's layout, kept as reference. The target has no layout for these tools yet:

| Folder | What |
| --- | --- |
| `infra/<cloud>/` | An OpenTofu root or module per cloud, with `policies/` (templates for roles and bucket policies, the one source of truth) and `tests/` (`tofu test`, mocked provider) |
| `infra/modules/sandbox/` | A sandbox module with its own `tests/` |
| `infra/images/` | Packer definitions of the earlier runner and workspace images, with a README |
| `charts/` | Helm charts for the server and the runner (earlier design) |
| `infra/contract.md` | What every earlier sandbox module had to accept, produce and guarantee |

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

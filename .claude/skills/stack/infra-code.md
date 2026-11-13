# Infrastructure code

OpenTofu and Packer live under `infra/`. Start with `infra/README.md` and `infra/contract.md` (what every sandbox module must accept, produce and guarantee).

| Folder | What |
| --- | --- |
| `infra/aws/` | The OpenTofu module of the evaluator sandbox (AWS), with `policies/` (IAM and bucket templates, the one source of truth) and `tests/` (`tofu test`, mocked provider) |
| `infra/modules/sandbox/` | A sandbox module with its own `tests/machines.tftest.hcl` (AWS resources; CI does not run its tests) |
| `infra/images/` | Packer definitions of the runner and bench machine images; see `infra/images/README.md` |
| `infra/tests/` | A vitest suite: renders the policies, simulates IAM decisions, checks module wiring |

## Checks (from CI)

```bash
cd infra/aws
tofu fmt -check -recursive
tofu init -backend=false -input=false
tofu validate
tofu test
```

CI pins OpenTofu 1.8.5. For images:

```bash
packer fmt -check -diff infra/images
mkdir -p infra/images/dist
packer init infra/images
packer validate -var image_version=ci infra/images
```

CI pins Packer 1.14.2 and runs this when `infra/images/**`, `crates/**`, `templates/**` or `Cargo.toml` change. It makes no AWS calls. Building an image or applying a module spends money; see [land/real-cloud.md](../land/real-cloud.md).

The `infra/tests` suite runs with the pnpm checks in [typescript.md](typescript.md).

Rules:

- The module never sees model keys or tokens. State holds no secrets.
- The sealed bucket denies agent roles. Do not widen a policy to get a test green.
- A new cloud is a product-level change; see [infra/add-cloud.md](../infra/add-cloud.md).

# Terraform style, tests and module refactoring (digest)

Source: the `terraform-style-guide`, `terraform-test` and `refactor-module` skills in [hashicorp/agent-skills](https://github.com/hashicorp/agent-skills) (plugins/terraform/skills), by HashiCorp (MPL-2.0, [`../licenses/hashicorp-agent-skills-LICENSE`](../licenses/hashicorp-agent-skills-LICENSE); the skills carry a "Copyright IBM Corp. 2026" notice). The style guide is based on HashiCorp's [Terraform style guide](https://developer.hashicorp.com/terraform/language/style). This digest is written in our own words and keeps no code verbatim beyond one-line fragments.

**These skills are written for Terraform and apply to OpenTofu.** OpenTofu reads the same HCL and has the same `fmt`, `validate`, `test` and `moved`. Where the source says `terraform`, run `tofu`. Where it gives a Terraform version constraint (it uses `>= 1.14`), use OpenTofu's own version from [`../../libraries.md`](../../libraries.md); the two products' version numbers are different. Check a feature (write-only attributes, ephemeral resources, `mock_provider`, `parallel` runs) against the OpenTofu release notes before relying on it. [`../ratchet.md`](../ratchet.md) wins where they differ.

## Style

**Files.** One concern per file: `versions.tf` (or `terraform.tf`) for the required version and providers, `providers.tf`, `main.tf` for the main resources and data sources, `variables.tf` and `outputs.tf` (declarations in alphabetical order), `locals.tf`. Split `main.tf` by topic (`network.tf`, `iam.tf`) when it grows.

**Formatting.** Two spaces, no tabs, equals signs aligned in a run of arguments. Inside a block: meta-arguments (`count`, `for_each`, `provider`, `depends_on`) first, then arguments, then nested blocks, then `lifecycle` last. `tofu fmt -recursive` is the authority; never argue with it.

**Names.** Lowercase with underscores. Name a resource for what it is, not its type (`aws_instance.web_api`, not `aws_instance.web_api_instance`), singular, and use `main` when there is exactly one and no better name. Variables name the thing, with its unit when it has one (`timeout_seconds`).

**Variables and outputs.** Every variable has a `type` and a `description`; add `validation` blocks for values that have a closed set or a format; mark secrets `sensitive = true`. Every output has a `description`; mark outputs derived from secrets `sensitive`. Use `object({...})` types for a coherent group of inputs.

**Repeated resources.** Prefer `for_each` over `count`: instances are addressed by a stable key, so removing the middle one does not renumber the rest and force replacements. Keep `count` for the on/off switch (`count = var.enabled ? 1 : 0`).

**Versions.** Pin the language version and every provider in `required_providers` (`~> 6.0` allows the rightmost component to rise). Use the latest major of each provider unless a lock file or another module forbids it. Commit `.terraform.lock.hcl`. Never commit state, `.terraform/`, plan files or `.tfvars` files that hold secrets.

**Tags.** Put the common tags in the provider's `default_tags` (or one `local.common_tags` merged in) so every resource carries them, and add only the resource-specific `Name` on the resource.

**Security.** Encrypt at rest by default; keep networks private where you can; grant least privilege in security groups and policies; turn on logging; never hard-code credentials; mark sensitive outputs. For a bucket: versioning, server-side encryption, and a public-access block with all four settings on. For secrets, in priority order: (1) let the resource manage the secret in a secrets manager itself (for a database, the "manage the master password" option) so the value never enters your configuration; (2) pass the value through an `ephemeral` resource into a write-only attribute (`*_wo` with a version counter) so it is never stored in state; (3) otherwise treat state as secret and encrypt and restrict it. State holds plaintext values in every format: never print it.

**Review checklist.** Formatted; validated; standard file layout; variables have type and description; outputs have descriptions; names are descriptive nouns with underscores; versions pinned; secrets marked sensitive; no credentials in code; security practices applied. Linters named by the source: `tflint` for style, `checkov` or `tfsec` for security (not required here; adding one is a tooling decision).

## Tests (`tofu test`)

The built-in framework runs `.tftest.hcl` (or `.json`) files in a module's `tests/` directory (or the module root).

- **Structure.** A file has optional `variables {}` (highest precedence), optional `provider` blocks, and one or more `run "name" { ... }` blocks, run in order. Each run has a `command` (`plan` or `apply`), optional `variables`, optional `module { source = "./..." }` (local or registry only), and `assert { condition, error_message }` blocks. Later runs can read earlier runs' outputs through `run.<name>.<output>`.
- **Plan mode is the unit test.** `command = plan` checks logic (counts, tags, outputs, conditional resources, variable validation) in seconds with no resources created. `command = apply` (the default) creates and then destroys real resources; keep it for a few integration tests. Name files `*_unit_test.tftest.hcl` and `*_integration_test.tftest.hcl` so CI can select by pattern.
- **Mock providers** (`mock_provider "aws" { mock_resource ... { defaults = {...} } mock_data ... }`) simulate a provider so plan-mode tests need no credentials and no network. Give mocks realistic computed values (ids, ARNs) so assertions on them are meaningful. Mocks only work with plan runs in the source's description; check the OpenTofu docs for how apply runs behave.
- **Expected failures.** `expect_failures = [var.environment]` asserts that a validation rejects a bad value: the run passes only if that failure occurs.
- **Plan options.** `plan_options { mode = refresh-only, refresh, replace, target }` for special plans.
- **State isolation.** Runs share state through the file unless a `state_key` says otherwise; `parallel = true` runs independent runs concurrently (and `test { parallel = true }` for the whole file).
- **Cleanup.** Resources are destroyed in reverse run order at the end, so a child (objects in a bucket) must be created in a later run than its parent. `-no-cleanup` keeps them for debugging, and then they cost money until removed by hand.
- **Run.** `tofu test`, one file, `-filter=<run name>`, `-test-directory=<dir>`, `-verbose`. Assert on behaviour you care about (an agent role cannot read the sealed bucket), not on the provider's defaults.
- **CI.** Run plan-mode tests on every change; run apply-mode tests on a schedule or by hand with a dedicated account and a budget, because they spend money.

## Module refactoring

Use when a flat configuration repeats itself or two things have different lifecycles.

1. **Analyse.** Group resources by function (network, compute, data), find repeated patterns and the cross-references between groups.
2. **Design the interface first.** Typed, validated variables in; documented outputs out; sensible defaults; a `tags` map input. Encapsulate resources that are tightly coupled or share a lifecycle (a VPC and its subnets). Keep cross-cutting concerns and things with different lifecycles outside (monitoring, tagging policy, provider configuration).
3. **Do not over-abstract.** A module with a hundred switches is worse than none. Prefer specific, typed interfaces and small, focused modules composed by the root. Do not couple sibling modules by reaching into each other's resources: pass values through the root module.
4. **Keep the state.** Before moving anything, list current addresses (`tofu state list`, or `tofu show -json` when providers are installed) rather than reading the raw state file, which is not a stable format and holds secrets. Record each move in a `moved { from = ..., to = module.x.... }` block, then run a plan: it must show no changes. Only then apply.
5. **Document and test.** Each module has a README (overview, usage, requirements, inputs, outputs, examples) and a `tests/` directory (above). Version modules that are shared; a module used from one place stays local.

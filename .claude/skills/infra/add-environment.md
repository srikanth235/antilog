# Add an environment

An environment is a machine type and an image. Today the bench machine type is `evaluator.hardware` or `environment.machine` in `ratchet.yaml` (`crates/ratchet-core/src/config/schema.rs`), and a setup agent can draft a fuller environment (`crates/ratchet-env/src/`).

1. **A name and an id.** Give the environment a stable id. Benchmarks refer to it by id, never by instance type. (target)
2. **A machine type.** Name the machine type or types. Add each to the allow list the module reads (`allowed_instance_types` in `infra/contract.md`). A benchmark stays on one machine type.
3. **An image.** Build or choose the image. Runner and bench images are built by Packer from `infra/images/`. A repo image comes from the repo's Dockerfile or devcontainer, which is a protected path.
4. **Both sides.** Say whether it is a workspace environment (agents propose) or an evaluator environment (measuring). They are never the same machines.
5. **Validate.** Run the environment through the checks that exist: `crates/ratchet-env/src/validate.rs` and the plan check in `crates/ratchet-sandbox/src/sandbox/environment_check.rs`.
6. **Cost it.** Show the estimated spend before approval (`crates/ratchet-env/src/cost.rs`). The environment is locked once the user approves (`crates/ratchet-env/src/approve.rs`).
7. **Tag and clean up.** Every machine is tagged on launch and removed by the reaper. The module for drafted environments is `infra/modules/sandbox/`.
8. **No provider words upward.** What optimization sees is the id, the machine class and the hourly cost, not the region or instance id.
9. **Tests.** Add a case to `crates/ratchet-env/tests/validate.rs` and a `tofu test` for any new machine shape (`infra/modules/sandbox/tests/machines.tftest.hcl`), and rebuild the images in CI (`.github/workflows/images.yml`).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

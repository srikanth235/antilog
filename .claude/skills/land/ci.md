# CI

Workflows are in `.github/workflows/`. Read the file before guessing why a job failed.

| Workflow | When | What it runs |
| --- | --- | --- |
| `ci.yml` (CI) | every push and pull request | `node`: `pnpm install --frozen-lockfile`, `pnpm build`, `pnpm typecheck`, `pnpm test`. `infra`: in `infra/aws`, `tofu fmt -check -recursive`, `tofu init -backend=false -input=false`, `tofu validate`, `tofu test` (OpenTofu 1.8.5). `demos`: builds the three demos, runs `runtime_simulation` of `ratchet-engine` with `RATCHET_E2E=1`, then `ratchetd simulate` for search, forecast and cli |
| `native.yml` (Native) | push and pull request (see the file for path filters) | `rust` on Ubuntu and macOS: fmt, clippy with warnings denied, `cargo test --workspace --no-fail-fast`, then the CLI contract script on the built binaries |
| `images.yml` (Images) | push and pull request touching `Cargo.toml`, `crates/**`, `templates/**`, `infra/images/**`; also by hand | builds the static launcher and runner binaries per architecture, then `packer fmt` and `packer validate` (Packer 1.14.2, no AWS calls) |
| `harness-canary.yml` | by hand only (`workflow_dispatch`) | installs the latest agent CLIs, records fresh transcripts and runs the conformance tests. The nightly schedule is commented out on purpose: it spends provider credit |

Notes:

- Only CI can run the macOS Rust job and the Packer validation. Say so in the report when you could not.
- `harness-canary.yml` is never started without the owner's go-ahead.

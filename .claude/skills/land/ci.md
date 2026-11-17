# CI

`.github/workflows/checks.yml` runs on every push to every branch and on every pull request. Agents push often, so CI, not a local hook, is the gate: hooks can be skipped and a fresh clone may not have them on.

| Job | Runs |
| --- | --- |
| commits | `.githooks/commit-msg` on every commit since the default branch (the new commits only, on the default branch) |
| files | `.githooks/pre-commit --all`: secrets, stray and oversized files, em dashes, model names, and `python3 design/tools/tokens.py --check` |

Add each job below with the first code of its layer, and keep the local hooks fast: anything slower than a second belongs here, not in a hook.

## Target jobs

One job per layer, each running the commands in [checks.md](checks.md). A pull request runs the `pr` gate profile.

| Job | Runs |
| --- | --- |
| rust | fmt, clippy with warnings denied, tests, `cargo deny`, on Linux and on macOS (the CLI builds for both) |
| api | the OpenAPI document and generated client are current |
| storage | the storage suite against Postgres (a container); migrations from empty and from the last release |
| deploy-config | `helm lint` and `helm template` per cloud, `tofu validate` per module, and an install of the chart on k3s with a smoke test |
| web | typecheck, lint, tests |
| macos-app | build and test the Swift app, on a macOS runner only |
| platform | the platform test suite against the fake platform |
| judge | the acceptance criteria tests, the seal checks against the fake judge sandbox, the protected-path test |
| release | builds the programs (`ratchet-server`, `ratchet`), the container image and the Helm chart; signs them and attaches an SBOM |

Rules:

- Only CI can run the macOS jobs and the image build. Say in the report when you could not run them.
- A job that spends money (a real-cloud run, an agent canary) is never scheduled or started without the owner's go-ahead; see [real-cloud.md](real-cloud.md).
- A step that is skipped for a missing tool is a loud skip locally and a failure in CI.

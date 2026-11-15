# CI

`.github/workflows/checks.yml` runs on every push to every branch and on every pull request. Agents push often, so CI, not a local hook, is the gate: hooks can be skipped and a fresh clone may not have them on. The earlier code's workflows are in [today.md](today.md).

| Job | Runs |
| --- | --- |
| commits | `.githooks/commit-msg` on every commit since the default branch (the new commits only, on the default branch) |
| files | `.githooks/pre-commit --all`: secrets, stray and oversized files, em dashes, model names, and `python3 design/tools/tokens.py --check` |

Add each job below with the first code of its layer, and keep the local hooks fast: anything slower than a second belongs here, not in a hook.

## Target jobs

One job per layer, each running the commands in [checks.md](checks.md). A pull request runs the `pr` gate profile.

| Job | Runs |
| --- | --- |
| rust | fmt, clippy with warnings denied, tests, `cargo deny`, on Linux and on macOS (the runner and CLI build for both) |
| wasm | `ratchet-core` builds for `wasm32-unknown-unknown` as well as natively, on every commit, so a tokio, `sqlx` or Cloudflare import in the core fails at once |
| api | the OpenAPI document and generated client are current |
| storage | the one storage suite against Postgres (a container) and against SQLite; both dialects' migrations; the dialects agree on tables and columns |
| web | typecheck, lint, tests |
| macos-app | build and test the Swift app, on a macOS runner only |
| infra | `tofu fmt`, `validate`, `test`; `packer fmt` and `validate`; the backend contract suite against fakes |
| release | builds the three programs and the container images; signs them |

Rules:

- Only CI can run the macOS jobs and image builds. Say in the report when you could not run them.
- A job that spends money (a real-cloud run, an agent canary) is never scheduled or started without the owner's go-ahead; see [real-cloud.md](real-cloud.md).
- A step that is skipped for a missing tool is a loud skip locally and a failure in CI.

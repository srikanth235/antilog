# CI

There is no CI on this branch: `.github/workflows/` does not exist yet. Add the workflow with the first code, and rewrite this page in the same change. The earlier code's workflows are in [today.md](today.md).

## Target jobs

One job per layer, each running the commands in [checks.md](checks.md). A pull request runs the `pr` gate profile.

| Job | Runs |
| --- | --- |
| rust | fmt, clippy with warnings denied, tests, `cargo deny`, on Linux and on macOS (the runner and CLI build for both) |
| api | the OpenAPI document and generated client are current |
| postgres | migrations and tests against a Postgres container |
| web | typecheck, lint, tests |
| macos-app | build and test the Swift app, on a macOS runner only |
| infra | `tofu fmt`, `validate`, `test`; `packer fmt` and `validate`; the backend contract suite against fakes |
| design | `python3 design/tools/tokens.py --check` |
| release | builds the three programs and the container images; signs them |

Rules:

- Only CI can run the macOS jobs and image builds. Say in the report when you could not run them.
- A job that spends money (a real-cloud run, an agent canary) is never scheduled or started without the owner's go-ahead; see [real-cloud.md](real-cloud.md).
- A step that is skipped for a missing tool is a loud skip locally and a failure in CI.

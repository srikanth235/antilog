# Context: what exists today, and what is target

Almost all of this area is target. There is no dataset, source, grant or marking object in the repo. Every path below was checked to exist.

## Exists, and the nearest thing to each target object

| Target object | What exists | Where |
| --- | --- | --- |
| source | Connectors: signed-in accounts of three kinds (GitHub, AWS, agent logins), one default per kind, with secrets in the macOS Keychain or a 0600 file | `crates/ratchet-connectors/src/model.rs`, `crates/ratchet-connectors/src/secrets.rs`, `crates/ratchet-connectors/src/store/` |
| source, GitHub | Device-flow sign-in, repos, branches, files, a git host with pull requests | `crates/ratchet-github/src/api.rs`, `crates/ratchet-github/src/github_host.rs`, `crates/ratchet-github/src/signin.rs` |
| read through a connector | `connector_query`, a read-only question to a connector the user signed in; refused on a cloud runner, so credentials never reached an agent | `crates/ratchetd/src/real/service.rs`, `crates/ratchet-agents/src/runner_mcp.rs` |
| holdout | `evaluator.holdout` (alias `confirm_trace`) names hidden data in `ratchet.yaml`; the sealed bucket is readable only by the confirm identity | `crates/ratchet-core/src/config/schema.rs`, `crates/ratchet-bench/src/pipeline.rs`, `infra/contract.md`, `infra/aws/s3.tf` |
| protected data and files | Protected globs; the evaluator's files are overlaid from a protected ref so a candidate cannot change them | `crates/ratchet-core/src/protected_set.rs`, `crates/ratchet-bench/src/overlay.rs` |
| note | Steering notes agents read at the start of a turn | `crates/ratchet-mcp/src/sets.rs`, `docs/ratchet-mcp.md` |
| history | Candidates with verdicts and results in the ledger; the `history` agent tool returns them | `crates/ratchet-ledger/src/store.rs`, `crates/ratchet-mcp/src/sets.rs` |
| write-back action | Opening a pull request with the verified wins, by the chat tool `open_pr` | `crates/ratchet-mcp/src/sets.rs`, `crates/ratchet-github/src/github_host.rs` |
| secrets never in the clear | Keychain service and refusal to store a secret in plaintext | `crates/ratchet-connectors/src/secrets.rs` |

Tests: `crates/ratchet-connectors/tests/connectors.rs`, `crates/ratchet-github/tests/`, `crates/ratchet-agents/tests/ratchet_mcp.rs`, `infra/tests/src/agent-boundary.test.ts`. CI: `.github/workflows/ci.yml` (infra and demos), `.github/workflows/native.yml` (Rust tests).

## Gaps in what exists

- The holdout is a name in a config, not a marking on a versioned dataset. Enforcement is the bucket policy and the evaluator ref, not a grant check.
- Runs do not record dataset versions.
- The mapping from the code host to a source is not made: GitHub is a connector and a git host today.

## Target, not built

- Skill pack as a stored, versioned object, chosen per experiment and recorded on the receipt. Today nothing like it exists; agent instructions live in repo files and prompts.

- Source, dataset, dataset version and virtual dataset as stored objects.
- Source kinds: Snowflake, BigQuery, Postgres, object storage, GitLab, monitoring, any MCP server. (GitHub exists only as a connector.)
- Grants with audiences, and markings enforced in one place.
- Short-lived, per-run read access to granted datasets.
- History stored as a dataset.
- Write-back actions as named, audited actions.

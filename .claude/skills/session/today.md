# Session: what the earlier code did, and what it taught

The earlier Rust daemon and CLI live on the branch `claude/peaceful-lamport-leak6p`. They are reference, not a base. Nothing below exists on this branch. Every path named is on that branch, and the words (experiment, run, drafting session, evaluator, optimization, note) are the earlier design's, not the target's. The earlier design also had skill packs, plugins, Customize and projects; the target's are in [model.md](model.md), and nothing below is their code.

## What it built

| Piece | Where |
| --- | --- |
| The sandbox contract: inputs, outputs and invariants every cloud module satisfied | `infra/contract.md` |
| The AWS module (OpenTofu): VPC, buckets (artifacts and sealed), agent permission boundary, bench and confirm identities, DNS firewall, optional egress proxy | `infra/aws/` |
| Machine images (Packer): runner and bench AMIs | `infra/images/` |
| Sandbox lifecycle (plan, apply, destroy, status; mock provider) and compute (launch, list by tag, terminate) | `crates/ratchet-sandbox/src/sandbox/`, `crates/ratchet-sandbox/src/compute/` |
| Tags on every machine: optimization, agent, role, sandbox, purpose | `crates/ratchet-sandbox/src/util.rs` |
| The reaper: lifetime and per-agent instance limits, clean-up of ended work | `crates/ratchet-sandbox/src/reaper.rs`, `crates/ratchet-engine/src/reaper.rs` |
| Spend: estimate, append-only ledger events, billing reconcile; budget stop and 80 percent warnings | `crates/ratchet-core/src/cost.rs`, `crates/ratchet-sandbox/src/cost_explorer.rs`, `crates/ratchet-engine/src/tick.rs` |
| Agent adapters (Claude Code, Codex, Grok), local detection, worktrees, the runner binary, fallback order | `crates/ratchet-agents/src/` |
| Environments drafted by a setup agent, validated, costed and locked on approval | `crates/ratchet-env/src/` |
| Connectors: signed-in accounts (GitHub, AWS, agent logins) with secrets in the macOS Keychain or a 0600 file | `crates/ratchet-connectors/src/` |
| GitHub: device-flow sign-in, repos, branches, a git host with pull requests | `crates/ratchet-github/src/` |
| `connector_query`, a read-only question to a signed-in connector; refused on a cloud runner so credentials never reached an agent | `crates/ratchetd/src/real/service.rs`, `crates/ratchet-agents/src/runner_mcp.rs` |
| Steering text agents read at the start of a turn, the `history` tool, and `open_pr` | `crates/ratchet-mcp/src/sets.rs`, `docs/ratchet-mcp.md` |
| Ledger of sandboxes and spend | `crates/ratchet-ledger/src/store.rs` |

## What it taught

- **Spend** worked as append-only rows with a budget stop. That carries over. It was tied to the earlier optimization, not to a session, and the cap was enforced by an engine tick, which the target replaces with the judge refusing `measure`.
- **Tag everything.** Tags on every machine made clean-up possible. Launch failed closed on a missing tag. Keep this, keyed to the session.
- **Clean-up must be verified.** A reaper that lists by tag and deletes is the model. Finished with resources left was a real leak.
- **Credentials.** Secrets in a Keychain or a file worked for one machine and could not reach a platform. The target moves every credential into the platform's credential store. `connector_query` refusing on a cloud runner was the right instinct: credentials never reached an agent.
- **Sealed isolation as buckets.** The holdout was a bucket the evaluator identity reached and agents were denied twice (store policy and agent boundary). The target replaces it with a credential only the judge sandbox holds.
- **Where work ran** was This machine or Cloud, and the provider a two-value enum. Both were too narrow: the target has five platforms behind one trait.
- **Agent CLIs on the user's machine** gave agents the user's own login. The target runs agents as platform sessions with scoped credentials.
- **Environments drafted by an agent** and locked on approval was the seed of `set_environment`. The target makes it a card in the chat.
- **Steering text and the `history` tool** worked: agents read what was tried and did not repeat it. The target keeps `history` and drops the separate steering feature; a person talks to the agent in the chat.
- **Opening a pull request** was a chat tool with no audit entry. The target makes it `deliver`, an approved and audited call.
- **Connectors were accounts, not data systems.** The earlier code had no datasets and no read path to a warehouse. The target's connectors are the systems an agent's code reads, with read-only credentials that leave out the holdout.
- **Agent instructions lived in repo files and prompts**, with no one place and no test. The target keeps them in one versioned, tested place ([prompts.md](prompts.md)).

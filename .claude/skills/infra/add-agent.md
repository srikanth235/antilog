# Add an agent

An agent is a worker that proposes changes, such as a new coding-agent CLI. Today they are Claude Code, Codex and Grok (`AgentKind` in `crates/ratchet-core/src/enums.rs`).

1. **An adapter.** Implement the adapter that starts the CLI and maps its event stream to Ratchet's agent events. Model it on `crates/ratchet-agents/src/claude_code.rs`, `crates/ratchet-agents/src/codex.rs` and `crates/ratchet-agents/src/grok.rs`, with the shared pieces in `crates/ratchet-agents/src/cli_agent.rs` and `crates/ratchet-agents/src/process.rs`.
2. **Detection.** The app finds the CLI and its sign-in state without asking the user to type a path (`crates/ratchet-agents/src/detect.rs`).
3. **Model provider.** Say how the agent's model is served: a subscription or an API key. Subscription runs record tokens and no dollars (the `billing` field of a spend event in `crates/ratchet-core/src/rows.rs`). A model provider as its own stored object is (target).
4. **No source credentials.** The agent gets no connector secret and no holdout. A runner refuses `connector_query` (`crates/ratchet-agents/src/runner_mcp.rs`).
5. **Where it runs.** On the user's machine or in the workspace (This machine or Cloud, in `docs/GLOSSARY.md`). The agent is never placed on an evaluator machine.
6. **Spend.** Every turn records tokens and cost against the run. A cap set by optimization stops it.
7. **Fallback.** Add the agent to the fallback order rules (`crates/ratchet-agents/src/fallback.rs`, `crates/ratchet-core/src/agents_policy.rs`). An optimization chooses which agents and how many; infra only runs them.
8. **Words.** Name it in `docs/GLOSSARY.md` and [model.md](model.md).
9. **Tests.** Record fixtures with `scripts/record-fixtures.sh` and run the conformance suite in `crates/ratchet-agents/tests/conformance.rs`. The weekly canary is `.github/workflows/harness-canary.yml`. The harness notes are in `docs/harness-matrix.md`.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

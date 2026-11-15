# Add an agent

An agent is a worker that proposes changes, such as a new coding-agent CLI. Today they are Claude Code, Codex and Grok (`AgentKind` in `crates/ratchet-core/src/enums.rs`).

1. **An adapter.** Implement the adapter that starts the CLI and maps its event stream to Ratchet's agent events. Model it on `crates/ratchet-agents/src/claude_code.rs`, `crates/ratchet-agents/src/codex.rs` and `crates/ratchet-agents/src/grok.rs`, with the shared pieces in `crates/ratchet-agents/src/cli_agent.rs` and `crates/ratchet-agents/src/process.rs`.
2. **Detection.** The app finds the CLI and its sign-in state without asking the user to type a path (`crates/ratchet-agents/src/detect.rs`).
3. **Model provider.** Say how the agent's model is served: a subscription or an API key. Subscription runs record tokens and no dollars (the `billing` field of a spend event in `crates/ratchet-core/src/rows.rs`). A model provider as its own stored object is (target).
4. **Scoped credentials only.** The agent gets short-lived, read-only connector credentials minted per run, never the holdout, a long-lived or write credential, or a connector admin secret. Its code reads data through connectors itself. Model keys come through the egress proxy and git access is a short-lived scoped token. Today a runner refuses `connector_query` (`crates/ratchet-agents/src/runner_mcp.rs`).
5. **Where it runs.** On the user's machine today (target: the Mac app's "This machine" backend, with scoped credentials like any other), or in a workspace sandbox in the customer's infrastructure (This machine or Cloud, in `docs/GLOSSARY.md`). The agent is never placed on an evaluator machine.
6. **Spend.** Every turn records tokens and cost against the run and the agent. A cap set by optimization, starts as even shares across the run's agents and stops it at its share; a stopped agent's unspent share goes to the agents still iterating (target).
   **The loop (target).** The agent runs its own loop in its own sandbox and asks for a measurement with `ratchet measure` or the `measure` MCP tool (see [../ratchet/SKILL.md](../ratchet/SKILL.md)). It reads the history, and reads a note before its next trial.
7. **Fallback.** Add the agent to the fallback order rules (`crates/ratchet-agents/src/fallback.rs`, `crates/ratchet-core/src/agents_policy.rs`). An experiment chooses which agents and how many; infra only runs them.
8. **Words.** Name it in `docs/GLOSSARY.md` and [model.md](model.md).
9. **Tests.** Record fixtures with `scripts/record-fixtures.sh` and run the conformance suite in `crates/ratchet-agents/tests/conformance.rs`. The weekly canary is `.github/workflows/harness-canary.yml`. The harness notes are in `docs/harness-matrix.md`.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

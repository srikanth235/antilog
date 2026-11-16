# Add an agent

An agent is a model on an agent platform (target): Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AWS AgentCore. The earlier code ran agent CLIs on the user's machine (`AgentKind` in `crates/ratchet-core/src/enums.rs`, adapters in `crates/ratchet-agents/src/`); see [today.md](today.md).

1. **Its platform.** Name the platform it runs on. If the platform is new, add it first ([add-platform.md](add-platform.md)).
2. **Its model.** Name the model and how the platform reaches it. On AgentCore, a model outside Bedrock needs a key in AgentCore Identity under the platform account. No model key ever enters a sandbox.
3. **Its definition.** The instructions, tools and skill packs the agent gets, and the `measure` tool. Keep the definition the same across platforms where the platform allows, so agents compare fairly.
4. **Credentials.** Read-only connector credentials that exclude the holdout and a short-lived git token for its own branch, through the platform's credential store. Never the holdout, a write credential or a connector admin secret.
5. **Spend.** Price its tokens. Every turn's usage is recorded against the run and the agent. The run's budget starts as even shares across its agents and stops each at its share; a stopped agent's unspent share goes to the agents still iterating.
6. **The loop.** The agent runs its own loop in its own run session, asks for a measurement with the `measure` tool, reads the history, and reads a note before its next trial (see [../optimization/model.md](../optimization/model.md)).
7. **Words.** Name it in `docs/GLOSSARY.md` and [model.md](model.md).
8. **Tests.** Add it to the platform contract suite's agent cases, with recorded events.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

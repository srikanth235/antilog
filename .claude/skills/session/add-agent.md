# Add an agent

An agent is a model on an agent platform (target): Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AWS AgentCore, GitHub Copilot (the Copilot SDK) on Microsoft Foundry Agent Service, Gemini (ADK) on Google Agent Runtime. Gemini also stays available on AgentCore. The earlier code ran agent CLIs on the user's machine (`AgentKind` in `crates/ratchet-core/src/enums.rs`, adapters in `crates/ratchet-agents/src/`); see [today.md](today.md).

1. **Its platform.** Name the platform it runs on. If the platform is new, add it first ([add-platform.md](add-platform.md)).
2. **Its model.** Name the model and how the platform reaches it. On AgentCore, a model outside Bedrock needs a key in AgentCore Identity under the platform account. No model key ever enters a sandbox. On Microsoft Foundry Agent Service and Google Agent Runtime the platform runs a container image Ratchet supplies, so the agent comes with one agent image holding its own harness.
3. **Its definition.** The session brief and tool descriptions from [prompts.md](prompts.md), and the Ratchet tools. Keep the definition the same across platforms where the platform allows, so agents compare fairly. No agent's vendor is privileged.
4. **Credentials.** Read-only connector credentials that exclude the holdout and a short-lived git token for its own branch, through the platform's credential store. Never the holdout, a write credential or a connector admin secret.
5. **Spend.** Price its tokens. Every turn's usage is recorded against the session and the agent, and counts against the experiment's budget, shared with its other sessions.
6. **The loop.** The agent runs its own loop in its session, asks for a measurement with `measure`, reads `history` before its next trial, and may `spawn` other agents (see [tools.md](tools.md)).
7. **Second opinion.** If it can read a diff, say whether it may give the second opinion. It must be a different model provider from the proposer (see [../judge/anti-cheat.md](../judge/anti-cheat.md)).
8. **Words.** Name it in `docs/GLOSSARY.md` and [model.md](model.md).
9. **Tests.** Add it to the platform test suite's agent cases, with recorded events.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

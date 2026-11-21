# Add an agent

An agent is a model run by a harness. In v0 the harness is OpenCode ([docs/V0.md](../../../docs/V0.md)). Built: `antilog-core/src/agents.rs` (the agents an organization turned on, and Auto) and the `/agents` routes.

1. **Its model.** Name the model and the provider key the organization saved for it. No model key ever enters a session machine; the proxy swaps the session's token for the key (see [docs/V0.md](../../../docs/V0.md)).
2. **Its definition.** The tool descriptions and endpoint instructions from [prompts.md](prompts.md), and the setup tools. Keep the definition the same across agents, so agents compare fairly. No agent's vendor is privileged.
3. **Credentials.** Connector tools through the session endpoint, never a connector credential. Never a model key.
4. **Spend and cost tier.** Price its tokens. Every turn's usage is recorded against the session and the agent and counts against the session's, project's and organization's caps. The cost tier ("$" to "$$$") is set with the agent and shown wherever the agent is chosen.
5. **Auto.** It is an Auto choice only when an owner turned it on or set it as the default. Adding an agent never makes it the default.
6. **Customize.** It is listed in Customize, Agents (boards `Agents`, `AgentsEmpty`, `AgentsDiscover`, `AgentDetail`, `AddAgent`; layout in [../design/behaviour.md](../design/behaviour.md)). A plugin may recommend an agent, but adding one is always a person's action. There is no agent order and no per-agent budget share.
7. **Words.** Name it in `docs/GLOSSARY.md` and [model.md](model.md).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

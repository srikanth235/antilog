# Add an agent

An agent is a model on an agent platform (target): Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AWS AgentCore, GitHub Copilot (the Copilot SDK) on Microsoft Foundry Agent Service, Gemini (ADK) on Google Agent Runtime.

1. **Its platform.** Name the platform it runs on. If the platform is new, add it first ([add-platform.md](add-platform.md)).
2. **Its model.** Name the model and how the platform reaches it. On AgentCore, a model outside Bedrock needs a key in AgentCore Identity under the platform account. No model key ever enters a sandbox. On Microsoft Foundry Agent Service and Google Agent Runtime the platform runs a container image Antilog supplies, so the agent comes with one agent image holding its own harness.
3. **Its definition.** The session brief and tool descriptions from [prompts.md](prompts.md), and the Antilog tools. Keep the definition the same across platforms where the platform allows, so agents compare fairly. No agent's vendor is privileged.
4. **Credentials.** Read-only connector credentials that exclude the holdout and a short-lived git token for its own branch, through the platform's credential store. Never the holdout, a write credential or a connector admin secret. Each session of the agent has its own session and branch.
5. **Spend and cost tier.** Price its tokens. Every turn's usage is recorded against the session and the agent, and counts against the experiment's budget, shared with its other sessions. From the price set its cost tier ("$" to "$$$"), shown wherever the agent is chosen, next to the measured figure ("about $2.40 per trial here"), which comes from the judge's measurements, not the price list.
6. **The loop.** The agent runs its own loop in its session, asks for a measurement with `measure`, reads `history` before its next trial, and may `spawn` other agents (see [tools.md](tools.md)).
7. **Scoreboard, Auto and hand off.** It appears in the agent scoreboard with no extra work (the figures come from the judge's measurements here). It is an Auto choice only when an owner turned it on, a session on it can be handed off and can receive a hand off (summary and history, same branch), and its platform failing offers Continue on another agent?. Adding an agent never makes it the default.
8. **Second opinion.** If it can read a diff, say whether it may give the second opinion. It must be a different model provider from the proposer (see [../judge/anti-cheat.md](../judge/anti-cheat.md)).
9. **Customize.** It is listed in Customize, Agents (boards `Agents`, `AgentsEmpty`, `AgentsDiscover`, `AgentDetail`, `AddAgent`; layout in [../design/behaviour.md](../design/behaviour.md)), with its cost tier and scoreboard row. A plugin may recommend an agent, but adding one is always a person's action. There is no agent order and no per-agent budget share: the experiment's budget covers every session.
10. **Words.** Name it in `docs/GLOSSARY.md` and [model.md](model.md).
11. **Tests.** Add it to the platform test suite's agent cases, with recorded events, including a hand off to and from it and a failover.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

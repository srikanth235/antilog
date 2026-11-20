# Add an agent

An agent is a model run by a harness (target). In v0 the harness is OpenCode ([docs/V0.md](../../../docs/V0.md)).

1. **Its model.** Name the model and the provider key the organization saved for it. No model key ever enters a session machine; the proxy swaps the session's token for the key (see [docs/V0.md](../../../docs/V0.md)).
2. **Its definition.** The session brief and tool descriptions from [prompts.md](prompts.md), and the Antilog tools. Keep the definition the same across agents, so agents compare fairly. No agent's vendor is privileged.
3. **Credentials.** Read-only connector credentials that exclude the holdout and a short-lived git token for its own branch, through the server (see [docs/V0.md](../../../docs/V0.md)). Never the holdout, a write credential or a connector admin secret. Each session of the agent has its own session and branch.
5. **Spend and cost tier.** Price its tokens. Every turn's usage is recorded against the session and the agent, and counts against the experiment's budget, shared with its other sessions. From the price set its cost tier ("$" to "$$$"), shown wherever the agent is chosen, next to the measured figure ("about $2.40 per trial here"), which comes from the judge's measurements, not the price list.
6. **The loop.** The agent runs its own loop in its session, asks for a measurement with `measure`, reads `history` before its next trial, and may `spawn` other agents (see [tools.md](tools.md)).
7. **Scoreboard, Auto and hand off.** It appears in the agent scoreboard with no extra work (the figures come from the judge's measurements here). It is an Auto choice only when an owner turned it on, a session on it can be handed off and can receive a hand off (summary and history, same branch), and its platform failing offers Continue on another agent?. Adding an agent never makes it the default.
8. **Second opinion.** If it can read a diff, say whether it may give the second opinion. It must be a different model provider from the proposer (see [../judge/anti-cheat.md](../judge/anti-cheat.md)).
9. **Customize.** It is listed in Customize, Agents (boards `Agents`, `AgentsEmpty`, `AgentsDiscover`, `AgentDetail`, `AddAgent`; layout in [../design/behaviour.md](../design/behaviour.md)), with its cost tier and scoreboard row. A plugin may recommend an agent, but adding one is always a person's action. There is no agent order and no per-agent budget share: the experiment's budget covers every session.
10. **Words.** Name it in `docs/GLOSSARY.md` and [model.md](model.md).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

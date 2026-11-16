# Add an agent platform

An agent platform runs an agent's loop and its sandbox as one durable session. Ratchet supports a platform through one adapter behind the platform trait (target). The order is fixed:

1. Claude Managed Agents.
2. AWS AgentCore harness, which runs one harness over many model providers and brings Gemini and Grok.
3. The OpenAI Agents API.
4. Microsoft Foundry Agent Service (hosted agents, on Azure). Natural agent: GitHub Copilot (Copilot SDK).
5. Google Agent Runtime (on Gemini Enterprise Agent Platform; it was Vertex AI Agent Engine until April 2026). Natural agent: Gemini (ADK).

The trait is designed against all five from the start. On the last two, the platform runs a container image Ratchet supplies, so Ratchet supplies one agent image per agent (the agent's own harness inside it). The platform still runs the sandbox; Ratchet still runs none.

Before the first adapter, one tiny experiment runs end to end on all five to prove the trait, the `measure` tool and the evaluator sandbox.

## Steps

1. **Implement the trait.** Start a session from a repo and branch with an agent definition, an environment, credentials and limits; send input and notes; read the session's events; answer a client tool call; run a command in a sandbox with no agent; stop and delete. Map the platform's events to Ratchet's agent events: turn started and ended, tool call, usage, waiting for input, ended, failed.
2. **The `measure` tool.** Declare it as a tool the client answers, and answer it from the event stream. A platform that needs to call Ratchet over the network to use a tool is not valid for self-hosted servers behind a firewall.
3. **Measuring platform or not.** Say whether the platform can run a command in a sandbox with no agent, with its own credential and no route from agent sessions. Only then may it be a measuring platform.
4. **Credentials.** Hand every credential through the platform's credential store: connector credentials, the git token, and the evaluator's credential only to evaluator sandboxes. Say how the platform hides the value from the sandbox (for example the Claude Managed Agents vault substitutes it at egress) and what it cannot hide. For Foundry Agent Service the session has its own Microsoft Entra identity, and Google Agent Runtime uses Google Cloud's own identity; the exact credential store of each is confirmed in the spike. No credential goes in a prompt, a repo file or Ratchet's database.
5. **Network.** Use the platform's allowed-hosts setting, where it has one, to let a session reach only its model, its connectors and the code host.
6. **Limits.** Map the run's limits onto the platform's own (turns, wall-clock time, tokens) as a second stop. Ratchet's budget check from usage events is the first.
7. **Spend.** Read tokens and sandbox time from the platform's events or usage records, price them, and write append-only spend rows against the run and the agent. Reconcile against the platform's bill.
8. **Tag and clean up.** Tag every session and sandbox with its run and agent where the platform allows, list them by tag, and delete them. Report what could not be deleted. A run is not finished until clean-up is verified.
9. **Data and retention.** Write down where the platform keeps transcripts and files, for how long, in which regions, and whether it offers zero data retention. Ratchet cloud lists it as a subprocessor. Foundry Agent Service reads data in the Azure region chosen; Google Agent Runtime in the Google Cloud region chosen.
10. **Platform words stay here.** Agent ids, vault ids, ARNs and regions appear in no other skill and in no public type outside the adapter.
11. **Same promises.** Run the checks in [owes.md](owes.md) against the new platform. A promise that holds on one platform and not another is a defect in the new adapter.
12. **Tests.** Pass the shared platform contract suite against a recorded fake of the platform, and add a CI job for it. A run on a real platform account needs the owner's go-ahead; see [../land/real-cloud.md](../land/real-cloud.md).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

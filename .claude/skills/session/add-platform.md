# Add an agent platform

An agent platform runs an agent's loop and its sandbox as one durable session. Ratchet supports a platform through one adapter behind the agent trait (target). The order is fixed:

1. Claude Managed Agents.
2. AWS AgentCore harness, which runs one harness over many model providers and brings Gemini and Grok.
3. The OpenAI Agents API.
4. Microsoft Foundry Agent Service (hosted agents, generally available, on Azure). Natural agent: GitHub Copilot (Copilot SDK).
5. Google Agent Runtime (on Gemini Enterprise Agent Platform; Google writes "Agent Runtime on Gemini Enterprise Agent Platform"; "Google Agent Runtime" is our display name. It was Vertex AI Agent Engine until April 2026, and the API resource is still `ReasoningEngine`). Natural agent: Gemini (ADK).

The trait is designed against all five from the start. On the last two, the platform runs a container image Ratchet supplies, so Ratchet supplies one agent image per agent (the agent's own harness inside it). The platform still runs the sandbox; Ratchet still runs none.

Before the first adapter, one tiny experiment is measured end to end on all five to prove the agent trait, the Ratchet tools served over the event stream and a judge sandbox that holds a credential no agent sees.

## Steps

1. **Implement the trait.** Start a session with repos, an agent, an environment, credentials and tools; send input; read the session's events; answer a client tool call; stop and delete. Map the platform's events to session events: turn started and ended, message, tool call, usage, waiting for input, ended, failed.
2. **The Ratchet tools.** Declare them as tools the client answers, and answer them from the event stream. A platform that needs to call Ratchet over the network to use a tool is not valid for self-hosted servers behind a firewall. Points to check: on Foundry the model emits a `function_call` and the caller returns a `function_call_output` with the same call id, but a documented 10-minute expiry for tool outputs may apply. On Google Agent Runtime, answering a client tool over the stream is not documented, and bidirectional streaming is in preview with a 10-minute limit per query. `measure` is asynchronous, which sidesteps both.
3. **Judge platform or not.** Say whether the platform can run a command in a sandbox with no agent, with its own credential and no route from agent sessions. Only then may it be a judge platform (see [../judge/model.md](../judge/model.md)). AWS AgentCore can. Claude Managed Agents, the OpenAI Agents API and Microsoft Foundry Agent Service cannot. Foundry's Code Interpreter is a tool the agent calls (Python only, no network); the likely path is a judge image deployed as a hosted agent that runs only the judge's code and no model, checked in the spike. Google Agent Platform has sandboxes that run without an agent (Code Execution, shell sandboxes, custom container sandboxes); check in the spike whether one can hold a credential the agent never sees.
4. **Credentials.** Hand every credential through the platform's credential store: connector credentials, the git token, and the judge's credential only to judge sandboxes. Say how the platform hides the value from the sandbox (for example the Claude Managed Agents vault substitutes it at egress) and what it cannot hide. On Foundry Agent Service each hosted agent has its own Microsoft Entra agent identity, and secrets reach it through a Foundry connection to Azure Key Vault, never in the image or environment variables. Google Agent Runtime uses the agent's Agent Identity (or its service account), with secrets read from Secret Manager. Agent Identity is partly in preview, so the exact credential store of each is confirmed in the spike. No credential goes in a prompt, a repo file or Ratchet's database.
5. **Network.** Use the platform's allowed-hosts setting, where it has one, to let a session reach only its model, its connectors and the code host.
6. **Limits.** Map the budget onto the platform's own limits (turns, wall-clock time, tokens) as a second stop. The judge refusing `measure` at the budget is the first.
7. **Spend.** Read tokens and sandbox time from the platform's events or usage records, price them, and write append-only spend rows against the session and the agent. Reconcile against the platform's bill.
8. **Tag and clean up.** Tag every session and sandbox with its session where the platform allows, list them by tag, and delete them. Report what could not be deleted. A session is not finished until clean-up is verified.
9. **Data and retention.** Write down where the platform keeps transcripts and files, for how long, in which regions, and whether it offers zero data retention. Ratchet cloud lists it as a subprocessor. The vendors promise only where data is stored at rest. Foundry Agent Service keeps it in the geography of the chosen Azure resource; where the model processes it depends on the model deployment type (Standard and Regional stay in the geography, Data Zone in the US or EU zone, Global anywhere). Google Agent Runtime keeps it in the location chosen (the US or EU multi-region, or a supported country); where it is processed is checked in the spike.
10. **Platform words stay here.** Agent ids, vault ids, ARNs and regions appear in no other skill and in no public type outside the adapter.
11. **Same promises.** Run the checks in [owes.md](owes.md) against the new platform. A promise that holds on one platform and not another is a defect in the new adapter.
12. **Tests.** Pass the shared platform test suite against a recorded fake of the platform, and add a CI job for it. A session on a real platform account needs the owner's go-ahead; see [../land/real-cloud.md](../land/real-cloud.md).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

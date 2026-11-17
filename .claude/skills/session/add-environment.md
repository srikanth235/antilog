# Add an environment

An environment is an image or setup script, a size and a network policy on an agent platform (target). A session runs on one. A judge sandbox runs on one too, on the judge platform.

1. **A name and an id.** Give the environment a stable id. Experiments refer to it by id, never by a platform's own environment id.
2. **Its platform.** Name the platform. An environment belongs to one platform.
3. **An image or a setup script.** Choose or build the image the platform accepts (for example AgentCore needs a `linux/arm64` container image; on Microsoft Foundry Agent Service and Google Agent Runtime the image is a container image Antilog supplies, the agent image for an agent session; Foundry hosted agents take a `linux/amd64` image in Azure Container Registry at 0.5 vCPU / 1 GiB, 1 / 2 or 2 / 4). A repo image comes from the repo's Dockerfile or devcontainer, which an experiment may list as a protected path.
4. **A size.** Name the CPU, memory and disk. An experiment stays on one judge platform and one environment, so measurements compare.
5. **Which side.** Say whether it is for agent sessions or for judge sandboxes. A judge environment never runs an agent.
6. **Network.** List the hosts it may reach, using the platform's allowed-hosts setting where it has one.
7. **Cost it.** Show the estimated spend per hour on the Change environment card before approval.
8. **Changed by a card.** An agent proposes a change with `set_environment`; a person approves it (see [tools.md](tools.md)). A change to a judge environment makes a new experiment version, because numbers do not carry across.
9. **No platform words upward.** What the rest of the product sees is the id, the size and the hourly cost.
10. **Tests.** Add a case to the platform test suite that starts a sandbox on it and removes it.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

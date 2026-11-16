# Add an environment

An environment is an image and a size on an agent platform, named by id (target). A session and an evaluator sandbox each run on one. The earlier code's environments were AWS machine types and Packer images; see [today.md](today.md).

1. **A name and an id.** Give the environment a stable id. Benchmarks refer to it by id, never by a platform's own environment id.
2. **Its platform.** Name the platform. An environment belongs to one platform.
3. **An image.** Choose or build the image the platform accepts (for example AgentCore needs a `linux/arm64` container image). A repo image comes from the repo's Dockerfile or devcontainer, which is a protected path.
4. **A size.** Name the CPU, memory and disk. A benchmark stays on one measuring platform and one environment.
5. **Which side.** Say whether it is for agent sessions or for evaluator sandboxes. An evaluator environment never runs an agent.
6. **Network.** List the hosts it may reach, using the platform's allowed-hosts setting where it has one.
7. **Cost it.** Show the estimated spend per hour before approval. The environment is locked once a run is approved.
8. **No platform words upward.** What optimization sees is the id, the size and the hourly cost.
9. **Tests.** Add a case to the platform contract suite that starts a sandbox on it and removes it.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

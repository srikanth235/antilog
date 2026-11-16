---
name: infra
description: >-
  Where does work happen, and what may it cost? Use for agent platforms (Claude Managed Agents, the OpenAI Agents API, AWS AgentCore) and their one trait, sessions (drafting sessions and runs), the evaluator sandbox where measuring happens, the measure tool, platform accounts and credential stores, environments (image and size), agents and their models, budgets and spend records, clean-up, and adding an agent platform, an agent or an environment.
---

# Infra

Infra runs the work on agent platforms and keeps it cheap and clean (target). Ratchet runs no sandboxes. An agent platform (Claude Managed Agents, the OpenAI Agents API, AWS AgentCore) runs an agent's loop and its sandbox as one durable session, and every platform sits behind one trait. Self-hosted, Ratchet uses the customer's own platform accounts; on Ratchet cloud, its own. A drafting session works out an experiment with a person and ends on a branch; a run starts one autonomous session per agent from that branch. When an agent asks to measure, Ratchet starts an evaluator sandbox on the measuring platform, with no agent in it, for that one trial. The earlier code ran its own sandboxes on AWS; [today.md](today.md) says what it built.

| If you are doing | Read |
| --- | --- |
| Deciding what this area promises | [owes.md](owes.md) |
| Looking up a word (agent platform, platform account, session, drafting session, run, evaluator sandbox, measuring platform, environment, credential store, spend record) | [model.md](model.md) |
| Checking a change against the rules that never bend | [rules.md](rules.md) |
| Finding what the earlier code built, and what is only target | [today.md](today.md) |
| Supporting a new agent platform | [add-platform.md](add-platform.md) |
| Supporting a new agent (a model on a platform) | [add-agent.md](add-agent.md) |
| Adding an environment (image and size) | [add-environment.md](add-environment.md) |
| What a benchmark asks of an environment, and who sets spend caps | [../optimization/SKILL.md](../optimization/SKILL.md) |
| What a connector credential may reach | [../context/SKILL.md](../context/SKILL.md) |
| Who may add platform accounts and environments | [../organization/SKILL.md](../organization/SKILL.md) |
| Choosing tools and layers for the change | [../stack/SKILL.md](../stack/SKILL.md) |
| Deciding whether a change belongs in the product | [../product/SKILL.md](../product/SKILL.md) |

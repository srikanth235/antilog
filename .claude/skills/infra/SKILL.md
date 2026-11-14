---
name: infra
description: >-
  Where does work happen, and what may it cost? Use for the launcher and the runner, sandbox backends (Kubernetes, then VMs on AWS, GCP, Azure), minting scoped credentials, environments (machine type, image), workspaces where agents propose, evaluators where measuring happens, agent workers and model providers, budgets and spend records, tagging and clean-up, the OpenTofu module and Packer images, and adding a cloud, an agent or an environment.
---

# Infra

Infra runs the work in the customer's own infrastructure and keeps it cheap and clean. One launcher starts and removes sandboxes behind one backend trait. Self-hosted, it is built into the server. On Ratchet cloud, the same code runs as a small runner in the customer's infrastructure and connects out to the hosted server. Proposing and measuring happen on different machines. A Mac running the macOS app may be the "This machine" backend for proposing (target); measuring never runs there. Only AWS VMs exist today (target: Kubernetes first); [today.md](today.md) says what is built.

| If you are doing | Read |
| --- | --- |
| Deciding what this area promises | [owes.md](owes.md) |
| Looking up a word (launcher, runner, sandbox, backend, workspace, evaluator, environment, spend record) | [model.md](model.md) |
| Checking a change against the rules that never bend | [rules.md](rules.md) |
| Finding what exists in the repo today, and what is only target | [today.md](today.md) |
| Supporting a new sandbox backend (a cloud, or Kubernetes) | [add-cloud.md](add-cloud.md) |
| Supporting a new agent or model provider (the provider is a field on the agent, not a connector kind) | [add-agent.md](add-agent.md) |
| Adding a machine type or image | [add-environment.md](add-environment.md) |
| What a benchmark asks of an environment, and who sets spend caps | [../optimization/SKILL.md](../optimization/SKILL.md) |
| What a scoped credential may reach | [../context/SKILL.md](../context/SKILL.md) |
| Who may create accounts and environments | [../organization/SKILL.md](../organization/SKILL.md) |
| Choosing tools and layers for the change | [../stack/SKILL.md](../stack/SKILL.md) |
| Deciding whether a change belongs in the product | [../product/SKILL.md](../product/SKILL.md) |

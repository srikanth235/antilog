# Rulings

## Settled boundaries

- **Context owns the code host.** The code host (GitHub, GitLab) is a context source. A pull request is a write-back action that an optimization requests; context performs it, as an explicit, audited action.
- **Agents are infrastructure workers.** An agent is something infrastructure runs. An optimization picks which agents and how many. The model provider is a field on the agent.
- **Infrastructure records spend; the optimization sets the cap.** The limit belongs to the goal; the meter belongs to the place work runs.
- **A benchmark belongs to the optimization.** It points at data (context) and machines (infrastructure). It is not a context or infrastructure object.
- **The native Mac app was removed.** The web app is the client (target). Whether a native client returns is the owner's call.
- **Sign-in is OpenID Connect only.** The hosted service offers GitHub and Google. A self-hosted server uses the administrator's provider.

## The web client (target)

The owner approved a web-client direction, shown as a clickable prototype in `design/web/`.

- **The screen is organised around experiments as sessions and one Customize page.** This replaces the four navigation groups (Experiments, Data, Compute, Admin). The three basics still own everything underneath. Area colour survives in the stepper and in the lineage columns (Experiment, Data, Compute). Behaviour is in [design behaviour](../design/behaviour.md).
- **An experiment is a session.** It opens as one thread: header, chart, conversation, trials, result. "Draft experiment" creates it at once as a Draft, and nothing is created in the customer's cloud until Approve.
- **Skills are a product concept, owned by Context.** A skill pack is versioned written know-how that agents read. The evaluator never reads it, it never holds holdout data, and the experiment records the versions read. It is not the agent guidance in this repo's `.claude/skills/`; in code and docs say "skill pack" where that could be confused.
- **A plugin has no powers of its own.** It is a bundle that adds evaluations, skills, suggested connectors and recommended agents under their own kinds. Each item is governed by its owning basic: an evaluation by optimization, a skill and a connector by context, an agent by infrastructure. Removing a plugin asks whether to keep its items, and items in use by an experiment are never removed. A plugin can never mark, unmark or grant access to a dataset.
- **Global search finds the exact place.** One palette, scopes All, Experiments, Projects, Datasets, Customize, Runs and People. A result opens the exact place, for example a trial inside its thread.
- **The model provider is a field on an agent.** It is not a connector category. Agents belong to infrastructure.
- **Evaluation templates are added under Customize** and picked when an experiment is started.

## Open, ask the owner

Write these as options with a recommendation when they come up. Do not pick one by building it.

- **Who brings the benchmark?** Does Ratchet draft the benchmark, or do engineers bring it?
- **Offline or production results.** Is scoring offline only, or are production results used too?
- **Reshaping data.** Does Ratchet ever reshape data? Today the answer is no (see [non-goals](non-goals.md)).
- **Where chat and drafting run.** Direct model calls from the server is the current recommendation. Not decided.
- **"Try a demo".** Does it stay in the web app? It runs on the simulation, which is a developer and demo tool.
- **The licence.** The workspace manifest says `UNLICENSED` today. What it becomes is not decided.

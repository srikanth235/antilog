# Rulings

## Settled boundaries

- **Context owns the code host.** The code host (GitHub, GitLab) is a context connector. A pull request is a write-back action that an optimization requests; context performs it, as an explicit, audited action.
- **Agents are infrastructure workers.** An agent is something infrastructure runs. An optimization picks which agents and how many. The model provider is a field on the agent.
- **Infrastructure records spend; the optimization sets the cap.** The limit belongs to the goal; the meter belongs to the place work runs.
- **A benchmark belongs to the optimization.** It points at data (context) and machines (infrastructure). It is not a context or infrastructure object.
- **The earlier native Mac app was removed.** The web app is the client (target). A new native macOS app is ruled in the next section (target).
- **Sign-in is OpenID Connect only.** Ratchet cloud offers GitHub and Google. A self-hosted server uses the administrator's provider.

## Architecture (target)

Decided by the owner. Everything here is target; nothing is built on this branch.

- **Two ways to run.** Self-hosted: the Ratchet server is a Docker Compose service (server and Postgres) in the customer's infrastructure. Ratchet cloud: we host the server, as Databricks hosts its control plane. Both are supported, and the same code serves both.
- **One launcher starts and removes sandboxes.** Self-hosted, it is built into the server. On Ratchet cloud, the same code is installed as a small runner in the customer's infrastructure. The runner connects out to the hosted server over HTTPS: it pulls jobs and the server streams updates. It opens no inbound port. Ratchet never holds access to the customer's cloud: no cross-account role, and no serverless compute in Ratchet's account for now.
- **The server never connects to data.** It orchestrates and records. Ratchet stores no data, only ids, versions, numbers, states and audit rows.
- **The coding agent's code runs the experiment.** The agent writes code, that code runs the experiment, and it reads data through connectors itself. There is no separate context plane that pulls or brokers data. This replaces the earlier idea that a context plane pulls and never copies.
- **Credentials are scoped, short-lived and minted per run.** The launcher mints them from connector admin secrets that stay in the customer's infrastructure (their secret manager). An agent gets read-only connector credentials that exclude the holdout. The evaluator gets a separate credential that includes the holdout. The holdout is enforced by those credentials: a read-only role, view or policy that leaves holdout data out. Model keys go through an egress proxy that adds them on the way out. Git access is a short-lived scoped token. This reverses the earlier ruling that an agent never holds a connector credential.
- **Sandboxes are cloud-agnostic.** Backends sit behind one trait. Kubernetes comes first, with separate node pools for workspace and evaluator and a NetworkPolicy for "no route". Then VMs on AWS, GCP and Azure. Measuring still runs on different machines from proposing, and a benchmark stays on one provider and one machine type. Provider words stay inside each adapter. On-prem Kubernetes counts as the customer's infrastructure.
- **Connectors** an agent's code may use: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, S3, GitHub, GitLab and any MCP server. Each knows how to mint a scoped, read-only, holdout-excluding credential for its system.

## The macOS app and the "This machine" backend (target)

Decided by the owner, 14 Nov.

- **A native macOS app, SwiftUI, macOS only at launch.** Windows and Linux use the web app. It is a client of the server's API and a local runner.
- **The Mac can be the "This machine" sandbox backend.** Agents propose there, with the same short-lived, read-only, scoped credentials as on any backend, governed, budgeted and cleaned up the same way.
- **Measuring never runs on the laptop.** The evaluator stays in the customer's infrastructure, so a Mac can never hold the holdout credential or produce a number that counts.
- **Clients share behaviour, words and element ids, never pixels.** The web app and the Mac app do not diverge on state or words; see [design behaviour](../design/behaviour.md).
- **Open:** how the Mac app is distributed, direct with Sparkle or the Mac App Store; see [stack/libraries.md](../stack/libraries.md).

## The web client (target)

The owner approved a web-client direction, shown as boards in `design/mockups/`.

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

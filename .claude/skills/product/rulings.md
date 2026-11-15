# Rulings

## Settled boundaries

- **Context owns the code host.** The code host (GitHub, GitLab) is a context connector. A pull request is a write-back action that an experiment requests; context performs it, as an explicit, audited action.
- **Agents are infrastructure workers.** An agent is something infrastructure runs. An experiment picks which agents and how many. The model provider is a field on the agent.
- **Infrastructure records spend; the experiment sets the cap.** The limit belongs to the goal; the meter belongs to the place work runs.
- **A benchmark belongs to the experiment.** It points at data (context) and machines (infrastructure). It is not a context or infrastructure object.
- **The earlier native Mac app was removed.** The web app is the client (target). A new native macOS app is ruled in the next section (target).
- **Sign-in is OpenID Connect only.** Ratchet cloud offers GitHub and Google. A self-hosted server uses the administrator's provider.

## Architecture (target)

Decided by the owner. Everything here is target; nothing is built on this branch.

- **Two ways to run.** Self-hosted: the Ratchet server is a Docker Compose service (server and Postgres) in the customer's infrastructure. Ratchet cloud: we host the server, as Databricks hosts its control plane. Both are supported from day 0, and one core serves both (see Ratchet cloud below).
- **One launcher starts and removes sandboxes.** Self-hosted, it is built into the server. On Ratchet cloud, the same code is installed as a small runner in the customer's infrastructure, or, if an owner chooses Ratchet-managed, runs as a managed runner we operate. The runner connects out to the hosted server over HTTPS: it pulls jobs and the server streams updates. It opens no inbound port. Ratchet never holds access to the customer's cloud: no cross-account role. Compute in Ratchet's account exists only for Ratchet-managed sandboxes, by the owner's choice.
- **The server never connects to data.** It orchestrates and records. Ratchet stores no data, only ids, versions, numbers, states and audit rows.
- **The coding agent's code runs the experiment.** The agent writes code, that code runs the experiment, and it reads data through connectors itself. Only the evaluator's measurement counts (see the agent loop). There is no separate context plane that pulls or brokers data. This replaces the earlier idea that a context plane pulls and never copies.
- **Credentials are scoped, short-lived and minted per run.** The launcher mints them from connector admin secrets that stay in the customer's secret manager, or, for Ratchet-managed, in our encrypted secret store held by the managed runner and never by the server. An agent gets read-only connector credentials that exclude the holdout. The evaluator gets a separate credential that includes the holdout. The holdout is enforced by those credentials: a read-only role, view or policy that leaves holdout data out. Model keys go through an egress proxy that adds them on the way out, and a sandbox never holds one. Git access is a short-lived scoped token. This reverses the earlier ruling that an agent never holds a connector credential.
- **Sandboxes are cloud-agnostic.** Backends sit behind one trait. Kubernetes comes first, with separate node pools for workspace and evaluator and a NetworkPolicy for "no route". Then VMs on AWS, GCP and Azure. Ratchet-managed is one more backend, on Cloudflare Containers, behind the same trait. Measuring still runs on different machines from proposing, and a benchmark stays on one provider and one machine type. Provider words stay inside each adapter. On-prem Kubernetes counts as the customer's infrastructure.
- **Connectors** an agent's code may use: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, S3, GitHub, GitLab and any MCP server. Each knows how to mint a scoped, read-only, holdout-excluding credential for its system.

## Ratchet cloud (target)

Decided by the owner, 15 Nov.

- **Storage.** Self-hosted uses Postgres 18. Ratchet cloud uses one SQLite database per organization, inside one Cloudflare Durable Object per organization, pinnable to the EU, the US or FedRAMP. Both ship from day 0. Moving between them is an export and import of one neutral file format, not a database copy.
- **One core, two shells.** `ratchet-core` holds all logic and the API routes and compiles natively and to WebAssembly. Storage sits behind one trait with two implementations. `ratchet-server` is the self-hosted shell; `ratchet-cloud` is the Cloudflare shell. A small shared directory holds sign-in routing and billing, never tenant data. See [stack](../stack/SKILL.md).
- **Where work runs.** A per-organization setting, owner-only, with two values. **Your infrastructure**: sandboxes run in the customer's backends through their runner, data never leaves, connector admin secrets stay in their secret manager. It is the default for enterprises and the only value self-hosted offers. **Ratchet-managed**: sandboxes run on Cloudflare Containers through a runner we operate, and only on Ratchet cloud. It is the default for small teams. An enterprise on Ratchet cloud may switch to it, with a warning that data will be read in Ratchet's sandboxes.
- **Ratchet-managed stays within the promises.** The server still never connects to data. Only agent code in a sandbox reads it, and the sandbox is removed at the end of the run, with nothing kept. Workspace and evaluator are separate sandboxes with no route between them, the holdout credential goes only to the evaluator, and credentials are scoped, read-only and short-lived. Connector admin secrets sit in our encrypted secret store, held by the managed runner.
- **Ratchet-managed limits.** The largest sandbox is 4 vCPU, 12 GiB and 20 GB disk, with no GPUs. Shared hardware makes timing benchmarks noisier, so the noise band is wider and fewer trials are kept. The accept rule handles it, and the Infrastructure page says so in one line.
- **We are a data processor** for Ratchet-managed organizations: a data processing agreement, listed subprocessors (Cloudflare, the model providers) and a stated retention rule (nothing kept after a run).
- **Model keys.** Each agent's model provider has a key source. **Ratchet-supplied** works only with Ratchet-managed, is its default, and bills tokens at cost through the run's budget and spend records. **Your own key** is always available and is the only choice for Your infrastructure and self-hosted. It lives in the customer's secret manager, or in our encrypted secret store for Ratchet-managed. Either way only the egress proxy adds the key.

## The agent loop (target)

Decided by the owner, 15 Nov. It replaces the one-pass run, where each agent proposed one change and the evaluator measured it. It follows autoresearch (change, measure, keep or discard, repeat), for any measurable goal and made trustworthy.

- **Agents drive the loop. Ratchet decides what counts.** Each agent in an experiment runs its own loop in its own sandbox until it stops: write a change, check it cheaply there (numbers made in a sandbox never count), ask for a measurement, read the result, build on it if Kept, and go again after reading the history. The agent decides what to try.
- **A trial is one measurement.** One change by one agent, measured by the evaluator on separate machines. A run has many trials per agent. Each measurement runs the evaluation's repeats and reports the noise band. The evaluation may bring up what it needs inside the evaluator sandbox and removes it after.
- **What an agent sees.** Each trial's measured value and guardrail values on the evaluation data, so it can iterate. It never reads the holdout and never reads or changes the evaluation's code or the evaluator. This replaces "never let a proposer see the scoring".
- **Kept and confirmed.** A trial is Kept when it beats that agent's own best by more than the acceptance criteria require, the guardrails hold, and it is confirmed: measured again in a fresh evaluator sandbox and checked on the holdout. The agent learns the outcome ("Kept" or "Failed holdout"), never the holdout value. The accept rule stays one pure function in code.
- **Own lineage, may start from the best.** Each agent has its own lineage and best so far. It sees the experiment's best trial and its diff in the history and may start from it; the lineage records that. The run's result is the best kept trial across all agents.
- **The budget starts even and passes on what a stopped agent leaves.** Each agent starts with an even share. An agent's spend is its model tokens, its sandbox and the evaluator time for its trials, and it sees its remaining share. When an agent stops early (its plateau), its unspent share is split evenly among the agents still iterating. Budget is never taken from an agent that is still iterating, and a paused agent keeps its share. Every move is recorded and shown on the lanes. Ratchet does not weight shares by early gains, because early gains are often noise and weighting would make agents harder to compare.
- **Stops.** An agent stops when its share is spent or after 8 trials with none kept; its sandboxes are removed at once. The run stops at its budget, its deadline, or after 12 trials with none kept by any agent. A person can Pause agent or Stop run.
- **People steer with notes.** A note goes to all agents, or to one with @ (for example "@Codex focus on database queries"). Each agent reads it before its next trial.

## Desktop apps and the "This machine" backend (target)

Decided by the owner, 14 Nov.

- **A native macOS app, SwiftUI, macOS only at launch.** It is a client of the server's API and a local runner.
- **On Windows and Linux the desktop app is the web app, installed as a progressive web app** (decided 15 Nov). The same web client, installable from Chrome or Edge: its own window and taskbar icon, a badge for what needs attention, push notifications and an offline screen. On macOS the web app does not offer to install; it points to the Mac app. This machine stays Mac only.
- **Notifications are in the product** (decided 15 Nov, replacing "Notifications cut"): for a win, something that needs a person, a budget hit or a runner gone, and for a run's end when asked. Native on the Mac, web push elsewhere, ids only. See [organization/notifications.md](../organization/notifications.md).
- **The device stores no data either.** The service worker caches the app's own files only, never API responses, diffs or logs. A push message carries ids only; the app fetches the rest when opened. A self-hosted server that cannot reach the browsers' push services sends no push, and everything else works.
- **The Mac can be the "This machine" sandbox backend.** Agents propose there, with the same short-lived, read-only, scoped credentials as on any backend, governed, budgeted and cleaned up the same way.
- **Measuring never runs on the laptop.** The evaluator stays in the customer's infrastructure, so a Mac can never hold the holdout credential or produce a number that counts.
- **Clients share behaviour, words and element ids, never pixels.** The web app and the Mac app do not diverge on state or words; see [design behaviour](../design/behaviour.md).
- **The Mac app is a direct download, updated with Sparkle** (decided 15 Nov). It is signed with a Developer ID, notarized and not on the Mac App Store, so the App Store sandbox never limits the local runner. It checks one signed update feed, ours, daily by default; a person can turn checks off, and a company can turn them off for its Macs and deploy releases itself through device management, which is also how air-gapped installs update. See [stack/swift/ratchet.md](../stack/swift/ratchet.md).

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

# Rulings

The settled product, directed by the owner. Everything here is target; nothing is built on this branch. Cut features are in [non-goals](non-goals.md), the words in [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md).

## A chat and a judge

- **Ratchet is a chat and a judge.** The chat works like Claude Code on the web: a person talks to a coding agent in a sandbox with repos attached. The judge is a set of tools every session gets. Only the judge's numbers count, and only a measured win becomes a pull request.
- **No modes, no plan step, no orchestration loop on the server.** The agent drives. The only server-side state machines are the judge's queue and the budget meter.
- **Four things.** A **project** groups the experiments for one area of the code. An **experiment** is the sealed goal and everything measured against it. A **session** is one agent's work inside an experiment. A **measurement** is the judge's signed result for one trial. Everything else (scoreboard, wins, receipts, value, Return) is a view over measurements. Reusable setup lives in one Customize page.
- **Experiment.** Its definition of better (metric and direction, evaluation and protected files, guardrails, data and holdout, repeats, acceptance criteria, quick check, patience, value model) is immutable once sealed; a change makes a new version and numbers never carry across. State: Not sealed, Sealed, Watching or Regressed.
- **Session.** A chat with one agent in one sandbox on one agent platform. It always belongs to an experiment. Status: Working, Waiting for you, Done or Failed; a stopped one is Done, "Stopped by NAME". On screen it is named by its agent.
- **Trial.** One `measure` call: a set of commits (one per repo changed), the trial it started from and a one-line hypothesis.
- **Tools.** Every session gets the Ratchet tools, and `ratchet mcp` (target) serves them to an agent on a person's own machine. Work tools: `seal`, `measure` (asynchronous: a ticket now, the measurement later as an event), `history`, `spawn`, `deliver`, `watch`. Setup tools, each a card: `attach_repo`, `request_credential`, `set_environment`, `request_budget`. See [session tools](../session/tools.md) and [judge tools](../judge/tools.md).
- **Memory is `history`.** Each trial carries its hypothesis, so a later session reads what was tried and what failed. There is no notes feature. Skill packs are written know-how a person adds, not a record of trials.

## Settings are tools

- A person approves the tool calls that matter, and settings pages list and revoke what was approved. Setup happens in the chat, at the moment it is needed, as a card.
- Eight card kinds: Seal experiment, Raise budget, Add credential, Attach repo, Change environment, Open pull request, Keep going?, Continue on another agent?. Defaults, policy, Mode and who may approve are in [organization approvals](../organization/approvals.md).
- Add credential always asks and can never be pre-approved: a person types the value into a secure form that goes straight to the platform's credential store.
- Roles decide who may approve which card; a viewer approves nothing. The audit log lists tool calls that changed something and approvals.
- Two things are set up in Settings because no experiment can run without them: agent platform accounts (self-hosted, later) and members and sign-in. Connectors and agents are registered once in Customize, and a chat can ask for the same thing with an Add credential card, so nothing waits on it.

## The judge

- **A trait separate from the agent platform trait:** run this command at these commits with these secrets and return these numbers. See [judge rules](../judge/rules.md).
- **The trust rules never bend.** Only the judge's numbers count. An agent never holds the holdout credential. A sealed experiment never changes. A person or an agent can never mark a win: only the acceptance criteria can, as one pure function in code. Chat changes what an agent tries, never what counts.
- **Anti-cheat is built in:** a seal over the definition and protected files (a trial that changes a protected path is Rejected), a fresh judge sandbox for every measurement, seal checks, confirmation before Kept, a second opinion from a different model provider that flags and never decides, and a signed receipt. See [judge anti-cheat](../judge/anti-cheat.md).
- **Acceptance.** A trial is Kept when it beats the trial it started from by more than the bar, every guardrail holds, and it is confirmed in a fresh judge sandbox and on the holdout. An experiment's best is its best Kept trial. Outcomes: Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected, Failed.
- **The holdout as an oracle.** Failed holdout is one bit per trial, so it can be probed. The judge credential is read-only. Holdout checks are capped per experiment version (sealed, default 20); at the cap the experiment needs a new version with a new holdout. The trial's code sees the holdout at the same path and shape as the evaluation data, and a standard planted cheat that behaves differently on the holdout must be caught by the seal checks.
- **What an agent sees.** The measured value and guardrail values on the evaluation data, and the outcome. It may read the evaluation's code and may never change it. It never reads the holdout or learns a holdout value.
- **Paired measurement and the bar.** In each fresh judge sandbox the trial and the trial it started from run interleaved, for the sealed number of repeats, and the gain is the mean of the paired differences. The bar: the one-sided lower confidence bound of that gain must be above zero at confidence 1 − 0.05/k, where k is the number of trials measured in this experiment version, this one included, so many attempts cannot win by luck. Confirmation repeats the paired measurement in another fresh sandbox.
- **Quick check and patience.** A measurement starts with a quick check (one pair: the trial and the experiment's best, run once each in the same sandbox); the full measurement follows only if it beats the best by more than the noise. The quick check never decides Kept. When patience (default 6) trials in a row are not Kept, the judge first measures each session's branch head that differs from every measured trial as one cumulative trial, started from the best, so gains each below the bar can add up; it counts in k and meets the same bar. If none is Kept, the experiment waits on a Keep going? card. Both are sealed.
- **Delivery and watch.** `deliver` opens one linked pull request per repo changed, each carrying the receipt, and refuses any trial that is not Kept. The session stays attached for review comments and CI failures, and a rebase is measured again. A delivered experiment can be watched: the judge pairs the delivered commit with the current head of the default branch, and a regression is a paired loss whose lower confidence bound is above zero at 95%. It wakes or starts a session and notifies.
- **Receipt.** Every win carries a signed receipt with Experiment, Data and Compute groups, and a Verify action. See [judge model](../judge/model.md).
- **Value.** A win is Delivered once its pull request is merged. Pending is not in Return. Delivered is measured offline by the judge and says so wherever it is shown ("measured offline") until production scoring is decided. See the three aims below.

## Sessions, budget and storage

- **One agent per session,** with any number of repos. Git is the hand-off: each session gets a short-lived git token per repo that pushes only to its own branch, and Ratchet stores commit ids, never diffs.
- **Spawn.** A session may spawn others in the same experiment, on the same budget, on any agent and platform. A spawned session cannot spawn, appears as a card and opens as its own chat. See [session rules](../session/rules.md).
- **Budget.** One per experiment, covering all its sessions and judge time. It is set on the composer (default: the project's, then the organization's) and an editor or owner can change it at any time; lowering it below spend stops new measurements. When it is spent the judge refuses `measure` and the experiment is Waiting for you with a Raise budget card. It sits inside its project's optional monthly cap, inside the organization's.
- **The sandbox is temporary, the conversation is not.** A sandbox ends when the session is idle or stopped, and resuming starts a new one from the session's branches.
- **What the server stores:** the conversation, tool call names and card states, commit ids, measurements, spend and audit rows. Never tool output, file contents, diffs or data. Tool output and raw logs stay on the platform, are fetched when opened and may expire; the session says so.
- **Prompt injection is assumed.** Every agent reads untrusted text; what a hijacked agent holds and what still gets through is in [session threat model](../session/threat-model.md).
- **A person's terminal** in the agent's sandbox is audited, and its output is not stored.
- **What agents are told is part of the product** and is versioned and tested. See [session prompts](../session/prompts.md).
- **Rules kept:** one translator per outside system; Ratchet never reshapes or pipes data; a session contacts only the endpoints its environment allows; a stopped session is ended and deleted at once.

## Projects, Customize and data

- **Projects.** Every experiment belongs to exactly one project. A project has members as owner, editor or viewer; the organization role is the default in every project and a project can set another for that project. Organization owners manage organization settings; project owners manage the project's members, budget, cap and value models. Permission is checked at project scope. See [organization model](../organization/model.md).
- **Customize** is one sidebar page, a library of reusable things, with tabs Evaluations, Skills, Connectors, Agents and Plugins, each with Yours and Discover. Every item opens the one DetailPage layout. Setting one up with a card works too, and what a card adds appears here.
  - **Evaluations** are reusable templates (kind Speed, Quality, Size or Cost). A sealed experiment starts from a copy, and the seal checks stay the judge's.
  - **Skills** are versioned skill packs. The judge never reads one, a pack never holds holdout data, and an experiment records the versions read. See [session add-skill-pack](../session/add-skill-pack.md).
  - **Connectors** are Snowflake, Databricks, BigQuery, Postgres, DynamoDB, Amazon S3 and an MCP server. The code host is not a connector.
  - **Agents** are listed with platform, model and use. Settings, Agent platforms keeps platform accounts only.
  - **Plugins** bundle the other kinds. A plugin has no powers of its own and can never mark, unmark or grant a dataset. An item in use is never removed.
- **Two credentials per connector,** registered once and kept in the platform's credential store, never by Ratchet. The agent credential is read-only and leaves out the holdout; the judge credential is separate and also covers it. Both are short-lived and minted per session.
- **Datasets and the holdout.** Datasets live under a connector, and a dataset version never changes. A holdout is a marking enforced by the credentials; the grant control is absent, not disabled, and the row reads "Restricted to the judge". A holdout value never appears in history, a message or a receipt shown to an agent. A connector error shows datasets as Not checked, never empty.
- **Small things.** @-mention of a session in the composer, and a one-time prompt after the first measurement, "Notify me when this experiment needs me".

## The three aims

Beside trust: cost control, no tie to one model provider, and Return for the organization. They sit at the top of the [product skill](SKILL.md).

- **Value models.** What one unit of a metric is worth in dollars a year, with a source and who set it. A project owner sets one per metric; a different one proposed on the Seal experiment card needs a project owner's approval. It is sealed with the experiment. With none, the experiment shows "No value set" and counts in no dollar figure. Agents never set one.
- **Return** is Delivered divided by everything spent in the scope (project, organization, time range), including experiments that found nothing, were stopped or failed. A regressed win is At risk, not Delivered.
- **Auto picks the agent** with the best cost per win for this evaluation kind in this project, among the agents the organization turned on, and says why. Before there is data it uses the organization's default agent, set by an owner in Customize, Agents. A person can always pick a named agent, and no agent is preselected by brand.
- **Cost views.** Cost per trial, an estimate on the Seal experiment card, Where the spend went, the agent scoreboard (measured on the customer's code, never a vendor benchmark), hand off, failover, compaction, Ideas and a monthly Report. See [session model](../session/model.md), [judge model](../judge/model.md) and [organization approvals](../organization/approvals.md).

## Business model

- **Services first, on our own platform.** Ratchet is sold as engagements: our engineers work inside a client's organization with Ratchet and deliver measured wins. Client engineers get their own roles as the engagement goes on and keep using Ratchet after it ends. The platform's job is to make each engagement cheaper than the last: time to first win, engineer-days per engagement and evaluations reused are tracked.
- **Pricing.** A fixed fee per engagement, a capped success fee on verified wins, and a platform subscription once client engineers run it themselves. The success fee rests on the value model, which the client signs off on the Seal experiment card, so a dispute is settled before work starts.
- **Named offers, sold broadly.** Engagements are sold as modernisation with proof, delivered through a few named offers with ready evaluations and value models (cost and latency, build and test time first). Evaluation kinds and connectors are built as paying engagements need them.
- **Two phases.** Phase 1 is Ratchet cloud: we run it on our AWS account for early clients and pilots. Phase 2 is self-hosted in the client's own cloud account (AWS, Azure or GCP), which is what the Fortune 100 buy. Both run the same container images and Helm chart, so every phase 1 engagement tests what phase 2 ships. At least one Fortune 100 pilot, on a low-risk repo, runs in phase 1.
- **Our staff across organizations.** Our engineers work in several client organizations at once: one sign-in reaches each organization they are members of, and every action is in that organization's audit log.
- **Compute is rebilled.** Platform spend (tokens, sandboxes, judge time) is metered by the budget and appears on the client's invoice. Self-hosted, it is on the client's own cloud bill.

## The screen and the clients

- **Frame and pages.** A fixed 256px sidebar and a main area; a New experiment composer with five chips (Project, Repos, Agent, Environment, Budget), not a dialog; an experiment page that is the chat of its lead session; Projects, a project page, Customize, an experiment detail page, a receipt page and Settings as a dialog. A new experiment starts Not sealed: the agent explores, measures the baseline and proposes a definition with `seal`, and it may stay Not sealed and end there. The composer follows Claude Code's, with a Mode per experiment (Ask or Follow policy; "Auto" names only the agent choice). A card is the only place structured output appears. See [design behaviour](../design/behaviour.md).
- **The chat's voice is Ratchet.** An agent's message shows the Ratchet mark and the name "Ratchet", with the agent that did the work as a quiet second label. Agent logos stay where an agent is the subject.
- **Clients.** The web app, the `ratchet` CLI (`ratchet "cut checkout p95"` opens the same session in the terminal) and `ratchet mcp` (all target), and the native macOS app, which stays ruled in with its boards removed until the web direction is approved. Clients share behaviour, words and element ids, never pixels.
- **Windows and Linux** use the web app installed as a progressive web app. On macOS it points to the Mac app. Notifications (a win, Waiting for you, a regression, a budget spent) carry ids only ([organization notifications](../organization/notifications.md)). The device stores no data: the service worker caches the app's own files only.
- **The Mac app** is a direct download, signed, notarized and updated with Sparkle from one signed feed; checks can be turned off and releases deployed through device management. See [stack/swift/ratchet.md](../stack/swift/ratchet.md).

## Offerings, platforms and vendor facts

- **Two offerings, one core.** Ratchet cloud (phase 1) and self-hosted (phase 2) run the same server, images and Helm chart on Kubernetes with Postgres. Ratchet cloud: we run it on our AWS account and every session runs on Ratchet's platform accounts. Self-hosted: the client runs it in its own AWS, Azure or GCP account, on its own platform accounts. Nothing is specific to one cloud except a thin infrastructure module per cloud; see Deployment below.
- **Ratchet runs no sandboxes for agents. Agent platforms do:** Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service and Google Agent Runtime, each behind one agent trait with one adapter (start a session, send input, read events, stop and delete it). Foundry and Google run a container image Ratchet supplies, one per agent.
- **An agent is a model on a platform:** Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AgentCore, GitHub Copilot on Foundry, Gemini (ADK) on Google Agent Runtime.
- **Build order.** The traits are designed against all five from the start. Adapters ship: AgentCore (agents and the judge sandbox), then a Kubernetes sandbox adapter, then Foundry Agent Service and Google Agent Runtime as clients on Azure and GCP need them, then Claude Managed Agents and the OpenAI Agents API.
- **A Kubernetes sandbox adapter.** It runs the judge sandbox, and optionally agents, as isolated pods (gVisor or Kata) on a dedicated node pool in the cluster Ratchet runs in. It gives one judge that behaves the same on every cloud, keeps the client's code in its own cluster, and gives quieter timing than shared sandboxes.
- **Where the judge sandbox runs:** on a platform that can run a command in a sandbox with no agent. AgentCore can. Google Agent Runtime is checked in the spike. Claude Managed Agents, the OpenAI Agents API and Foundry cannot; on Azure and GCP the judge runs in the Kubernetes sandbox adapter.
- **`measure` is asynchronous** so no platform limit applies (Foundry may expire tool outputs after 10 minutes; Google's event stream is undocumented and limited to 10 minutes per query). The spike checks it.
- **Credentials reach a session only through the platform's credential store,** never a prompt, a repo file or Ratchet's database. No model key enters a sandbox.
- **Ratchet cloud uses only Ratchet's platform accounts.** The adapters support the client's accounts because self-hosted needs them. A client that needs its code and data to stay in its own network self-hosts.
- **Timing is noisier on shared sandboxes,** so the noise band is wider and fewer trials are Kept; paired measurement cancels drift between sandboxes, and the experiment detail page says so.
- **A Helm chart and OpenTofu.** The server ships as signed container images and one Helm chart. One OpenTofu module per cloud creates what sits around the cluster. Docker Compose is kept for local development and quick evaluations only. Ratchet's own accounts are owned by the owner.
- **"This machine" is deferred.** Ratchet never starts an agent on a Mac until a later ruling. `ratchet mcp` is separate: it serves the tools to an agent a person runs themselves, and that agent's trials are still measured only in a judge sandbox. Measuring never runs on a laptop.

## Licence

Ratchet is licensed under the Functional Source License, FSL-1.1-ALv2: the source is public and anyone may run, change and self-host it, but not offer it as a competing product or service, and each release becomes Apache-2.0 two years after it is published. The licensor is Duaility, LLC; the text is in [`LICENSE.md`](../../../LICENSE.md). Every dependency's licence must allow this ([stack/libraries.md](../stack/libraries.md)).

## Deployment

- **The server is cloud-agnostic.** One Rust server (`ratchet-server`) with the web app built in, and Postgres 18 as the only stateful dependency, which also holds the job queue and live events. Files and exports go to object storage through one crate that speaks S3, Azure Blob and GCS. Secrets are read from mounted files or the environment, never through a cloud SDK. Cloud access is by workload identity, with no stored cloud keys. Telemetry is OpenTelemetry, exported to whatever the client uses. Images are signed, with an SBOM, and can be pushed to a client's own registry for an air-gapped install.
- **Kubernetes everywhere.** EKS, AKS, GKE or OpenShift run the same Helm chart. Upgrades are a Helm upgrade with a job that runs forward-only migrations first.
- **One OpenTofu module per cloud** creates managed Postgres (RDS, Azure Database for PostgreSQL, Cloud SQL), a bucket, a key in the cloud's key service and workload identity. A client with its own cluster uses the chart alone. AWS ships first; Azure and GCP are built when the first client on each signs.
- **Ratchet cloud grows in steps, the chart unchanged.** Demos: one small EC2 instance running k3s, with Postgres in the cluster and a nightly backup to S3, stopped between demos (about $5 a month idle). First paying client: EKS with RDS, one namespace and one database per client (about $200 a month). Clients relying on it: Multi-AZ RDS and more nodes (about $400 to $500 a month). A client whose contract requires it gets its own database instance or cluster.
- **Ratchet cloud in phase 1** runs in the US. Organizations join by invitation. Sign-in is GitHub and Google; repos come from GitHub only, through a GitHub App. The first cut runs only models AgentCore reaches through Amazon Bedrock, Claude first, so no model key enters a sandbox; Claude is also on Microsoft Foundry and Vertex AI for phase 2 on Azure and GCP.
- **Ratchet's platform credentials** sit in AWS Secrets Manager, read by the server through workload identity, never in an organization's database, a response or a log.
- **We are a data processor** for every Ratchet cloud organization: a data processing agreement, listed subprocessors (AWS, and Anthropic through Bedrock) and a stated retention rule. Ratchet keeps no tool output, file contents or diffs. Each platform's retention and residency is stated beside it: the vendors promise only where data is stored at rest, and the OpenAI Agents API offers United States residency only and no zero data retention. Self-hosted, the data stays in the client's account and a services agreement sets our engineers' access.
- **Security paperwork starts in phase 1.** SOC 2 Type I is started during phase 1, since the Fortune 100 ask for it on the first call.

## Sign-in

OpenID Connect plus GitHub's OAuth, no passwords. Ratchet cloud offers GitHub (OAuth with its user API, since GitHub offers no OpenID Connect for app sign-in), Google (OpenID Connect) and an organization's own OpenID Connect provider (single sign-on, set up only by an owner). A self-hosted server uses the client's provider (Entra ID, Okta, Google).

## Decided by the owner

| Date | Decision |
| --- | --- |
| 14 and 15 Nov 2026 | A native macOS app, SwiftUI, macOS only at launch. |
| 15 and 16 Nov 2026 | Ratchet cloud on Cloudflare, one Durable Object per organization. Replaced on 17 Nov by AWS and Kubernetes. |
| 16 Nov 2026 | The design is a chat and a judge. |
| 16 Nov 2026 | The server stores the conversation's words, never tool output. |
| 16 Nov 2026 | The customer's CI runner is not a judge sandbox; the trait leaves room for it. |
| 16 Nov 2026 | "Experiment" names the goal; a session is one agent's work inside it. |
| 16 Nov 2026 | Projects, Customize, skill packs and plugins are in; notes and the dashboard page are out. |
| 16 Nov 2026 | The macOS app's boards are removed until the web direction is approved. |
| 16 Nov 2026 | No per-repo visibility; roles can be set per project. |
| 16 Nov 2026 | One budget per experiment, changeable at any time, inside the monthly caps. |
| 16 Nov 2026 | The composer follows Claude Code's, with Mode (Ask or Follow policy). |
| 16 Nov 2026 | The chat speaks as Ratchet. An agent may read the evaluation's code, never change it. |
| 16 Nov 2026 | The three aims, with value models, Auto, Return on all spend, sealed quick check and patience, and two new card kinds. |
| 16 Nov 2026 | The spike measures noise on each judge platform. |
| 16 Nov 2026 | Paired measurement; the bar is a lower confidence bound above zero at 1 − 0.05/k. |
| 16 Nov 2026 | AgentCore is the first adapter, for agents and the judge. |
| 16 Nov 2026 | A thin first release: self-hosted, AgentCore, the web app, Speed, and `ratchet mcp`. Replaced the same day by cloud first. |
| 16 Nov 2026 | The licence is the Functional Source License (FSL-1.1-ALv2). |
| 16 Nov 2026 | Cumulative trials at patience, so small gains add up. |
| 16 Nov 2026 | Watch is paired: the delivered commit against the current head. |
| 16 Nov 2026 | Holdout as an oracle: read-only judge credential, a sealed cap on holdout checks, a holdout look-alike cheat. |
| 16 Nov 2026 | Prompt injection is assumed; the session threat model is written. |
| 16 Nov 2026 | Delivered is labelled "measured offline" until production scoring is decided. |
| 16 Nov 2026 | Cloud first: the first release is Ratchet cloud; self-hosted follows. |
| 16 Nov 2026 | Invite only with no billing at first; Ratchet pays under a hard cap per organization. Replaced on 17 Nov: compute is rebilled. |
| 16 Nov 2026 | US jurisdiction first, then the EU. |
| 16 Nov 2026 | Models on Bedrock only in the first cut, Claude first. |
| 16 Nov 2026 | GitHub is the only code host in the first cut. |
| 16 Nov 2026 | Three to five design partners at launch. Replaced on 17 Nov by paid engagements and pilots. |
| 17 Nov 2026 | Services first: our engineers deliver engagements on Ratchet, client engineers take over. |
| 17 Nov 2026 | Pricing: a fixed fee per engagement, a capped success fee on verified wins, then a subscription. |
| 17 Nov 2026 | Phase 1 is Ratchet cloud on our AWS; phase 2 is self-hosted in the client's AWS, Azure or GCP account. |
| 17 Nov 2026 | Cloudflare is dropped. Kubernetes, one Helm chart and one OpenTofu module per cloud. |
| 17 Nov 2026 | A Kubernetes sandbox adapter runs the judge on any cloud. |
| 17 Nov 2026 | Demos run on k3s on one EC2 instance; EKS from the first paying client. |
| 17 Nov 2026 | Value models, Return and the Report are in the first release. |

Decided calls also appear on the Decisions board.

## Still unproven: the spike

The spike comes before any product code. One tiny experiment is measured end to end on all five platforms. It proves the agent trait, the Ratchet tools served to a session over its event stream and a judge sandbox that holds a credential no agent sees. It also runs one fixed benchmark many times in fresh judge sandboxes on each judge platform and records the spread of paired differences, which says whether Speed experiments can be Kept there.

The spike also answers:

- **Repeats.** How many paired repeats make the bar reachable. With 3 repeats and k = 10 the critical value is about 10, so almost nothing is Kept. The Seal experiment card should show the smallest gain the setup can detect.
- **The Speed holdout.** What a holdout is for a timing evaluation: other inputs, another load profile, or something else.
- **A first example that fits.** "Cut checkout p95" needs real services and load that a sandbox with no connectors does not have. Find a Speed experiment that can be measured honestly there, such as test suite or build time, or a library benchmark.
- **The agent image.** AgentCore runs a container Ratchet supplies, so Ratchet packages the coding agent itself. What goes in it and who maintains it.
- **Usage timing.** Whether each platform reports usage fast enough for the budget meter to stop spend on time.
- **Quotas.** Platform and model limits on concurrent sessions, and what happens when one is hit.
- **One branch per session.** A GitHub App installation token can push to any branch of a repo, not just the session's. Show that pushes can be held to the session's branch (for example a ruleset on a `ratchet/*` prefix, or a push relay that checks the ref), or change the design.
- **The Kubernetes judge.** How noisy and how well isolated judge pods (gVisor or Kata, dedicated nodes) are, beside AgentCore.
- **Cost of one experiment.** Tokens, sandbox and judge time for one sealed experiment end to end, which sets engagement pricing.
- **Pair interference.** Whether warm caches, shared state or migrations leak between the two sides of a pair, and whether an ABBA order is enough.

## First release

Ratchet cloud on our AWS (k3s for demos, EKS from the first paying client), with AgentCore and models on Bedrock, GitHub sign-in and repos (and Google sign-in), the web app and `ratchet mcp`, one evaluation kind (Speed) with no connectors: seal, measure, Kept, receipt, deliver, budget, approvals and the audit log, plus value models, Return and the Report, since they are what an engagement is paid on. Before the first client: the push scan and second opinion from the threat model, a data processing agreement, the subprocessor list, a retention and deletion rule, the cross-organization staff sign-in and spend alerts. Then, in this order unless the owner says otherwise: the Kubernetes sandbox adapter, self-hosted on the client's AWS, evaluation kinds and connectors as engagements need them, Azure and GCP with their adapters, plugins, Ideas, and the Mac app. Their fields stay in the sealed definition and the store from the start, so nothing is redesigned.

A `ratchet mcp` connection works inside a session in the experiment, named by its agent, with no platform sandbox. Its trial commits are pushed to that session's branch with a short-lived git token minted for the session, and are measured only in a judge sandbox.

## Open questions

- Whether scoring is offline only or production results are used too.
- Whether Ratchet ever reshapes data (today no, see [non-goals](non-goals.md)).
- Whether "Try a demo" stays in the web app.
- Whether every trial in the version, from every session and including quick-check stops, should count in k. Spawned sessions raise the bar for all.
- How experiments on the same code interact: overlapping changes, a default branch that moves mid-experiment, one win making another's baseline stale. Today only delivery rebases.
- How a secret pasted into the chat is caught before the conversation is stored.
- Who holds the judge's signing key when self-hosted (on Ratchet cloud it is Ratchet's, in AWS Secrets Manager), how it rotates, and what "Signed by the judge" proves to someone outside the organization.
- The seal is the hardest step and seal checks catch only a trivially broken evaluation. How the product helps a team that has no benchmark.
- How time to merge is shown and eased, since value counts only after a merge.
- What the second opinion costs per win, and its data path: the diff goes to a second provider.
- The FSL's "competing use" wording and whether contributors sign an agreement.

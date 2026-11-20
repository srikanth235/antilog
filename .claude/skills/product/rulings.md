# Rulings

> `docs/V0.md` wins over this page: agent platforms, the Platform trait, the judge sandbox, the stepping relay and Kubernetes and Helm as a requirement are retired. Lines that named them are removed; the dated decision log below is history.

The settled product, directed by the owner. Everything here is target; nothing is built on this branch. Cut features are in [non-goals](non-goals.md), the words in [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md).

## A chat and a judge

- **Antilog is a chat and a judge.** The chat works like Claude Code on the web: a person talks to a coding agent in a sandbox with repos attached. The judge is a set of tools every session gets. Only the judge's numbers count, and only a measured win becomes a pull request.
- **No modes, no plan step, no orchestration loop on the server.** The agent drives. The only server-side state machines are the judge's queue and the budget meter.
- **Four things.** A **project** groups the experiments for one area of the code. An **experiment** is the sealed goal and everything measured against it. A **session** is one agent's work inside an experiment. A **measurement** is the judge's signed result for one trial. Everything else (scoreboard, wins, receipts, value, Return) is a view over measurements. Reusable setup lives in one Customize page.
- **Experiment.** Its definition of better (metric and direction, evaluation and protected files, guardrails, data and holdout, repeats, acceptance criteria, quick check, patience, value model) is immutable once sealed; a change makes a new version and numbers never carry across. State: Not sealed, Sealed, Watching or Regressed.
- **Session.** A chat with one agent in one machine. It always belongs to an experiment. Status: Working, Waiting for you, Done or Failed; a stopped one is Done, "Stopped by NAME". On screen it is named by its agent.
- **Trial.** One `measure` call: a set of commits (one per repo changed), the trial it started from and a one-line hypothesis.
- **Tools.** Every session gets the Antilog tools, and `antilog mcp` (target) serves them to an agent on a person's own machine. Work tools: `seal`, `measure` (asynchronous: a ticket now, the measurement later as an event), `history`, `deliver`, `watch`. Setup tools, each a card: `attach_repo`, `request_credential`, `set_environment`, `request_budget`. See [session tools](../session/tools.md) and [judge tools](../judge/tools.md).
- **Memory is `history`.** Each trial carries its hypothesis, so a later session reads what was tried and what failed. There is no notes feature. Skill packs are written know-how a person adds, not a record of trials.

## Settings are tools

- A person approves the tool calls that matter, and settings pages list and revoke what was approved. Setup happens in the chat, at the moment it is needed, as a card.
- Eight card kinds: Seal experiment, Raise budget, Add credential, Attach repo, Change environment, Open pull request, Keep going?, Continue on another agent?. Defaults, policy, Mode and who may approve are in [organization approvals](../organization/approvals.md).
- Add credential always asks and can never be pre-approved: a person types the value into a secure form that goes straight to the server's secret store.
- Roles decide who may approve which card; a viewer approves nothing. The audit log lists tool calls that changed something and approvals.
- Two things are set up in Settings because no experiment can run without them: model keys (saved per organization per provider) and members and sign-in. Connectors and agents are registered once in Customize, and a chat can ask for the same thing with an Add credential card, so nothing waits on it.

## The judge

- **The trust rules never bend.** Only the judge's numbers count. An agent never holds the holdout credential. A sealed experiment never changes. A person or an agent can never mark a win: only the acceptance criteria can, as one pure function in code. Chat changes what an agent tries, never what counts.
- **Anti-cheat is built in:** a seal over the definition and protected files (a trial that changes a protected path is Rejected), a fresh checkout for every measurement, seal checks, confirmation before Kept, a second opinion from a different model provider that flags and never decides, and a signed receipt. See [judge anti-cheat](../judge/anti-cheat.md).
- **Acceptance.** A trial is Kept when it beats the trial it started from by more than the bar, every guardrail holds, and it is confirmed in a fresh checkout and on the holdout. An experiment's best is its best Kept trial. Outcomes: Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected, Failed.
- **The holdout as an oracle.** Failed holdout is one bit per trial, so it can be probed. The judge credential is read-only. Holdout checks are capped per experiment version (sealed, default 20); at the cap the experiment needs a new version with a new holdout. The trial's code sees the holdout at the same path and shape as the evaluation data, and a standard planted cheat that behaves differently on the holdout must be caught by the seal checks.
- **What an agent sees.** The measured value and guardrail values on the evaluation data, and the outcome. It may read the evaluation's code and may never change it. It never reads the holdout or learns a holdout value.
- **Paired measurement and the bar.** In each fresh checkout the trial and the trial it started from run interleaved, for the sealed number of repeats, and the gain is the mean of the paired differences. The bar: the one-sided lower confidence bound of that gain must be above zero at confidence 1 − 0.05/k, where k is the number of trials measured in this experiment version, this one included, so many attempts cannot win by luck. Confirmation repeats the paired measurement in another fresh sandbox.
- **Quick check and patience.** A measurement starts with a quick check (one pair: the trial and the experiment's best, run once each in the same sandbox); the full measurement follows only if it beats the best by more than the noise. The quick check never decides Kept. When patience (default 6) trials in a row are not Kept, the judge first measures each session's branch head that differs from every measured trial as one cumulative trial, started from the best, so gains each below the bar can add up; it counts in k and meets the same bar. If none is Kept, the experiment waits on a Keep going? card. Both are sealed.
- **Delivery and watch.** `deliver` opens one linked pull request per repo changed, each carrying the receipt, and refuses any trial that is not Kept. The session stays attached for review comments and CI failures, and a rebase is measured again. A delivered experiment can be watched: the judge pairs the delivered commit with the current head of the default branch, and a regression is a paired loss whose lower confidence bound is above zero at 95%. It wakes or starts a session and notifies.
- **Receipt.** Every win carries a signed receipt with Experiment, Data and Compute groups, and a Verify action. See [judge model](../judge/model.md).
- **Value.** A win is Delivered once its pull request is merged. Pending is not in Return. Delivered is measured offline by the judge and says so wherever it is shown ("measured offline") until production scoring is decided. See the three aims below.

## Sessions, budget and storage

- **One agent per session,** with any number of repos. Git is the hand-off: each session gets a short-lived git token per repo that pushes only to its own branch, and Antilog stores commit ids, never diffs.
- **Budget.** One per experiment, covering all its sessions and judge time. It is set on the composer (default: the project's, then the organization's) and an editor or owner can change it at any time; lowering it below spend stops new measurements. When it is spent the judge refuses `measure` and the experiment is Waiting for you with a Raise budget card. It sits inside its project's optional monthly cap, inside the organization's.
- **The sandbox is temporary, the conversation is not.** A sandbox ends when the session is idle or stopped, and resuming starts a new one from the session's branches.
- **What the server stores:** the conversation, tool call names and card states, commit ids, measurements, spend and audit rows. Never tool output, file contents, diffs or data. Tool output and raw logs stay on the platform, are fetched when opened and may expire; the session says so.
- **Prompt injection is assumed.** Every agent reads untrusted text; what a hijacked agent holds and what still gets through is in [session threat model](../session/threat-model.md).
- **A person's terminal** in the agent's sandbox is audited, and its output is not stored.
- **What agents are told is part of the product** and is versioned and tested. See [session prompts](../session/prompts.md).
- **Rules kept:** one translator per outside system; Antilog never reshapes or pipes data; a session contacts only the endpoints its environment allows; a stopped session is ended and deleted at once.

## Projects, Customize and data

- **Projects.** Every experiment belongs to exactly one project. A project has members as owner, editor or viewer; the organization role is the default in every project and a project can set another for that project. Organization owners manage organization settings; project owners manage the project's members, budget, cap and value models. Permission is checked at project scope. See [organization model](../organization/model.md).
- **Customize** is one sidebar page, a library of reusable things, with tabs Evaluations, Skills, Connectors, Agents and Plugins, each with Yours and Discover. Every item opens the one DetailPage layout. Setting one up with a card works too, and what a card adds appears here.
  - **Evaluations** are reusable templates (kind Speed, Quality, Size or Cost). A sealed experiment starts from a copy, and the seal checks stay the judge's.
  - **Skills** are versioned skill packs. The judge never reads one, a pack never holds holdout data, and an experiment records the versions read. See [session add-skill-pack](../session/add-skill-pack.md).
  - **Connectors** are Snowflake, Databricks, BigQuery, Postgres, DynamoDB, Amazon S3 and an MCP server. The code host is not a connector.
  - **Agents** are listed with model and use.
  - **Plugins** bundle the other kinds. A plugin has no powers of its own and can never mark, unmark or grant a dataset. An item in use is never removed.
- **Two credentials per connector,** registered once and kept by the server, never in a session machine. The agent credential is read-only and leaves out the holdout; the judge credential is separate and also covers it. Both are short-lived and minted per session.
- **Datasets and the holdout.** Datasets live under a connector, and a dataset version never changes. A holdout is a marking enforced by the credentials; the grant control is absent, not disabled, and the row reads "Restricted to the judge". A holdout value never appears in history, a message or a receipt shown to an agent. A connector error shows datasets as Not checked, never empty.
- **Small things.** @-mention of a session in the composer, and a one-time prompt after the first measurement, "Notify me when this experiment needs me".

## The three aims

Beside trust: cost control, no tie to one model provider, and Return for the organization. They sit at the top of the [product skill](SKILL.md).

- **Value models.** What one unit of a metric is worth in dollars a year, with a source and who set it. A project owner sets one per metric; a different one proposed on the Seal experiment card needs a project owner's approval. It is sealed with the experiment. With none, the experiment shows "No value set" and counts in no dollar figure. Agents never set one.
- **Return** is Delivered divided by everything spent in the scope (project, organization, time range), including experiments that found nothing, were stopped or failed. A regressed win is At risk, not Delivered.
- **Auto picks the agent** with the best cost per win for this evaluation kind in this project, among the agents the organization turned on, and says why. Before there is data it uses the organization's default agent, set by an owner in Customize, Agents. A person can always pick a named agent, and no agent is preselected by brand.
- **Cost views.** Cost per trial, an estimate on the Seal experiment card, Where the spend went, the agent scoreboard (measured on the customer's code, never a vendor benchmark), hand off, failover, compaction, Ideas and a monthly Report. See [session model](../session/model.md), [judge model](../judge/model.md) and [organization approvals](../organization/approvals.md).

## Business model

- **Services first, on our own platform.** Antilog is sold as engagements: our engineers work inside a client's organization with Antilog and deliver measured wins. Client engineers get their own roles as the engagement goes on and keep using Antilog after it ends. The platform's job is to make each engagement cheaper than the last: time to first win, engineer-days per engagement and evaluations reused are tracked.
- **Pricing.** A fixed fee per engagement, a capped success fee on verified wins, and a platform subscription once client engineers run it themselves. The success fee rests on the value model, which the client signs off on the Seal experiment card, so a dispute is settled before work starts.
- **Named offers, sold broadly.** Engagements are sold as modernisation with proof, delivered through a few named offers with ready evaluations and value models (cost and latency, build and test time first). Evaluation kinds and connectors are built as paying engagements need them.
- **Two phases.** Phase 1 is Antilog cloud: we run it on our AWS account for early clients and pilots. Phase 2 is self-hosted in the client's own cloud account (AWS, Azure or GCP), which is what the Fortune 100 buy. Both run the same container images and Helm chart, so every phase 1 engagement tests what phase 2 ships. At least one Fortune 100 pilot, on a low-risk repo, runs in phase 1.
- **Our staff across organizations.** Our engineers work in several client organizations at once: one sign-in reaches each organization they are members of, and every action is in that organization's audit log.
- **Compute is rebilled.** Platform spend (tokens, sandboxes, judge time) is metered by the budget and appears on the client's invoice. Self-hosted, it is on the client's own cloud bill.

## The screen and the clients

- **Frame and pages.** A fixed 256px sidebar and a main area; a New experiment composer with five chips (Project, Repos, Agent, Environment, Budget), not a dialog; an experiment page that is the chat of its lead session; Projects, a project page, Customize, an experiment detail page, a receipt page and Settings as a dialog. A new experiment starts Not sealed: the agent explores, measures the baseline and proposes a definition with `seal`, and it may stay Not sealed and end there. The composer follows Claude Code's, with a Mode per experiment (Ask or Follow policy; "Auto" names only the agent choice). A card is the only place structured output appears. See [design behaviour](../design/behaviour.md).
- **The chat's voice is Antilog.** An agent's message shows the Antilog mark and the name "Antilog", with the agent that did the work as a quiet second label. Agent logos stay where an agent is the subject.
- **Clients.** The web app, the `antilog` CLI (`antilog "cut checkout p95"` opens the same session in the terminal) and `antilog mcp` (all target), and the native macOS app, which stays ruled in with its boards removed until the web direction is approved. Clients share behaviour, words and element ids, never pixels.
- **Windows and Linux** use the web app installed as a progressive web app. On macOS it points to the Mac app. Notifications (a win, Waiting for you, a regression, a budget spent) carry ids only ([organization notifications](../organization/notifications.md)). The device stores no data: the service worker caches the app's own files only.
- **The Mac app** is a direct download, signed, notarized and updated with Sparkle from one signed feed; checks can be turned off and releases deployed through device management. See [stack/swift/antilog.md](../stack/swift/antilog.md).

## Offerings, platforms and vendor facts

- **Two offerings, one core.** Antilog cloud (phase 1) and self-hosted (phase 2) run the same server with Postgres. Antilog cloud: we run it on our AWS account. Self-hosted: the client runs it in its own AWS, Azure or GCP account. Nothing is specific to one cloud except a thin infrastructure module per cloud; see Deployment below.
- **Timing is noisier on shared sandboxes,** so the noise band is wider and fewer trials are Kept; paired measurement cancels drift between sandboxes, and the experiment detail page says so.
- **"This machine" is deferred.** Antilog never starts an agent on a Mac until a later ruling. `antilog mcp` is separate: it serves the tools to an agent a person runs themselves, and that agent's trials are still measured only in a judge sandbox. Measuring never runs on a laptop.

## Licence

Antilog is licensed under the Functional Source License, FSL-1.1-ALv2: the source is public and anyone may run, change and self-host it, but not offer it as a competing product or service, and each release becomes Apache-2.0 two years after it is published. The licensor is Duaility, LLC; the text is in [`LICENSE.md`](../../../LICENSE.md). Every dependency's licence must allow this ([stack/libraries.md](../stack/libraries.md)).

## Deployment

- **The server is cloud-agnostic.** One Rust server (`antilog-server`) with the web app built in, and Postgres 18 as the only stateful dependency, which also holds the job queue and live events. Files and exports go to object storage through one crate that speaks S3, Azure Blob and GCS. Secrets are read from mounted files or the environment, never through a cloud SDK. Cloud access is by workload identity, with no stored cloud keys. Telemetry is OpenTelemetry, exported to whatever the client uses. Images are signed, with an SBOM, and can be pushed to a client's own registry for an air-gapped install.
- **One OpenTofu module per cloud** creates managed Postgres (RDS, Azure Database for PostgreSQL, Cloud SQL), a bucket, a key in the cloud's key service and workload identity. A client with its own cluster uses the chart alone. AWS ships first; Azure and GCP are built when the first client on each signs.
- **Antilog cloud starts pay-per-use, then grows in steps.** Phase 1 on our AWS runs on Lambda, serving the unchanged axum `Router` through `lambda_http`, behind a function URL with response streaming, with Aurora Serverless v2 (PostgreSQL) at a minimum of 0 ACUs so it pauses when idle. Cost tracks traffic: about $3 to $5 a month idle (Aurora's paused storage, Secrets Manager, Route 53, S3), Aurora about $0.06 an hour while awake, and Lambda a fraction of a cent a request. The first request after a pause waits about 15 seconds. AgentCore and Bedrock are per use on top. A client who needs steady traffic, or whose contract requires it, gets EKS with RDS, one namespace and one database per client (about $200 a month). Clients relying on it: Multi-AZ RDS and more nodes (about $400 to $500 a month). A client whose contract requires it gets its own database instance or cluster. GCP (Cloud Run) and Azure (Container Apps) scale to zero the same way and are built when a client on each cloud needs it.
- **Scale to zero forces four rules.** (1) Nothing runs after a response is sent: no spawned task, in-process timer or background loop. (2) Work that must happen later is a row in Postgres, claimed by a poller (the runner), not by a timer in the server. (3) Lambda caps an invocation at 15 minutes, so long work (waiting on a measurement or a watch) is asked of the session's machine and answered by a later call. (4) Live event streams end before the cap and the browser reconnects with `Last-Event-ID`, replayed from the events table; `LISTEN/NOTIFY` is for the Kubernetes setup only.
- **Antilog cloud in phase 1** runs in the US. Organizations join by invitation. Sign-in is GitHub and Google; repos come from GitHub only, through a GitHub App. The first cut runs only models AgentCore reaches through Amazon Bedrock, Claude first, so no model key enters a sandbox; Claude is also on Microsoft Foundry and Vertex AI for phase 2 on Azure and GCP.
- **We are a data processor** for every Antilog cloud organization: a data processing agreement, listed subprocessors (AWS, and Anthropic through Bedrock) and a stated retention rule. Antilog keeps no tool output, file contents or diffs. Each platform's retention and residency is stated beside it: the vendors promise only where data is stored at rest, and the OpenAI Agents API offers United States residency only and no zero data retention. Self-hosted, the data stays in the client's account and a services agreement sets our engineers' access.
- **Security paperwork starts in phase 1.** SOC 2 Type I is started during phase 1, since the Fortune 100 ask for it on the first call.

## Sign-in

No passwords. Antilog cloud offers exactly two sign-ins, GitHub (OAuth with its user API, since GitHub offers no OpenID Connect for app sign-in) and Google (OpenID Connect). It has no per-organization single sign-on (owner's call, 17 Nov 2026). A self-hosted server uses the client's own OpenID Connect provider (Entra ID, Okta, Google).

## Decided by the owner

| Date | Decision |
| --- | --- |
| 14 and 15 Nov 2026 | A native macOS app, SwiftUI, macOS only at launch. |
| 15 and 16 Nov 2026 | Antilog cloud on Cloudflare, one Durable Object per organization. Replaced on 17 Nov by AWS and Kubernetes. |
| 16 Nov 2026 | The design is a chat and a judge. |
| 16 Nov 2026 | The server stores the conversation's words, never tool output. |
| 16 Nov 2026 | The customer's CI runner is not a judge sandbox; the trait leaves room for it. |
| 16 Nov 2026 | "Experiment" names the goal; a session is one agent's work inside it. |
| 16 Nov 2026 | Projects, Customize, skill packs and plugins are in; notes and the dashboard page are out. |
| 16 Nov 2026 | The macOS app's boards are removed until the web direction is approved. |
| 16 Nov 2026 | No per-repo visibility; roles can be set per project. |
| 16 Nov 2026 | One budget per experiment, changeable at any time, inside the monthly caps. |
| 16 Nov 2026 | The composer follows Claude Code's, with Mode (Ask or Follow policy). |
| 16 Nov 2026 | The chat speaks as Antilog. An agent may read the evaluation's code, never change it. |
| 16 Nov 2026 | The three aims, with value models, Auto, Return on all spend, sealed quick check and patience, and two new card kinds. |
| 16 Nov 2026 | The spike measures noise on each judge platform. |
| 16 Nov 2026 | Paired measurement; the bar is a lower confidence bound above zero at 1 − 0.05/k. |
| 16 Nov 2026 | AgentCore is the first adapter, for agents and the judge. |
| 16 Nov 2026 | A thin first release: self-hosted, AgentCore, the web app, Speed, and `antilog mcp`. Replaced the same day by cloud first. |
| 16 Nov 2026 | The licence is the Functional Source License (FSL-1.1-ALv2). |
| 16 Nov 2026 | Cumulative trials at patience, so small gains add up. |
| 16 Nov 2026 | Watch is paired: the delivered commit against the current head. |
| 16 Nov 2026 | Holdout as an oracle: read-only judge credential, a sealed cap on holdout checks, a holdout look-alike cheat. |
| 16 Nov 2026 | Prompt injection is assumed; the session threat model is written. |
| 16 Nov 2026 | Delivered is labelled "measured offline" until production scoring is decided. |
| 16 Nov 2026 | Cloud first: the first release is Antilog cloud; self-hosted follows. |
| 16 Nov 2026 | Invite only with no billing at first; Antilog pays under a hard cap per organization. Replaced on 17 Nov: compute is rebilled. |
| 16 Nov 2026 | US jurisdiction first, then the EU. |
| 16 Nov 2026 | Models on Bedrock only in the first cut, Claude first. |
| 16 Nov 2026 | GitHub is the only code host in the first cut. |
| 16 Nov 2026 | Three to five design partners at launch. Replaced on 17 Nov by paid engagements and pilots. |
| 17 Nov 2026 | Services first: our engineers deliver engagements on Antilog, client engineers take over. |
| 17 Nov 2026 | Pricing: a fixed fee per engagement, a capped success fee on verified wins, then a subscription. |
| 17 Nov 2026 | Phase 1 is Antilog cloud on our AWS; phase 2 is self-hosted in the client's AWS, Azure or GCP account. |
| 17 Nov 2026 | Cloudflare is dropped. Kubernetes, one Helm chart and one OpenTofu module per cloud. |
| 17 Nov 2026 | A Kubernetes sandbox adapter runs the judge on any cloud. |
| 17 Nov 2026 | Demos run on k3s on one EC2 instance; EKS from the first paying client. Replaced the same day by pay-per-use. |
| 17 Nov 2026 | Phase 1 Antilog cloud runs pay-per-use (Lambda, Aurora Serverless v2 at zero); the Helm chart on EKS stays for clients who need steady traffic and for phase 2. Same server and image both ways. |
| 17 Nov 2026 | Value models, Return and the Report are in the first release. |
| 17 Nov 2026 | The product is renamed Antilog; it is hosted at antilog.tech. |
| 17 Nov 2026 | Antilog cloud sign-in is GitHub and Google only, with no per-organization single sign-on. Self-hosted keeps the client's own OpenID Connect provider. |
| 17 Nov 2026 | Antilog cloud runs in AWS region us-east-1. |
| 18 Nov 2026 | Agents on AgentCore use GLM 4.7 Flash on Bedrock, with Qwen3 Coder 30B as the fallback; the test account allows only those two models. |
| 18 Nov 2026 | No username and password, not even for testing. Agents and tests sign in through fakes: access tokens and a fake sign-in provider. |
| 18 Nov 2026 | An organization always keeps at least one owner, as a project does. |
| 18 Nov 2026 | Done and Failed sessions are final; resuming means a new session. |
| 18 Nov 2026 | The seal is refused when the smallest gain the setup can detect is above 10% of the baseline; tuned after the spike. |
| 18 Nov 2026 | Only the receipt is signed; it carries a hash of each of its measurements. |
| 18 Nov 2026 | The kill-switch runs under a role the owner creates and holds, outside `antilog-*`, so the agent's boundary cannot reach it. |
| 18 Nov 2026 | antilog.tech's DNS and the landing page are on Cloudflare (free plan, Pages); the product stays on AWS at `app.antilog.tech`, through a DNS-only record to CloudFront. A narrow exception to "Cloudflare is dropped" (17 Nov). The owner sets the domain up by hand; OpenTofu takes the certificate ARN as an input and creates no DNS. |
| 18 Nov 2026 | The antilog.tech landing page (`site/index.html`) spoke to CTOs and VPs of engineering as "your cloud bill, edited by autonomous agents": agents propose, a judge decides, people approve. It is a marketing page outside the design rules (its own fonts, no board), and its figures are invented examples labelled "Illustrative example". The product and its boards are unchanged. Contact is hello@antilog.tech. The audience and headline were replaced on 19 Nov; the rest stands. |
| 19 Nov 2026 | Messaging: Antilog is an autonomous agent platform for business outcomes that can be quantified (cost down, revenue up, errors down, risk down); cloud spend is one example, not the pitch. The spine is "You set the outcome. Agents do the work. Antilog shows the return.", and the landing page headline is "Autonomous agents, measured by business outcomes", for business readers, with the judge explained once in plain words. The landing page never mentions services, its button is "Request access" (hello@antilog.tech), and outcomes beyond cost and speed in code are labelled illustrative examples. Words not used: "RSI", "AutoML", "self-improving AI", "multi-cloud", and any statistic we cannot back up. |
| 18 Nov 2026 | In dev, Aurora accepts connections from any address, protected by its password and forced TLS, because the Lambda runs outside a VPC (no NAT gateway). Before any client data, the function and the database move into a VPC and the ingress narrows. |

Decided calls also appear on the Decisions board.

## Still unproven: the spike

The spike on five agent platforms is retired (see `docs/V0.md`). What stays unproven:

- **Repeats.** How many paired repeats make the bar reachable. With 3 repeats and k = 10 the critical value is about 10, so almost nothing is Kept. The Seal experiment card should show the smallest gain the setup can detect.
- **The Speed holdout.** What a holdout is for a timing evaluation: other inputs, another load profile, or something else.
- **A first example that fits.** "Cut checkout p95" needs real services and load that a sandbox with no connectors does not have. Find a Speed experiment that can be measured honestly there, such as test suite or build time, or a library benchmark.
- **One branch per session.** A GitHub App installation token can push to any branch of a repo, not just the session's. Show that pushes can be held to the session's branch (for example a ruleset on a `antilog/*` prefix, or a push relay that checks the ref), or change the design.
- **Scale to zero.** How long Aurora's resume takes in practice and what the first request shows, whether a function URL streams server-sent events as designed, and how small the Lambda connection pool can be before it hurts.
- **Cost of one experiment.** Tokens, sandbox and judge time for one sealed experiment end to end, which sets engagement pricing.
- **Pair interference.** Whether warm caches, shared state or migrations leak between the two sides of a pair, and whether an ABBA order is enough.

## First release

Antilog cloud on our AWS (pay-per-use on Lambda and Aurora Serverless v2), with a runner and OpenCode ([docs/V0.md](../../../docs/V0.md)), GitHub sign-in and repos (and Google sign-in), the web app and `antilog mcp`, one evaluation kind (Speed) with no connectors: seal, measure, Kept, receipt, deliver, budget, approvals and the audit log, plus value models, Return and the Report, since they are what an engagement is paid on. Before the first client: the push scan and second opinion from the threat model, a data processing agreement, the subprocessor list, a retention and deletion rule, the cross-organization staff sign-in and spend alerts. Then, in this order unless the owner says otherwise: the Kubernetes sandbox adapter, self-hosted on the client's AWS, evaluation kinds and connectors as engagements need them, Azure and GCP with their adapters, plugins, Ideas, and the Mac app. Their fields stay in the sealed definition and the store from the start, so nothing is redesigned.

A `antilog mcp` connection works inside a session in the experiment, named by its agent, with no platform sandbox. Its trial commits are pushed to that session's branch with a short-lived git token minted for the session, and are measured only in a judge sandbox.

## Open questions

- Whether scoring is offline only or production results are used too.
- Whether Antilog ever reshapes data (today no, see [non-goals](non-goals.md)).
- Whether "Try a demo" stays in the web app.
- Whether every trial in the version, from every session and including quick-check stops, should count in k.
- How experiments on the same code interact: overlapping changes, a default branch that moves mid-experiment, one win making another's baseline stale. Today only delivery rebases.
- How a secret pasted into the chat is caught before the conversation is stored.
- Who holds the judge's signing key when self-hosted (on Antilog cloud it is Antilog's, in AWS Secrets Manager), how it rotates, and what "Signed by the judge" proves to someone outside the organization.
- The seal is the hardest step and seal checks catch only a trivially broken evaluation. How the product helps a team that has no benchmark.
- How time to merge is shown and eased, since value counts only after a merge.
- What the second opinion costs per win, and its data path: the diff goes to a second provider.
- The FSL's "competing use" wording and whether contributors sign an agreement.

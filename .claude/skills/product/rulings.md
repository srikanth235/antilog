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
- Two things are set up in Settings because no experiment can run without them: agent platform accounts (self-hosted) and members and sign-in. Connectors and agents are registered once in Customize, and a chat can ask for the same thing with an Add credential card, so nothing waits on it.

## The judge

- **A trait separate from the agent platform trait:** run this command at these commits with these secrets and return these numbers. See [judge rules](../judge/rules.md).
- **The trust rules never bend.** Only the judge's numbers count. An agent never holds the holdout credential. A sealed experiment never changes. A person or an agent can never mark a win: only the acceptance criteria can, as one pure function in code. Chat changes what an agent tries, never what counts.
- **Anti-cheat is built in:** a seal over the definition and protected files (a trial that changes a protected path is Rejected), a fresh judge sandbox for every measurement, seal checks, confirmation before Kept, a second opinion from a different model provider that flags and never decides, and a signed receipt. See [judge anti-cheat](../judge/anti-cheat.md).
- **Acceptance.** A trial is Kept when it beats the trial it started from by more than the bar, every guardrail holds, and it is confirmed in a fresh judge sandbox and on the holdout. An experiment's best is its best Kept trial. Outcomes: Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected, Failed.
- **What an agent sees.** The measured value and guardrail values on the evaluation data, and the outcome. It may read the evaluation's code and may never change it. It never reads the holdout or learns a holdout value.
- **Quick check and patience.** A measurement starts with a quick check (one repeat); the full measurement follows only if it beats the best by more than the noise. The quick check never decides Kept. When patience (default 6) trials in a row are not Kept, the experiment waits on a Keep going? card. Both are sealed.
- **Delivery and watch.** `deliver` opens one linked pull request per repo changed, each carrying the receipt, and refuses any trial that is not Kept. The session stays attached for review comments and CI failures, and a rebase is measured again. A delivered experiment can be watched; a regression wakes or starts a session and notifies.
- **Receipt.** Every win carries a signed receipt with Experiment, Data and Compute groups, and a Verify action. See [judge model](../judge/model.md).
- **Value.** A win is Delivered once its pull request is merged. Pending is not in Return. See the three aims below.

## Sessions, budget and storage

- **One agent per session,** with any number of repos. Git is the hand-off: each session gets a short-lived git token per repo that pushes only to its own branch, and Ratchet stores commit ids, never diffs.
- **Spawn.** A session may spawn others in the same experiment, on the same budget, on any agent and platform. A spawned session cannot spawn, appears as a card and opens as its own chat. See [session rules](../session/rules.md).
- **Budget.** One per experiment, covering all its sessions and judge time. It is set on the composer (default: the project's, then the organization's) and an editor or owner can change it at any time; lowering it below spend stops new measurements. When it is spent the judge refuses `measure` and the experiment is Waiting for you with a Raise budget card. It sits inside its project's optional monthly cap, inside the organization's.
- **The sandbox is temporary, the conversation is not.** A sandbox ends when the session is idle or stopped, and resuming starts a new one from the session's branches.
- **What the server stores:** the conversation, tool call names and card states, commit ids, measurements, spend and audit rows. Never tool output, file contents, diffs or data. Tool output and raw logs stay on the platform, are fetched when opened and may expire; the session says so.
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
- **Datasets and the holdout.** A dataset version never changes. A holdout is a marking enforced by the credentials; the grant control is absent, not disabled, and the row reads "Restricted to the judge". A holdout value never appears in history, a message or a receipt shown to an agent.
- **Small things.** @-mention of a session in the composer, and a one-time prompt after the first measurement, "Notify me when this experiment needs me".

## The three aims

Beside trust: cost control, no tie to one model provider, and Return for the organization. They sit at the top of the [product skill](SKILL.md).

- **Value models.** What one unit of a metric is worth in dollars a year, with a source and who set it. A project owner sets one per metric; a different one proposed on the Seal experiment card needs a project owner's approval. It is sealed with the experiment. With none, the experiment shows "No value set" and counts in no dollar figure. Agents never set one.
- **Return** is Delivered divided by everything spent in the scope (project, organization, time range), including experiments that found nothing, were stopped or failed. A regressed win is At risk, not Delivered.
- **Auto picks the agent** with the best cost per win for this evaluation kind in this project, among the agents the organization turned on, and says why. Before there is data it uses the organization's default agent, set by an owner in Customize, Agents. A person can always pick a named agent, and no agent is preselected by brand.
- **Cost views.** Cost per trial, an estimate on the Seal experiment card, Where the spend went, the agent scoreboard (measured on the customer's code, never a vendor benchmark), hand off, failover, compaction, Ideas and a monthly Report. See [session model](../session/model.md), [judge model](../judge/model.md) and [organization approvals](../organization/approvals.md).

## The screen and the clients

- **Frame and pages.** A fixed 256px sidebar and a main area; a New experiment composer with five chips (Project, Repos, Agent, Environment, Budget), not a dialog; an experiment page that is the chat of its lead session; Projects, a project page, Customize, an experiment detail page, a receipt page and Settings as a dialog. A new experiment starts Not sealed: the agent explores, measures the baseline and proposes a definition with `seal`, and it may stay Not sealed and end there. The composer follows Claude Code's, with a Mode per experiment (Ask or Auto, within policy). A card is the only place structured output appears. See [design behaviour](../design/behaviour.md).
- **The chat's voice is Ratchet.** An agent's message shows the Ratchet mark and the name "Ratchet", with the agent that did the work as a quiet second label. Agent logos stay where an agent is the subject.
- **Clients.** The web app, the `ratchet` CLI (`ratchet "cut checkout p95"` opens the same session in the terminal) and `ratchet mcp` (all target), and the native macOS app, which stays ruled in with its boards removed until the web direction is approved. Clients share behaviour, words and element ids, never pixels.
- **Windows and Linux** use the web app installed as a progressive web app. On macOS it points to the Mac app. Notifications (a win, Waiting for you, a regression, a budget spent) carry ids only ([organization notifications](../organization/notifications.md)). The device stores no data: the service worker caches the app's own files only.
- **The Mac app** is a direct download, signed, notarized and updated with Sparkle from one signed feed; checks can be turned off and releases deployed through device management. See [stack/swift/ratchet.md](../stack/swift/ratchet.md).

## Offerings, platforms and vendor facts

- **Two offerings, both from day 0, one core.** Self-hosted: Docker Compose (server and Postgres) using agent platform accounts the customer has. Ratchet cloud: we host the server and run every session on Ratchet's platform accounts, billed at cost through the budget.
- **Ratchet runs no sandboxes for agents. Agent platforms do:** Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service and Google Agent Runtime, each behind one agent trait with one adapter (start a session, send input, read events, stop and delete it). Foundry and Google run a container image Ratchet supplies, one per agent.
- **An agent is a model on a platform:** Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AgentCore, GitHub Copilot on Foundry, Gemini (ADK) on Google Agent Runtime.
- **Build order.** The traits are designed against all five from the start. Adapters ship: Claude Managed Agents, AgentCore, OpenAI Agents API, Foundry Agent Service, Google Agent Runtime.
- **Where the judge sandbox runs:** on a platform that can run a command in a sandbox with no agent. AgentCore can. Google Agent Runtime is checked in the spike. Claude Managed Agents, the OpenAI Agents API and Foundry cannot; on Foundry the likely path is a judge image deployed as a hosted agent that runs no model.
- **`measure` is asynchronous** so no platform limit applies (Foundry may expire tool outputs after 10 minutes; Google's event stream is undocumented and limited to 10 minutes per query). The spike checks it.
- **Credentials reach a session only through the platform's credential store,** never a prompt, a repo file or Ratchet's database. No model key enters a sandbox.
- **Ratchet cloud uses only Ratchet's platform accounts at launch.** The adapters support customer accounts because self-hosted needs them. An enterprise that needs data read in its own network self-hosts, for example with AgentCore in its own AWS account.
- **Timing is noisier on shared sandboxes,** so the noise band is wider and fewer trials are Kept; the experiment detail page says so.
- **No Helm chart and no OpenTofu at launch.** Self-hosted ships as Docker Compose only; Ratchet cloud ships with the Cloudflare config and `cf`. Ratchet's own platform accounts are set up by hand and owned by the owner.
- **"This machine" is deferred.** Agents never run on a Mac until a later ruling. Measuring never runs on a laptop.

## Ratchet cloud

- **Storage.** Self-hosted uses Postgres 18. Ratchet cloud uses one SQLite database per organization inside one Cloudflare Durable Object, pinnable to the EU, the US or FedRAMP. Moving between them is an export and import of one neutral file format.
- **One core, two shells.** `ratchet-core` holds all logic and compiles natively and to WebAssembly; storage sits behind one trait. `ratchet-server` and `ratchet-cloud` are the shells. See [stack](../stack/SKILL.md).
- **We manage everything.** An organization signs in, attaches repos and starts, and runs no infrastructure.
- **Ratchet's platform credentials** sit in Cloudflare's Secrets Store, bound to the Worker, never in an organization's database, a response or a log.
- **We are a data processor** for every Ratchet cloud organization: a data processing agreement, listed subprocessors (Cloudflare and the agent platforms in use) and a stated retention rule. Ratchet keeps no tool output, file contents or diffs. Each platform's retention and residency is stated beside it: the vendors promise only where data is stored at rest, and the OpenAI Agents API offers United States residency only and no zero data retention.

## Sign-in

OpenID Connect plus GitHub's OAuth, no passwords. Ratchet cloud offers GitHub (OAuth with its user API, since GitHub offers no OpenID Connect for app sign-in), Google (OpenID Connect) and an organization's own OpenID Connect provider (single sign-on, set up only by an owner). A self-hosted server uses the administrator's provider.

## Decided by the owner

| Date | Decision |
| --- | --- |
| 14 and 15 Nov 2026 | A native macOS app, SwiftUI, macOS only at launch. |
| 15 and 16 Nov 2026 | Ratchet cloud on Cloudflare, one Durable Object per organization. |
| 16 Nov 2026 | The design is a chat and a judge. |
| 16 Nov 2026 | The server stores the conversation's words, never tool output. |
| 16 Nov 2026 | The customer's CI runner is not a judge sandbox; the trait leaves room for it. |
| 16 Nov 2026 | "Experiment" names the goal; a session is one agent's work inside it. |
| 16 Nov 2026 | Projects, Customize, skill packs and plugins are in; notes and the dashboard page are out. |
| 16 Nov 2026 | The macOS app's boards are removed until the web direction is approved. |
| 16 Nov 2026 | No per-repo visibility; roles can be set per project. |
| 16 Nov 2026 | How the bar rises with the trials tried is settled with the first code. |
| 16 Nov 2026 | One budget per experiment, changeable at any time, inside the monthly caps. |
| 16 Nov 2026 | The composer follows Claude Code's, with Mode (Ask or Follow policy). |
| 16 Nov 2026 | The chat speaks as Ratchet. An agent may read the evaluation's code, never change it. |
| 16 Nov 2026 | The three aims, with value models, Auto, Return on all spend, sealed quick check and patience, and two new card kinds. |

Decided calls also appear on the Decisions board.

## Still unproven: the spike

The spike comes before any product code. One tiny experiment is measured end to end on all five platforms. It proves the agent trait, the Ratchet tools served to a session over its event stream and a judge sandbox that holds a credential no agent sees.

## Open questions

- Whether scoring is offline only or production results are used too.
- Whether Ratchet ever reshapes data (today no, see [non-goals](non-goals.md)).
- Whether "Try a demo" stays in the web app.
- The licence: the workspace manifest says `UNLICENSED` today, and what it becomes is not decided.

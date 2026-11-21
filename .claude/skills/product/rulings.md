# Rulings

The settled product, directed by the owner, as [docs/V0.md](../../../docs/V0.md) shapes it: v0 is cloud sessions only, copying Claude Code's cloud sessions (a runner, OpenCode, one proxy, no infrastructure tool, customers bring model keys). The runner, proxy, environments and the org lock are built; the real code host is not. Cut features are in [non-goals](non-goals.md), the words in [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md).

## A chat in a cloud session

- **Antilog is a chat with a coding agent in a cloud session.** A person talks to an agent that runs in a machine of its own with repos attached, started in a project. The agent asks for setup through Antilog's tools and a person approves each request.
- **No modes, no plan step, no orchestration loop on the server.** The agent drives. The only server-side state machines are the runner's queue and the budget meter.
- **Three things.** A **project** groups sessions for one area of the code. A **session** is one agent's work in one machine and belongs to one project. A **card** is a tool call a person approves. Reusable setup lives in one Customize page.
- **Session.** Its fields and routes are in [session model](../session/model.md). Status: Working, Waiting for you, Done or Failed; a stopped one is Done, "Stopped by NAME". The sidebar lists projects, each with its sessions.
- **Tools.** A session's MCP endpoint (`/mcp/sessions/{id}`) serves the setup tools, each a card: `attach_repo`, `request_credential`, `set_environment`, `request_budget`, plus the tools of every organization connector (On, Off or Ask first). See [session tools](../session/tools.md). There is no `antilog mcp` and no REST tool endpoint.

## Settings are tools

- A person approves the tool calls that matter, and settings pages list and revoke what was approved. Setup happens in the chat, at the moment it is needed, as a card.
- Six card kinds: Raise budget, Add credential, Attach repo, Change environment, Allow this tool? (`ask_first`) and Use a connector tool. Defaults, policy, Mode and who may approve are in [organization approvals](../organization/approvals.md).
- Add credential always asks and can never be pre-approved: a person types the value into a secure form that goes straight to the server's secret store.
- Roles decide who may approve which card; a viewer approves nothing. The audit log lists tool calls that changed something and approvals.
- Two things are set up in Settings because no session can run without them: model keys (saved per organization per provider) and members and sign-in. Connectors and agents are registered once in Customize, and a chat can ask for the same thing with a card, so nothing waits on it.

## Sessions, budget and storage

- **One agent per session,** with any number of repos. The repos, the environment version, the agent and the model are fixed when it starts.
- **Budget.** Caps nest: a session's own optional cap is at most its project's monthly cap, which is at most the organization's monthly cap. A new session's cap defaults to the project's default per session, then the organization's. The proxy refuses model calls with a 402 once any of them is spent; the session is then Waiting for you, and `request_budget` raises its own cap through a Raise budget card. See [organization approvals](../organization/approvals.md).
- **The machine is temporary, the conversation is not.** A machine is released when the session is idle or stopped, and resuming starts a new one from the exported history.
- **What the server stores:** the conversation, tool call names and card states, spend and audit rows. Never tool output, file contents, diffs or data.
- **Prompt injection is assumed.** Every agent reads untrusted text; what a hijacked agent holds and what still gets through is in [session threat model](../session/threat-model.md).
- **A person's terminal** in the session's machine is audited, and its output is not stored.
- **What agents are told is part of the product.** See [session prompts](../session/prompts.md).
- **Rules kept:** one translator per outside system; Antilog never reshapes or pipes data; a session contacts only the endpoints its environment allows; a stopped session's machine is released and its disk dropped.

## Projects and Customize

- **Projects.** Every session belongs to exactly one project. A project has members as owner, editor or viewer; the organization role is the default in every project and a project can set another for that project. Organization owners manage organization settings and create projects; project owners manage the project's roles and limits (monthly cap, default budget per session). Permission is checked at project scope. See [organization model](../organization/model.md).
- **Customize** is one sidebar page, a library of reusable things, with tabs Skills, Connectors, Agents and Plugins. Every item opens the one DetailPage layout. Setting one up with a card works too, and what a card adds appears here. Every session gets all connectors and the latest version of every skill.
  - **Skills** are versioned `SKILL.md` skill packs. See [session add-skill-pack](../session/add-skill-pack.md).
  - **Connectors** are remote MCP servers ([docs/V0.md](../../../docs/V0.md)). The code host is not a connector. A connector's credential is kept sealed on the server and never sent to a session machine.
  - **Agents** are listed with model and use; an owner sets the default agent.
  - **Plugins** bundle skills and connectors. A plugin has no powers of its own.

## Parked: experiments and the judge

Owner, 21 Nov 2026: v0 is only cloud sessions. Experiments and the judge are parked, not redesigned, and their code is deleted (not hidden). The last commit that has all of it is `daa445f` on `claude/dreamy-ptolemy-vg7uga`. What was there: a sealed experiment (metric, evaluation, protected paths, guardrails, data and holdout, repeats, acceptance criteria), the judge's tools (`seal`, `measure`, `history`, `deliver`, `watch`), trials and measurements with a paired bar, the holdout machine, Kept trials and wins, signed receipts, value models, Delivered, Pending, At risk, Return, the Report, evaluation templates, datasets, the agent scoreboard, Auto by cost per win, hand off, failover, pull-request delivery and watch. The iteration ideas of 21 Nov (a draft stage, diff-based versions, trials that carry across versions, k counted across the experiment, a minimal seal input, a version timeline) wait with them. The earlier rulings, promises and skill pages are readable in that commit. If the judge returns, restore from there in a change the owner asks for.

The business model the owner set on 17 Nov (services first, a capped success fee on verified wins, named offers) rested on measured wins and waits with the judge; the platform subscription and the two phases below stand.

## Customer and phases

- **Two phases.** Phase 1 is Antilog cloud: we run it for early clients and pilots. Phase 2 is self-hosted in the client's own cloud account, which is what the Fortune 100 buy. Both run the same server and proxy binaries.
- **Our staff across organizations.** Our engineers work in several client organizations at once: one sign-in reaches each organization they are members of, and every action is in that organization's audit log.
- **Compute is rebilled.** Compute (session machines) is metered by the budget and appears on the client's invoice. Model calls are the customer's own: they bring their keys and the proxy meters the spend.

## The screen and the clients

- **Frame and pages.** A fixed 256px sidebar and a main area; a New session composer with five chips (Project, Repos, Agent, Environment, Budget), not a dialog; a session page that is the chat; Projects, a project page, Customize and Settings as a dialog. The composer follows Claude Code's, with a Mode per session (Ask or Follow policy; "Auto" names only the agent choice). A card is the only place structured output appears. See [design behaviour](../design/behaviour.md).
- **The chat's voice is Antilog.** An agent's message shows the Antilog mark and the name "Antilog", with the agent that did the work as a quiet second label.
- **Clients.** The web app and the native macOS app, which stays ruled in with its boards removed until the web direction is approved. The CLI is `antilog runner` only. Clients share behaviour, words and element ids, never pixels.
- **Windows and Linux** use the web app installed as a progressive web app. On macOS it points to the Mac app. Notifications carry ids only ([organization notifications](../organization/notifications.md)).
- **The Mac app** is a direct download, signed, notarized and updated with Sparkle. See [stack/swift/antilog.md](../stack/swift/antilog.md).

## Licence

Antilog is licensed under the Functional Source License, FSL-1.1-ALv2: the source is public and anyone may run, change and self-host it, but not offer it as a competing product or service, and each release becomes Apache-2.0 two years after it is published. The licensor is Duaility, LLC; the text is in [`LICENSE.md`](../../../LICENSE.md). Every dependency's licence must allow this ([stack/libraries.md](../stack/libraries.md)).

## Deployment

- **The server is cloud-agnostic.** One Rust server (`antilog-server`) with the web app built in, and Postgres 18 as the only stateful dependency, which also holds the job queue and live events. Files and exports go to object storage through one crate that speaks S3, Azure Blob and GCS. Secrets are read from mounted files or the environment, never through a cloud SDK. Cloud access is by workload identity, with no stored cloud keys. Telemetry is OpenTelemetry, exported to whatever the client uses. Images are signed, with an SBOM, and can be pushed to a client's own registry for an air-gapped install.
- **Antilog cloud runs on a small always-on host.** The server and the proxy are plain binaries on one small host, with managed Postgres; session machines run on a pool of hosts, each locked to one organization ([docs/V0.md](../../../docs/V0.md)). An agent writes and runs whatever deploys them, on demand; the repo has no infrastructure code.
- **Work that must happen later is a row in Postgres**, claimed by a poller (the runner, or the server's own loop), so a restart loses nothing. Live event streams end after a few minutes and the browser reconnects with `Last-Event-ID`, replayed from the events table.
- **Antilog cloud in phase 1** runs in the US. Organizations join by invitation. Sign-in is GitHub and Google; repos come from GitHub only, through a GitHub App. Customers bring their own model keys (Anthropic and OpenAI first), held by the proxy and never sent into a session.
- **We are a data processor** for every Antilog cloud organization: a data processing agreement, listed subprocessors and a stated retention rule. Antilog keeps no tool output, file contents or diffs. Model calls go to the customer's own provider account through the proxy, so the model vendor's retention is the customer's agreement. Self-hosted, the data stays in the client's account and a services agreement sets our engineers' access.
- **Security paperwork starts in phase 1.** SOC 2 Type I is started during phase 1, since the Fortune 100 ask for it on the first call.

## Sign-in

No passwords. Antilog cloud offers exactly two sign-ins, GitHub (OAuth with its user API, since GitHub offers no OpenID Connect for app sign-in) and Google (OpenID Connect). It has no per-organization single sign-on (owner's call, 17 Nov 2026). A self-hosted server uses the client's own OpenID Connect provider (Entra ID, Okta, Google).

## Decided by the owner

Rows that name experiments, the judge, value models, receipts or holdout were decided for the design now parked (see above); they stand as history, not as live rules.

| Date | Decision |
| --- | --- |
| 14 and 15 Nov 2026 | A native macOS app, SwiftUI, macOS only at launch. |
| 16 Nov 2026 | The design is a chat and a judge. |
| 16 Nov 2026 | The server stores the conversation's words, never tool output. |
| 16 Nov 2026 | "Experiment" names the goal; a session is one agent's work inside it. |
| 16 Nov 2026 | Projects, Customize, skill packs and plugins are in; notes and the dashboard page are out. |
| 16 Nov 2026 | The macOS app's boards are removed until the web direction is approved. |
| 16 Nov 2026 | No per-repo visibility; roles can be set per project. |
| 16 Nov 2026 | One budget per experiment, changeable at any time, inside the monthly caps. |
| 16 Nov 2026 | The composer follows Claude Code's, with Mode (Ask or Follow policy). |
| 16 Nov 2026 | The chat speaks as Antilog. An agent may read the evaluation's code, never change it. |
| 16 Nov 2026 | The three aims, with value models, Auto, Return on all spend, sealed quick check and patience, and two new card kinds. |
| 16 Nov 2026 | Paired measurement; the bar is a lower confidence bound above zero at 1 − 0.05/k. |
| 16 Nov 2026 | The licence is the Functional Source License (FSL-1.1-ALv2). |
| 16 Nov 2026 | Cumulative trials at patience, so small gains add up. |
| 16 Nov 2026 | Watch is paired: the delivered commit against the current head. |
| 16 Nov 2026 | Holdout as an oracle: read-only judge credential, a sealed cap on holdout checks, a holdout look-alike cheat. |
| 16 Nov 2026 | Prompt injection is assumed; the session threat model is written. |
| 16 Nov 2026 | Delivered is labelled "measured offline" until production scoring is decided. |
| 16 Nov 2026 | Cloud first: the first release is Antilog cloud; self-hosted follows. |
| 16 Nov 2026 | US jurisdiction first, then the EU. |
| 16 Nov 2026 | GitHub is the only code host in the first cut. |
| 16 Nov 2026 | Three to five design partners at launch. Replaced on 17 Nov by paid engagements and pilots. |
| 17 Nov 2026 | Services first: our engineers deliver engagements on Antilog, client engineers take over. |
| 17 Nov 2026 | Pricing: a fixed fee per engagement, a capped success fee on verified wins, then a subscription. |
| 17 Nov 2026 | Value models, Return and the Report are in the first release. |
| 17 Nov 2026 | The product is renamed Antilog; it is hosted at antilog.tech. |
| 17 Nov 2026 | Antilog cloud sign-in is GitHub and Google only, with no per-organization single sign-on. Self-hosted keeps the client's own OpenID Connect provider. |
| 18 Nov 2026 | No username and password, not even for testing. Agents and tests sign in through fakes: access tokens and a fake sign-in provider. |
| 18 Nov 2026 | An organization always keeps at least one owner, as a project does. |
| 18 Nov 2026 | Done and Failed sessions are final; resuming means a new session. |
| 18 Nov 2026 | Only the receipt is signed; it carries a hash of each of its measurements. |
| 18 Nov 2026 | The kill-switch runs under a role the owner creates and holds, outside `antilog-*`, so the agent's boundary cannot reach it. |
| 18 Nov 2026 | The antilog.tech landing page (`site/index.html`) spoke to CTOs and VPs of engineering as "your cloud bill, edited by autonomous agents": agents propose, a judge decides, people approve. It is a marketing page outside the design rules (its own fonts, no board), and its figures are invented examples labelled "Illustrative example". The product and its boards are unchanged. Contact is hello@antilog.tech. The audience and headline were replaced on 19 Nov; the rest stands. |
| 20 Nov 2026 | No Lambda and no infrastructure code in the repo; the server and proxy are plain binaries, and agents write and run deploy code on demand. |
| 21 Nov 2026 | v0 is only cloud sessions, copying Claude Code's. Experiments and the judge are parked and their code deleted (`daa445f` keeps it). |
| 21 Nov 2026 | Projects stay: a project groups sessions, as Claude Code's sidebar does; a session belongs to one project. |
| 21 Nov 2026 | Spend is capped by the organization (monthly), the project (monthly) and optionally the session; caps nest. |

Decided calls also appear on the Decisions board.

## Open questions

- Whether "Try a demo" stays in the web app.
- How a secret pasted into the chat is caught before the conversation is stored.
- How a pull request is opened: by the agent with the repo's credentials, a code host, or both.
- Whether session machines get a microVM or VM backend before self-hosting.
- The FSL's "competing use" wording and whether contributors sign an agreement.

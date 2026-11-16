# Rulings

Everything here is target; nothing is built on this branch. Each section is dated and says who directed it.

## The design: a chat and a judge (target)

Directed by the owner, 16 Nov 2026. It replaces the earlier design (see "What this replaces").

- **Ratchet is a chat and a judge.** The chat works like Claude Code on the web: a person talks to a coding agent in a sandbox with repos attached. The judge is a set of tools every session gets. Only the judge's numbers count, and only a measured win becomes a pull request. There are no modes, no plan step and no orchestration loop on the server, and nothing sits between a session and a contract (see "What this replaces").
- **Three objects.** A **session** (a chat with one agent in one sandbox on one agent platform, with any number of repos; status Working, Waiting for you, Done or Failed; a session a person stops is Done, "Stopped by NAME"). A **contract** (a sealed definition of better: metric and direction, the evaluation (a command and its protected files), guardrails, the data (evaluation data and a holdout), repeats and the acceptance criteria; immutable once sealed, a change makes a new version, numbers never carry across versions). A **measurement** (the judge's signed result for one trial). A **trial** is one `measure` call: a set of commits (one per repo changed), the trial it started from and a one-line hypothesis. Everything else (scoreboard, wins, receipts, value, Return) is a view over measurements.
- **The tools.** Every session gets the Ratchet tools, and `ratchet mcp` (target) serves the same tools to an agent a person already uses on their own machine, so a change can be proposed anywhere and is measured only by the judge. Work tools: `seal`, `measure` (asynchronous: a ticket now, the measurement later as an event), `history`, `spawn`, `deliver`, `watch`. Setup tools, each shown as a card: `attach_repo`, `request_credential`, `set_environment`, `request_budget`. The pages are [session tools](../session/tools.md) and [judge tools](../judge/tools.md).
- **Settings are tools.** A person approves the tool calls that matter, and settings pages list what was approved. Setup happens in the chat, at the moment it is needed, as a card. Policy (Settings, Approvals) can pre-approve a card kind: Always ask, Allow up to a limit, or Allow. Add credential always asks and can never be pre-approved, because a person types the value into a secure form that goes straight to the platform's credential store. Roles decide who may approve which card; a viewer approves nothing. The audit log lists tool calls that changed something and approvals. Settings pages read and revoke; they are not where setup begins. Two things are still set up in Settings: agent platform accounts (self-hosted) and members and sign-in. See [organization approvals](../organization/approvals.md).
- **Approval cards.** Seal contract, Raise budget, Add credential, Attach repo, Change environment, Open pull request. Defaults: Seal contract asks; Raise budget asks above a standing allowance per session; Attach repo asks; Change environment asks; Open pull request asks.
- **The judge.** A separate trait from the agent platform trait: run this command at these commits with these secrets and return these numbers. The trust rules never bend: only the judge's numbers count; an agent never holds the holdout credential; a sealed contract never changes; a person or an agent can never mark a win, only the acceptance criteria can, as one pure function in code; chat can change what an agent tries and never what counts. See [judge rules](../judge/rules.md).
- **The anti-cheat is built into the judge.** A sealed contract (a hash over the contract and its protected files at pinned commits; a trial that changes a protected path is Rejected). A separate judge sandbox for every measurement (fresh, no agent in it, the only place with the holdout credential, a network route only to the contract's data, removed afterwards). Seal checks before a contract can be sealed (the baseline, the noise check, the cheat checks). Confirm before Kept (measured again in a fresh judge sandbox and checked on the holdout, with a bar that rises with the number of trials tried). A second opinion from an agent of a different model provider than the proposer, which flags and never decides. A signed receipt. See [judge anti-cheat](../judge/anti-cheat.md).
- **Acceptance.** A trial is Kept when it beats the trial it started from (the baseline or a Kept trial) by more than the bar, every guardrail holds, and it is confirmed in a fresh judge sandbox and on the holdout. A contract's best is its best Kept trial. Outcomes: Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected, Failed.
- **What an agent sees.** The measured value and guardrail values on the evaluation data, and the outcome. It may read the evaluation's code, since that lives in a repo, and may never change it. It never reads the holdout or learns a holdout value. This replaces the earlier rule that an agent never reads the evaluation's code.
- **Delivery and watch.** `deliver` opens one pull request per repo changed, linked to each other, each carrying the receipt, and refuses any trial that is not Kept. The session stays attached: review comments and CI failures arrive in the chat, and a rebase is measured again before the receipt is updated. After delivery the contract can be watched; a regression (worse than the delivered value by more than the noise) wakes the session or starts a new one, and notifies.
- **Memory is `history`.** Each trial carries its hypothesis, so a later session reads what was tried and what failed, failures included. There is no separate notes or skill feature.
- **Sessions.** One agent per session. A session attaches any number of repos, and protected paths in a contract name their repo. Git is the hand-off: each session gets a short-lived git token per repo that pushes only to that session's own branch, and Ratchet stores commit ids, never diffs. A session may spawn others on the same contract and budget, on any agent and platform; a spawned session cannot spawn; a person can open any of them and talk to it. No agent's vendor is privileged, because only the judge decides. The agent drives: the only server-side state machines are the judge's queue and the budget meter. See [session rules](../session/rules.md).
- **Budget.** One budget per session and the sessions it spawns. Spend is model tokens, sandbox time and judge time. A standing allowance comes from policy, and `request_budget` raises it by a card. When it is spent the judge refuses `measure` and the session is Waiting for you with a Raise budget card. An organization has a monthly cap.
- **The sandbox is temporary, the conversation is not.** A sandbox ends when the session is idle or stopped, and resuming starts a new sandbox from the session's branches.
- **What the server stores.** The conversation (what people and agents said; see Open calls), tool call names and card states, commit ids, measurements, spend and audit rows. Never tool output, file contents, diffs or data. Tool output and raw logs stay on the platform, are fetched when someone opens them, and may expire; the session says so.
- **What agents are told is part of the product.** The session brief, the tool descriptions, how history is shown and when to stop live in one place, are versioned and are tested against a set of repos with known wins. See [session prompts](../session/prompts.md).
- **The organization floor is flat.** Members and roles (owner, editor, viewer; organization-wide), sign-in, access tokens and service accounts, approvals and policy, budgets, the audit log and notifications (a win, Waiting for you, a regression, a budget spent).
- **The screen.** A fixed 256px sidebar (brand, New session, Search, Sessions, Contracts, user menu) and a main area. New session is a composer with three chips (Repos, Agent, Environment), not a dialog or a form. A session page has a header, a scoreboard strip when it works on a contract, the conversation of messages and cards, and a composer a person can use at any time, including to interrupt. A card is the only place structured output appears. Contracts are a list page and a detail page; a win has a receipt page. Settings is a dialog with a rail. Page types: Session, ListPage, DetailPage, Dialog, SettingsSection, SignIn. See [design behaviour](../design/behaviour.md).
- **Clients.** The web app; the installed web app on Windows and Linux; the `ratchet` CLI, where `ratchet "cut checkout p95"` opens the same session in the terminal (target); `ratchet mcp`. The native macOS app stays ruled in; see Open calls.
- **Principles.** The ten governing principles are in [SKILL.md](SKILL.md).

## What this replaces

Directed by the owner, 16 Nov 2026. These were the earlier target design. They are removed, not renamed, except where a row says otherwise.

| Earlier | Now |
| --- | --- |
| Experiments, and runs as numbered executions | Sessions on a contract. A trial is one `measure` call. |
| Drafts, drafting sessions, the config branch, Approve | No drafts. The agent proposes a contract with `seal` and a person seals it from a card. |
| The stepper and its side wizard | Cards in the conversation. |
| Customize, Evaluations, Skills, Plugins, Connectors and Agents pages | Setup tools shown as cards; Settings lists and revokes. |
| Plugins | Cut. |
| Skill packs | Cut. `history` is the memory. |
| Notes, and Pause agent | Cut. A person talks to the agent. |
| Projects, and the dashboard page | Cut. Roles are organization-wide; Contracts holds Delivered, Pending and Return. |
| The three basics: optimization, context, infrastructure | Session, judge and organization floor. |
| The evaluator, the evaluator sandbox, the measuring platform | The judge, the judge sandbox, the judge platform. |
| Benchmark and evaluation template | The contract's evaluation, inside a sealed contract. |
| Agents in a run with an even budget share that passes on | One budget per session and the sessions it spawns. |
| A server loop that starts, steers and stops agents | The agent drives. |
| An agent never reads the evaluation's code | It may read it and may never change it. |

The agent loop's ideas survive in the judge: trials, Kept, confirmation on the holdout and agents that build on each other's best.

## Offerings, platforms and vendor facts (target)

Decided by the owner, 16 Nov 2026, unchanged by the redesign.

- **Two offerings.** Self-hosted: the Ratchet server is a Docker Compose service (server and Postgres) in the customer's infrastructure, using the agent platform accounts the customer already has, at least one. Ratchet cloud: we host the server and run every session on Ratchet's platform accounts, billed to the customer at cost through the budget. Both ship from day 0, and one core serves both.
- **Ratchet runs no sandboxes for agents. Agent platforms do.** An agent platform is a managed service that runs an agent and its sandbox as one durable session: Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service and Google Agent Runtime. Every platform sits behind one agent trait with one adapter each: start a session with repos, an agent, an environment, credentials and tools; send input; read its events; stop it and delete it. Platform words stay inside each adapter. Foundry Agent Service and Google Agent Runtime run a container image Ratchet supplies, so on these two Ratchet supplies one agent image per agent, with the agent's own harness inside it.
- **An agent is a model on a platform.** For example Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AgentCore, which runs one harness over many model providers, GitHub Copilot (Copilot SDK) on Microsoft Foundry Agent Service, and Gemini (ADK) on Google Agent Runtime.
- **Build order.** The traits are designed against all five platforms from the start. Adapters ship in this order: Claude Managed Agents, AWS AgentCore (which brings Gemini and Grok), the OpenAI Agents API, Microsoft Foundry Agent Service, Google Agent Runtime.
- **Where the judge sandbox runs.** On a platform that can run a command in a sandbox with no agent. AWS AgentCore can. Google Agent Runtime sandboxes run without an agent; whether one can hold a credential the agent never sees is checked in the spike. Claude Managed Agents, the OpenAI Agents API and Microsoft Foundry Agent Service cannot. Foundry has no sandbox that runs a command with no agent, and its Code Interpreter is a tool the agent calls, so the likely path there is a judge image deployed as a hosted agent that runs only our measuring code and no model.
- **`measure` is asynchronous so no platform limit applies.** On Foundry, a client tool is answered by returning a `function_call_output` with the call id, and a documented 10-minute expiry for tool outputs may apply. On Google Agent Runtime serving tools over the event stream is not documented, and bidirectional streaming is in preview with a 10-minute limit per query. Returning a ticket at once and delivering the measurement later as an event sidesteps both. The spike checks it.
- **Credentials reach a session only through the platform's credential store** (the Claude Managed Agents vault; AgentCore Identity; the agent's Microsoft Entra agent identity on Foundry Agent Service, with secrets reached through a Foundry connection to Azure Key Vault; the agent's Agent Identity or service account on Google Agent Runtime, with secrets read from Secret Manager; or the platform's equivalent). The exact stores on the last two are confirmed in the spike, as Agent Identity is partly in preview. Never a prompt, a file in the repo or Ratchet's database. Model access belongs to the platform account, so no model key enters a sandbox.
- **Connectors** an agent's code may read: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, Amazon S3 and any MCP server. The code host (GitHub, GitLab) is how repos attach and pull requests open. Each connector says how to make a read-only, holdout-excluding credential for its system. The customer registers it once, and it lives in the platform's credential store, never in Ratchet. Credentials minted per session may come later, connector by connector, and need a new ruling.
- **Ratchet cloud uses only Ratchet's platform accounts at launch.** An enterprise cannot bring its own platform accounts to Ratchet cloud yet, though the adapters support customer accounts because self-hosted needs them. An enterprise that needs data read in its own network self-hosts, for example with AgentCore in its own AWS account.
- **Timing is noisier on shared sandboxes.** Platform sandboxes run on shared hardware, so a timing contract's noise band is wider and fewer trials are Kept. The noise check and the bar handle it, and the contract page says so in one line.
- **No Helm chart and no OpenTofu at launch.** The self-hosted server ships as Docker Compose only. Ratchet cloud ships with the Cloudflare config and `cf`. Ratchet's own platform accounts are set up by hand, written down and owned by the owner. Either tool comes back only with a new ruling.
- **"This machine" is deferred.** Agents never run on a Mac until a later ruling, after the first platform ships. Measuring never runs on a laptop either way.

## Ratchet cloud (target)

Decided by the owner, 15 and 16 Nov 2026, unchanged by the redesign.

- **Storage.** Self-hosted uses Postgres 18. Ratchet cloud uses one SQLite database per organization, inside one Cloudflare Durable Object per organization, pinnable to the EU, the US or FedRAMP. Both ship from day 0. Moving between them is an export and import of one neutral file format, not a database copy.
- **One core, two shells.** `ratchet-core` holds all logic and the API routes and compiles natively and to WebAssembly. Storage sits behind one trait with two implementations. `ratchet-server` is the self-hosted shell; `ratchet-cloud` is the Cloudflare shell. A small shared directory holds sign-in routing and billing, never tenant data. See [stack](../stack/SKILL.md).
- **We manage everything.** Ratchet cloud runs the server on Cloudflare and every session on Ratchet's platform accounts. An organization signs in, attaches repos and starts; it runs no infrastructure of its own.
- **Ratchet's platform credentials** sit in Cloudflare's Secrets Store, bound to the Worker, never in an organization's database, a response or a log.
- **We are a data processor** for every Ratchet cloud organization: a data processing agreement, listed subprocessors (Cloudflare and the agent platforms in use: Anthropic, OpenAI, AWS, Microsoft, Google) and a stated retention rule. Ratchet keeps no tool output, file contents or diffs; each platform's own retention is stated beside it. The vendors promise only where data is stored at rest. Foundry Agent Service keeps data at rest in the geography of the chosen Azure resource; where the model processes it depends on the model deployment type (Standard and Regional stay in the geography, Data Zone stays in the US or EU zone, Global can be anywhere). Google Agent Runtime keeps data at rest in the location chosen (the US or EU multi-region, or a supported country); where it is processed is checked in the spike. Today the OpenAI Agents API offers data residency in the United States only and no zero data retention.

## Sign-in (target)

- **Sign-in is OpenID Connect, plus GitHub's OAuth.** Ratchet cloud offers GitHub (OAuth with its user API, since GitHub does not offer OpenID Connect for app sign-in), Google (OpenID Connect), and an organization's own OpenID Connect provider (single sign-on), which only an owner sets up. A self-hosted server uses the administrator's OpenID Connect provider. No passwords.

## Desktop clients (target)

Decided by the owner, 14 and 15 Nov 2026, unchanged by the redesign.

- **A native macOS app, SwiftUI, macOS only at launch.** It is a client of the server's API. It stays ruled in; its boards are removed for now (see Open calls).
- **On Windows and Linux the desktop app is the web app, installed as a progressive web app.** The same web client, installable from Chrome or Edge: its own window and taskbar icon, a badge for what needs attention, push notifications and an offline screen. On macOS the web app does not offer to install; it points to the Mac app.
- **Notifications are in the product:** a win, Waiting for you, a regression, a budget spent. Native on the Mac, web push elsewhere, ids only. See [organization notifications](../organization/notifications.md).
- **The device stores no data either.** The service worker caches the app's own files only, never API responses, diffs or logs. A push message carries ids only; the app fetches the rest when opened. A self-hosted server that cannot reach the browsers' push services sends no push, and everything else works.
- **Clients share behaviour, words and element ids, never pixels.** See [design behaviour](../design/behaviour.md).
- **The Mac app is a direct download, updated with Sparkle.** It is signed with a Developer ID and notarized, not on the Mac App Store. It checks one signed update feed, ours, daily by default; a person can turn checks off, and a company can turn them off for its Macs and deploy releases itself through device management, which is also how air-gapped installs update. See [stack/swift/ratchet.md](../stack/swift/ratchet.md).

## Still unproven: the spike

The spike comes before any product code. One tiny contract is measured end to end on all five platforms. It proves the agent trait, the Ratchet tools served to a session over its event stream (undocumented on Google, a 10-minute expiry on Foundry; asynchronous `measure` is designed to sidestep both) and a judge sandbox that holds a credential no agent sees.

## Open calls for the owner

Directed 16 Nov 2026 as drawn, not decided. Write options with a recommendation when one comes up. Do not pick one by building it.

| # | Call | Status | The design is drawn with | The alternative |
| --- | --- | --- | --- | --- |
| 1 | The server stores the conversation's words | OPEN | Yes, never tool output | Transcripts stay only on the platform, so chats expire |
| 2 | The customer's CI runner as a judge sandbox | OPEN | Not included; the judge trait leaves room for it | A new dependency |
| 3 | "Experiment" is retired as a word | OPEN | Session and contract | Keep "experiment" for the thing a user creates |
| 4 | Cut: plugins, skill packs, notes, Customize, projects, the dashboard page | OPEN | Cut | Keep any of them |
| 5 | The macOS app's boards | OPEN | Removed until the web direction is approved; the app stays ruled in | Keep drawing them now |
| 6 | Per-repo visibility of sessions | OPEN | Roles are organization-wide | Visibility follows repo access |
| 7 | How the bar rises with the number of trials | OPEN | Settled with the first code | Settle it now |
| 8 | The standing budget allowance per session | OPEN | Boards show $10 as an example | Another default |

Older open questions that still stand: whether scoring is offline only or production results are used too; whether Ratchet ever reshapes data (today no, see [non-goals](non-goals.md)); whether "Try a demo" stays in the web app; the licence (the workspace manifest says `UNLICENSED` today, and what it becomes is not decided).

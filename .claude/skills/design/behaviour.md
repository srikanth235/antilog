# Behaviour

How a screen behaves. The web app and the macOS app (target; its boards are owed) share behaviour, words and element ids, never pixels. Values live in `design/tokens.toml`; the boards in `design/mockups/` show the result and are listed in the index in [`design/mockups/README.md`](../../../design/mockups/README.md) (flow and checks: [mockups.md](mockups.md)). Exact labels are in [wording.md](wording.md). What an experiment and a session are, what the judge does and what a card asks are in [session](../session/SKILL.md), [judge](../judge/SKILL.md) and [approvals](../organization/approvals.md). Where a board and this page disagree about a screen, the light board wins and this page is fixed.

A person states a goal, an experiment starts, and a coding agent in a session talks to them while the judge's numbers are the only ones that count (target). There is no mode, no plan step and no setup screen: setup happens in the chat, as a card.

## Shell and sidebar

A fixed 256px sidebar and a main area.

- **Top** (`SidebarTop`): the brand, "New experiment", Search (a link to `SearchStart`, with the ⌘K hint), and the nav rows Projects and Customize.
- **Rows** (`SidebarRows`): Pinned first, then experiments under collapsible project headings. An experiment is one line: its title and one activity icon (what needs you). Under an experiment that is open or pinned its sessions nest one level deeper, one line each, named by its agent with its status icon. A row's activity icon updates live.
- **Foot** (`SidebarFoot`): the user menu: Settings, a light and dark switch, the host name, Sign out.
- `SidebarEmpty` is the same chrome with no rows, shown only when the organization has no experiment; Projects and Customize stay.

An experiment has a **state** (Not sealed, Sealed, Watching, Regressed) and an **activity** that comes from its sessions: Working (an open arc), Waiting for you (a half-filled circle), Done (a check in a circle), Failed (a cross in a circle). Each has a word and its own icon shape, never colour alone. The sidebar row shows the activity; the experiment page shows both. A session a person stopped is Done, with "Stopped by NAME".

An agent's message carries the Antilog mark and "Antilog", with the agent as a quiet second label. Agent logos appear only where an agent is the subject: spawned session cards, the agent menu, Customize rows, receipts and lineage.

## Page types

Every screen is one of six types. A new kind of thing under Customize does not make a seventh. The type owns the header, back, search, the empty, loading and error states and the actions; a screen that hand-rolls any of those is a finding.

| Page type | Anatomy | Examples |
| --- | --- | --- |
| Session | One slim header bar, then (once sealed) the scoreboard strip, then the conversation with the composer floating at the foot. No back (the sidebar is the way out); global search only. A card waits until a person decides; a failure is one calm line | An experiment in any activity; Home is the Session type with no conversation yet |
| ListPage | Title, a count, search, filter and sort where the list is long, one primary action. Rows, or cards where a board draws them (Discover), never both. No back at a sidebar root. Empty: one sentence and at most one action. Loading: a skeleton of the rows. Error: what happened and what to do, with Try again | Projects, each Customize tab, Waiting for you on Home, Search results |
| DetailPage | Back to the named parent, a title with its state, one primary action, a menu for the rest, then sections of rows and quiet tables. One layout for every item under Customize. An empty section says so in a sentence. Read-only: controls disabled with a one-line reason | Project, experiment, receipt, and every Customize item |
| Dialog | Over the app. A title bar with a close button, a dimmed backdrop; focus moves in and Tab stays inside. Esc, the close button or the backdrop close it and return focus to what opened it. A destructive confirm is inline or a small dialog with the verb as its button | Settings, Add platform, Add value model |
| SettingsSection | One section of the Settings dialog, a rail on the left (`SettingsRail`), rows not cards, read-only for non-owners | Profile through Audit log |
| SignIn | One centred panel on the page ground: the mark, one line of purpose, the provider buttons. No sidebar, no search. Errors sit under the buttons in one sentence | `SignIn`, `SignInError`, `SignInNoAccess` |

## The session page

The experiment page is the chat of its lead session: one agent in one sandbox on one agent platform, any number of repos attached. Spawned sessions appear as cards in the conversation and open as their own chat inside it (`SessionSpawned`), with the breadcrumb "Project / Experiment title / Codex session". A session has an activity and nothing else; the experiment owns the state, scoreboard and budget.

### Header and panels

One slim bar across the main area, like Claude Code's desktop header.

- **Left.** The breadcrumb (experiment title in bold), a chevron that opens the **title menu** (Rename, Pin or Unpin, Experiment details, Copy link, Hand off to..., Stop session, Archive), then the state and activity pill. Stop session confirms inline in the chat.
- **Right.** Icon toggles in order Details, Trials, Changes, Terminal, Preview, Files; a separator; More (Export conversation, Keyboard shortcuts, Report a problem); Share (icon and word). Each toggle has `aria-pressed`, an `aria-label` and a `title` with its shortcut (⌘⇧D, R, G, T, P, F).
- **One panel at a time.** A toggle opens its panel beside the chat; the pressed toggle or the panel's close button closes it; another toggle swaps it. About 440px wide, resizable by a drag handle (a separator that takes the arrow keys), with its own header. The chat column narrows and the scoreboard strip tightens. Details is open by default on wide screens; boards draw panels closed unless they are about that panel.

| Panel | Shows |
| --- | --- |
| **Details** | Sections of rows (icon, value, trailing action): Experiment (state, version, link to the detail page); Environment (a settings icon starts the Change environment card); Repos (each with its branch; "+" posts an Attach repo card); Agent; Budget (figure, bar, "Change budget"); Sessions (status, name, spend, each linking to it); Context (share of the window used, "Compact", "Fresh session with summary") |
| **Trials** | The scoreboard and every trial across sessions, newest first, filtered by session. A row opens its trial card, or the receipt for a Kept trial |
| **Changes** | The diff of this session's branch per repo against its base; a picker for "Trial 12, Kept" (default) or "Working copy"; files grouped by repo with +/- counts, the unified diff below. A path the sealed experiment protects carries a lock and "Protected: a change here is Rejected." The server stores no diff: the panel fetches it from the platform ([session model](../session/model.md)) |
| **Terminal** | A shell in this session's sandbox. Editors and owners type; a viewer reads, with a line saying so. A note says the judge never runs here. Opening it is recorded in the audit log |
| **Preview** | A browser on a sandbox port, with address bar, reload, open in a new tab. The button exists only when the environment exposes a port: absent, not disabled |
| **Files** | The attached repos' file tree in the sandbox, repo as the top level; the open file read-only with a path bar |

- **Share** (a popover, no panel): people in the project with their role marked as inherited; a switch for "Anyone in the organization with the link can view"; Copy link. Sharing never grants more than a person's project role. A viewer sees only Copy link.
- **Read-only** (`SessionReadOnly`): a viewer opens every panel but cannot type in Terminal; Rename, Pin, Stop session and Archive are disabled, with a one-line reason in the header.

### Scoreboard strip

Pinned under the header once the experiment is sealed; absent while Not sealed. It shows the metric from baseline to best with the change, trials, kept, spend of budget with "Change budget", and spend per kept trial (absent while nothing is Kept). An editor or owner changes the budget there at any time; lowering it below what is spent stops new measurements; a viewer sees no control. One small chart of the metric per trial shows the baseline and noise band, with one line per session when sessions were spawned. It shows only the judge's numbers, never one made in a session; a trial being measured is a hollow point. The strip scrolls with nothing; the conversation scrolls under it.

### The conversation

In time order: messages (short sentences; text streams, ending with a cursor bar until done), tool calls (one quiet line each; a chevron expands to what was asked and the first lines back), timeline lines for what happened (a seal, a handoff, a compaction, a failover), and cards. A card is the only place structured output appears; there is no step list and no wizard.

**Long tool output** is collapsed and never stored by Antilog. Opening it fetches it from the platform ("Fetching from PLATFORM"), or says the platform has dropped it, or says it cannot be reached, with Try again. Neither is drawn as an empty block.

### The composer

It follows Claude Code's: a rounded box with a soft shadow and a quiet row of settings. On a session it floats over the conversation with a fade above it and no band of its own. Home's start composer is large, because on Home it is the page.

- **Idle, one line.** "+", the text, four small icons (Agent, Environment, Mode, the budget ring), mic and send. It grows with the text to about eight lines, then scrolls inside. On focus or with text the options show in full as a quiet row under the box; idle they collapse to icons. Every icon has a label and a tooltip.
- **Grown** (`SessionComposerExpanded`): an attachment strip when something is attached (thumbnails with a remove x on hover), the text area, and a bottom row: "+" (attach files, images, repos or connectors) left, mic and send right. "/" lists commands. Send is a return arrow, quiet while empty, a filled accent circle with text.
- **Options under the box**, each a popover: Project and Repos (Home only; fixed once it runs), Agent, Environment, Mode, and the budget ring (fills with spend). **Mode** is the approval mode: Ask (every card asks) or Follow policy (cards that policy allows go through; policy still caps it, see [approvals](../organization/approvals.md)). "Auto" names only the agent choice, never the approval mode.
- **By status.** *Working:* a person types to steer and @-mentions a session of the experiment to message it; without a mention the text goes to the session being viewed. While the agent works send is a stop button ("Interrupt"), as in Claude Code; with text in the box send shows and the text is delivered before the next step. The agent reads steering as chat; it can change what it tries and never what counts. *Waiting for you:* the composer stays open and the waiting card is above it. *Done or Failed:* "Continue this session" starts a new sandbox from the session's branches; Failed also offers "Start again". *Read-only:* the box is greyed and nothing is active.
- **Agent menu** (`SessionAgentMenu`). A popover above the box: **Auto** first with one line of why, then each agent turned on with its logo, a cost tier ("$" to "$$$") and the measured figure per trial, the current one marked; then **Hand off to...**; a note that the judge measures the figures on the organization's code. A person can always pick a named agent and no agent is pre-selected by brand. Before the judge has data Auto uses the organization's default agent. Home shows "Auto". How Auto chooses is in [session](../session/SKILL.md).
- **Hand off** (`SessionHandOff`; also in the title menu). The session ends as Done with "Handed off to AGENT" and a new session starts in the same experiment on the same branch, opening with a summary card written from the conversation and `history` and a card linking back. Skill packs, evaluations and connectors work with every agent.
- **Context.** Antilog compacts at 80% and says so in a timeline line.
- **A new experiment** (`Home`, `HomeEmpty`): all six settings. Project defaults to the project the person is in (or last used) and lists those where they are an editor or owner. The budget starts from the project's default per experiment, falling back to the organization's. Send starts the experiment; nothing else is asked.

## Cards

A card is a frame: a title, what would happen, what it may cost, two buttons; every new kind reuses it. The preview comes before Approve, and nothing is created or spent until a person decides. A pre-approved card collapses at once to "Allowed by policy". The eight kinds and their policies are in [approvals](../organization/approvals.md); their fields and words are in [wording](wording.md#cards). What a card does:

- **Seal experiment** (`SessionSeal`): the definition (metric, evaluation command, protected paths, data and holdout, **skill packs** with the versions agents may read, guardrails) and the seal checks (baseline, noise, cheats), then the **Cost and value** group, open and part of the sealed definition: the project's value model for the metric with its source and setter, "Propose a different value" (needs a project owner's approval on the same card), the expected return, the budget, the quick check, patience and the cost estimate. With no value model it says "No value set" and wins show the metric only. An agent never sets or edits a value model.
- **Trial card**: the hypothesis, the judge's value and change, what it started from, files, guardrails, and its cost (the agent's spend since the session's previous trial plus the judge time; "so far" while measuring). A trial that stopped at the quick check is tagged "Quick check" with its lower cost; the quick check never decides Kept.
- **Win** (`SessionWin`): the receipt in short, what it is worth a year with the value model named, what the win cost including trials not kept, the second opinion, and "Open pull request". **Open pull request**: per repo the branch, title and files changed; "Stays Pending while open"; nothing merges until a person merges it. The pull request card follows with its states and review comments as lines (`SessionDelivered`).
- **Regression** (`SessionRegression`): the measured value against the one delivered, and the amount At risk, taken out of Delivered until a fix is Kept and merged; "Start session" starts the fix.
- **Raise budget** (`SessionBudget`), **Add credential** (`SessionCredential`), **Attach repo** (`SessionRepo`): the card appears the moment the agent needs it. Add credential is never pre-approved: a person types each value into a secure field, never in the chat.
- **Keep going?** (`SessionKeepGoing`): when the sealed patience number of trials in a row across all sessions are not Kept, the experiment is Waiting for you with what was tried ("Copy as a note"), "Keep going" and "Stop here". Approvals policy can set it to Allow or Always ask.
- **Continue on another agent?** (`SessionFailover`): when an agent platform is unavailable or rate-limited, the session is Waiting for you with the agent Auto would choose and why, what the new session starts with, the policy, and "Continue on AGENT", "Choose another agent" and "Wait and try again". Policy may allow it without asking; a timeline line records it.
- **No win** (`SessionNoWin`) and **Failed** (`SessionFailed`) are calm lines, not cards: what was tried and nothing to deliver; where it failed and how to start again.

## Experiment page and receipt

**Experiment detail** (`ExperimentDetail`, a DetailPage), opened from "Details" in the session header. Back to its project. The title with state, version and activity; "New version" and "Start session". A strip: Best, Trials, Spent of budget with "Change budget", Per kept trial, and Delivered, Pending and Return (only when the experiment states a value per unit). Sections, in order:

1. **Definition**: metric and direction, evaluation command, protected paths per repo, guardrails, repeats, quick check, patience, value model, acceptance criteria, judge platform. Immutable: a sealed experiment is never edited; "New version" makes one, and numbers never carry across versions.
2. **Seal checks**: baseline, noise and cheat checks as they were when sealed.
3. **Data**: the evaluation data and the holdout with dataset versions, connector and marking (the holdout "Restricted to the judge"); then the skill pack versions read and the evaluation template it started from, each linking to Customize.
4. **Value**: the sealed value model, what it is worth, cost so far, Return, and how At risk works.
5. **Scoreboard**: the chart of every trial with one line per session, and a table of trials, kept, best and spend per session.
6. **Where the spend went**: Kept trials, Within noise, Worse, Guardrail failed, Rejected, Failed and Not measured as dollars and shares with a stacked bar to scale, worked out from spend records and never stored. Settings, Budgets shows the same for the organization.
7. **Sessions**: a quiet table (session with logo, platform, started, ended, spend, state, clean-up). A session still working shows "Running" and "When it ends", never an empty cell. A session not removed adds a banner above the table.
8. **Trials**: every trial across sessions, failures included, with hypothesis, session, outcome, value, cost and what it started from; quick-check trials are marked. A row opens that session's chat at that trial.
9. **Wins** with their pull requests, each linking to its receipt. 10. **Watch**: the switch, its triggers (a schedule, the default branch moving) and the last measurement. 11. **Versions**: each with its seal date and state.

**Receipt** (`Receipt`, a DetailPage), one win. The header has the trial, the signature and "Verify again", then the numbers: baseline, what it started from, this trial and the change. Groups: Signature checks; Experiment (version and seal, value model, worth, cost and Return of the win); Commits per repo with pull requests; Data (dataset versions read, repo commits, the credentials used with scope: agents read-only with the holdout excluded, the judge its own); Compute (judge platform and environment, judge image and digest, agents and sessions with platforms, a fresh judge sandbox per measurement); Values; Repeats and noise; Trials tried; Holdout (Passed or Failed, never a value); Second opinion; Spend; and Lineage, the chain from the baseline to the win with each node a link to its trial.

There is no experiments list page and no "All experiments" link: a project lists its experiments, and search has an Experiments scope.

## Home

New experiment is a composer on the home page, not a dialog or a form (`Home`): "What do you want to improve?", the large composer, then:

- **Ideas**: the top three across the person's projects (Antilog suggests, a person starts), each with the experiment, project, metric, expected value a year, expected cost and the ratio, sorted by it. "Start" opens the composer filled in. With none there are no Ideas, never a canned feature tour.
- **Waiting for you**: a ListPage section of experiments that need a person, each with the card it waits on and its age; a row opens the experiment with the card in view. With none the section is absent, not an empty box.

Sending starts an experiment with its lead session, Not sealed until a person seals it; it may stay Not sealed. With no experiments (`HomeEmpty`) the page is the composer and "No experiments yet.", with `SidebarEmpty`.

## Projects and Report

A project groups the experiments for one area of the code; every experiment belongs to exactly one, and New experiment asks for it with the Project chip.

**Projects** (`Projects`, a ListPage, from the sidebar row). Every project the person can see, with description, their role (Owner, Editor, Viewer), experiments, wins and spend this month; a person who is not an organization owner sees only their projects. Primary action "New project", with "Report" beside it. A strip at the top shows Delivered, Pending, Spent ("includes $X on experiments with no win") and Return (Delivered over all spend in scope). A win is Delivered once its pull request is merged; Pending, while a pull request is open, is not in Return. The strip is absent, not zero, when no experiment states a value per unit. There is no dashboard page. Empty: "No projects yet." with "New project"; a brand-new organization sees `HomeEmpty`.

**Project page** (`Project`, a DetailPage). Back to Projects; title, the person's role, counts, description, "New experiment" (opens the composer with this project chosen). Sections:

- **Experiments**: a table (experiment with repos, baseline to best, state, activity, spent of budget). Empty (`ProjectEmpty`): "No experiments in this project yet." with "New experiment".
- **Ideas**: experiments Antilog suggests from the repos, connectors and past experiments, ranked by expected value (from the project's value models) over expected cost (from similar experiments), each with "Start". An idea on a metric with no value model shows "No value set" and sorts last.
- **Value** (`AddValueModel`): value models per metric with source, setter and use count. A project owner adds one per metric (a dialog: metric, one unit, dollars a year, source, optional link; recorded in the audit log). A metric without one shows "No value set" and its wins count in no dollar figure. Agents never set or edit one.
- **Connectors used**: what this project's experiments read, with status; a name opens `ConnectorDetail`.
- **Budget**: spent this month of the project's cap, and the default budget per experiment (falling back to the organization's). Project owners change both; neither passes the organization's cap.
- **Members**: each member's role in this project; a role set for the project is marked, otherwise the organization role shows as the default. Project owners add and remove members and set roles; a project keeps at least one owner.

A viewer sees the same page with controls disabled and a one-line reason.

**Report** (`Report`, from "Report"). The monthly report for the organization or, with the project picker, one project: month picker, who it is emailed to, "Export PDF", "Export CSV", "Email this monthly" (the three buttons and the recipients are target; the page is built). Where the spend went is by kind and by what became of each experiment (with a win, sealed with no win, not sealed): spend has no trial yet, so the per-outcome table of the board is target. A strip (Spent, Delivered, Pending, At risk, Return), then "Where the spend went", the month's "Agent scoreboard", "Top wins" with "View receipt", and "Experiments stopped with no win" (their spend counts in Return). Owners and editors read it; a project's viewers read that project's.

## Customize

A sidebar page with tabs **Evaluations, Skills, Connectors, Agents and Plugins**. Each tab is a ListPage with a **Yours** and a **Discover** toggle (links, so each is a URL). Yours lists what the organization has. Discover lists what can be added in shelves of plain tiles (Featured, For you, New, each with "Show all"; connectors show a featured card and the grid of systems with their logos). Each tab's primary action is "Add evaluation", "Add skill", "Add connector", "Add agent" or "Add plugin". A row opens the one shared DetailPage. Customize is a library of reusable things; setting one up from a card in a chat adds it here too. Owners and editors add and change items; a viewer reads, with controls disabled and a one-line reason.

- **Evaluations**: reusable evaluation templates. A row shows name, kind (Speed, Quality, Size or Cost), the latest seal check result and use count. The detail page shows the template (metric, command, protected paths, repeats, environment, guardrails, the standard planted cheats for the kind), the seal checks with "Check again", and the experiments using it. A sealed experiment starts from a copy, so editing a template never changes one.
- **Skills**: versioned skill packs, written know-how agents read. A row shows name, source (Your team, By Antilog), version and use count. The detail page has an On and Off switch, the versions with what changed, which version agents get (latest or pinned), the source, which agents may read it, the experiments that read each version, and the line that the judge never reads a skill pack, so it cannot change how a result is measured
- **Connectors**: the data systems and MCP servers an agent's code may read. A cloud account is not a connector and neither is the code host (repos stay under Settings, Connections).
  - *List*: a quiet table; each row has the logo, name and target ("Snowflake · ANALYTICS"), a one-line meaning, status (Connected, Reconnect, Not checked) as word and icon, when last checked, and use count.
  - *Detail*: "How access is scoped": credentials are registered once and kept in the platform's credential store, never by Antilog; agents get a read-only credential on the views listed, holdout excluded; the judge gets its own read-only credential that also covers the holdout, kept on the judge platform; both short-lived and minted per session. A **Datasets** table (name with size, version, marking, Agents "Can read" or "No access", Judge "Can read"); the holdout row is "Restricted to the judge" with no grant control. Actions: Check connection, Update credential, Remove. A holdout opens `DatasetHoldout`: its versions by date (a version never changes), "Marked Holdout. Enforced by the credentials: only the judge's includes it.", who can read it, the experiments using it.
  - *Add* (`AddConnector`, `AddConnectorMcp`): the per-system fields, an agent credential and a judge credential with a short recipe each, "What each one gets", and secure fields whose values go straight to the platform's credential store.
  - *Error* (`ConnectorError`): what could not be read and what to do, "Check again" and "Update credential". Datasets show "Not checked", never empty, and the page names the experiments that cannot measure until then.
- **Agents**: Yours is the **agent scoreboard**. A row per agent: logo, platform, cost tier with the measured figure per trial, experiments, trials, kept, wins delivered, cost per win, median time to a win, Delivered, and a switch for whether Auto may use it. Filters: evaluation kind, project, range. Note: "Measured by the judge on your code. Not a vendor benchmark." A control sets the organization's default agent, which Auto uses until there is data. A row opens `AgentDetail`: platform, account, model, parallel limit, "What the agent holds" (a short-lived git token for its own branch per repo, read-only connector credentials that leave out the holdout, no model key), the same figures per evaluation kind, and recent trials. There is no agent order, no Pause agent and no per-agent budget share. Platform accounts stay in Settings, Agent platforms, which links here. Self-hosted, owners manage agents and others read; on Antilog cloud the list is read-only.
- **Plugins**: a bundle that adds evaluations, skill packs, suggested connectors and recommended agents, each under its own kind. A row shows what it adds and its status; the detail page lists the items by kind, each linking to its own page. A plugin has no powers of its own: it can never mark, unmark or grant access to a dataset, and its page says so. Suggested connectors stay off and recommended agents are not switched on until added. Removing a plugin asks in a small dialog whether to keep its items; an item in use by an experiment is never removed.

The holdout rule holds on every Customize page: a skill pack, plugin or connector never changes it.

## Global search

A centred palette over the app, opened from the sidebar's Search row, Cmd/Ctrl+K or "/". The field is focused. Scopes: All, Projects, Experiments, Sessions, Trials, Datasets, Customize, People; Left and Right change scope.

- **No query** (`SearchStart`): Needs attention (what waits for you, a connector that needs reconnecting), Recents (last experiments and connectors opened) and Actions (New experiment, New project, Add connector, Open Settings). Typing replaces it with results.
- **With a query** (`Search`): results grouped by kind in scope order, the match highlighted. A project shows its counts; an experiment its state and baseline to best; a session its state, agent and repos; a trial its number, outcome and experiment; a dataset its connector and, for a holdout, "Restricted to the judge"; Customize rows say what they are and the group has "Show all N". Each result opens the exact place (a session opens its chat inside the experiment; a trial its receipt or its place in the session). No match: `SearchEmpty`.
- **Footer keys**, only keys that work: Close Esc, Move Up and Down, Change scope Left and Right, Actions Option+Enter (only where a result has a menu), Open Enter.

## Settings

A large centred dialog, not a page, opened from the user menu or a Settings result or action in search. A left rail (`SettingsRail`) has a search field (the first Esc clears it) and two groups with a hairline between: **You** (Profile, Appearance, Access tokens, This device) and **Organization** (General, Members, Single sign-on, Agent platforms, Approvals, Budgets, Connections, Audit log). It reopens on the section last used; the search filters sections and rows by title, description and keywords. Below 720px the dialog is full-screen and the rail becomes a search field with a select.

Settings are for reading and revoking what was approved; setup does not begin here. Only two things are set up here, because no experiment can start without them: agent platforms (self-hosted) and members with sign-in. Everything reusable is set up once in Customize.

| Section | Shows |
| --- | --- |
| Profile | Name, email, role, sign-in provider, Sign out |
| Appearance | Theme: System, Light or Dark |
| Access tokens | Your tokens (kind, what each can do, last used, expiry, Revoke) and "Create access token"; a new token is shown once |
| This device | Install app, notifications for this device; installed web app only where it applies |
| General | Organization name, deployment (Antilog cloud or Self-hosted), server address, data location |
| Members | Users and service principals: how each signs in and their organization role (Owner, Editor, Viewer), their default in every project. Invite, change role, remove. At least one owner always remains. A project's own roles are on the project page |
| Single sign-on | The sign-in methods: GitHub and Google (each a switch), and, on a self-hosted server only, "Your OpenID Connect provider" (single sign-on), which only an owner sets up (provider, client ID, client secret reference, group mapping, status); how long a sign-in lasts. No passwords. Antilog cloud has no provider row (owner's call, 17 Nov 2026), so there the section shows only GitHub and Google. The boards show the self-hosted view |
| Agent platforms | The five platforms with columns Platform, Account and Status (platform accounts only; agents are in Customize) (Connected, or Needs attention when a credential is rejected), one badged "Judge platform"; Test connection, Update credential, Remove, "Add platform"; a link to Customize, Agents. Self-hosted, owners manage and others read. On Antilog cloud (`SettingsPlatformsManaged`) it is read-only for everyone: Antilog manages the accounts and bills at cost through each experiment's budget, and the section shows where data is kept and retention per platform. A rejected credential (`PlatformError`) names the sessions that cannot start |
| Approvals | One row per card kind (eight) with its policy: Always ask, Allow up to a limit, or Allow. Add credential always asks and offers no choice; Keep going? offers Always ask or Allow. Only owners change policy |
| Budgets | The default budget per experiment, the monthly cap and spent this month; experiments with budget and spend; an experiment's sessions with their share; "Where the spend went" for the organization; "Cost per win by project" with Return. No budget passes its project's cap or the monthly cap. Organization owners change the default and cap; project owners a project's cap and default (on the project page); editors one experiment's budget |
| Connections | What was approved in chats: Repos, Credentials (names and where kept, never values) and Environments, each with who added it, when, how many sessions use it, and Revoke. A link to Customize, Connectors |
| Audit log | The tool calls that changed something and the approvals: who, what, when, outcome, with an "Approvals only" filter. Never empty: signing in is the first entry |

- **A setting is a row**: title and a one-line muted description on the left, the control on the right, a hairline between rows. A sub-setting is indented and disabled while its parent is off. A list inside Settings is a quiet table with a tag for a kind and an icon and word for a status. No cards inside cards.
- **Read-only for non-owners, not hidden** (`SettingsReadOnly`): editors and viewers see the same section with controls disabled, a one-line reason and a lock beside the section in the rail. Revoking a connection is an owner action.

## Rules for every page

- **One set of page states**: loading, empty, failed, refused, partial, not checked. Each says what happened and what to do. Failure is never drawn as empty or zero, and something not checked is never drawn as healthy.
- **The holdout is marked.** It always shows as "Restricted to the judge". An agent never holds the holdout credential and never sees a holdout value, only "Kept" or "Failed holdout". No control grants it to an agent: the control is absent, not disabled. The copy says it is enforced by the credentials. A dataset version never changes, so the version on a receipt is the one read. Chat never changes it. (target)
- **Setup happens in the chat, as a card**, at the moment the agent needs a repo, credential, environment, budget or seal.
- **A card shows what would happen and what it may cost before Approve.** A destructive step such as Stop session confirms inline. Nothing is asked when policy already allows it; Add credential is never pre-approved.
- **Only the judge's numbers are shown as results.** A number made in a session is never drawn as one.
- **A control says exactly what it does** ("Seal experiment", not "Approve"). A control that does nothing is deleted. A footer or menu lists only keys and actions that work.
- **Status is never colour alone.** Every status and outcome has a word; activities and experiment states have their own icon shapes. Trial outcomes are words in a tag: Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected, Failed.
- **Same in every client.** If the web app, the macOS app, the CLI and MCP disagree about state or words, one is wrong.
- **Reuse the patterns.** One detail layout, one popover, one palette, one dialog. Menus and popovers close on an outside click. Every menu item is a `.mi`, every select uses `.selw`, every control has a disabled style.

### Empty states

One sentence and at most one action, no feature explanation. A list gets its own board only because the screen looks different. A count that is zero stays on screen (`$0`, `0 kept`).

| Screen | Empty when | Sentence | The one action | Board |
| --- | --- | --- | --- | --- |
| Home | no experiment | No experiments yet. | none; the composer is the action | `HomeEmpty` |
| Sidebar | no experiment | Experiments you start appear here. | none | `SidebarEmpty` |
| Search | no match | No results for “query”. | none | `SearchEmpty` |
| Settings, Access tokens | none made | No access tokens yet. | Create access token | `SettingsTokensEmpty` |
| Settings, Agent platforms | self-hosted, none added | No agent platform is added yet. | Add platform | `SettingsPlatformsEmpty` |
| Projects | no project | No projects yet. | New project | none; `HomeEmpty` covers a new organization |
| Project | no experiment | No experiments in this project yet. | New experiment | `ProjectEmpty` |
| Customize, Evaluations | none added | No evaluations yet. | Add evaluation | `EvaluationsEmpty` |
| Customize, Skills | none added | No skills yet. | Add skill | `SkillsEmpty` |
| Customize, Connectors | none added | No connectors yet. | Add connector | `ConnectorsEmpty` |
| Customize, Agents | none added | No agents yet. | Add agent | `AgentsEmpty` |
| Customize, Plugins | none installed | No plugins installed. | Add plugin | `PluginsEmpty` |

Never empty, so no board: Members, Audit log, Approvals, Budgets, General, Profile, Appearance and Single sign-on, and an experiment (it starts with its lead session). An experiment with no trials or wins says "No trials yet." or "No wins yet." inside its section. Connections before the first approval says "Nothing approved yet." A connector whose datasets cannot be read shows "Not checked" (`ConnectorError`), never an empty table.

## Colour and type

Midnight Brass; values are in `design/tokens.toml`.

- Brass is the brand and the only accent. People do not choose a colour.
- Text meets 4.5:1 in both modes. A control's outline meets 3:1, so a control uses `line2`, not the `line` divider.
- Light and dark are both designed and checked; a board is drawn light and its dark twin generated.
- Areas keep one colour each in diagrams and navigation: the judge is brass, experiments and sessions blue, the organization slate (target; not a token until added to `tokens.toml` with a contrast check). Green is the success state only (Kept, Done, Connected), never an area. Never rely on colour alone to say which area a thing belongs to.
- Tokens are named by meaning (`canvas`, `raised`, `line2`, `ink`, `accent`), and light mode redefines the same names. Never write a raw colour, radius, size or duration in a screen or a board. Type comes from the type classes, never an inline size.

## Clients

The clients are the web app, the web app installed on Windows and Linux, the macOS app (target), the `antilog` CLI and `antilog mcp`. `antilog "cut checkout p95"` starts the same experiment in the terminal (target); the CLI and MCP print the same words and cards as text.

The macOS app is SwiftUI and macOS only; on Windows and Linux the installed web app is the desktop app. Its boards are owed once the web direction is approved; its window chrome, menu bar item, native Settings window, notifications and offline banner follow this page. It is only a client of the server's API and runs no agents and no sandboxes.

The installed web app (`WebInstalled`, `WebNotification`, `WebOffline`) has its own window and taskbar icon, with no browser toolbar. Install is offered once, quietly, in Settings, This device ("Install app"; on macOS "Get the Mac app"), never as a banner over work. A badge on the icon counts experiments that are Waiting for you. Push notifications (a win, Waiting for you, a regression, a budget spent) are asked for at the moment they help, never on first load, and turned off in Settings, This device. When the server cannot be reached an offline screen shows what was last known, marked as not current, with Try again; nothing is cached beyond the app itself.

## Accessibility

Targets, labels and focus are checked on the boards in both light and dark. Every control is reachable by keyboard, the focus ring uses a token, and every icon-only control has a label. A streaming message is announced when it ends, not word by word; a card that needs a person is announced when it appears.

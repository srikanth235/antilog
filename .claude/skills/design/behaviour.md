# Behaviour

How a screen behaves. The web app and the macOS app (target; its boards are owed) share behaviour, words and element ids, never pixels. Values are not here: they live in `design/tokens.toml`, and the boards in `design/mockups/` show the result (see [mockups.md](mockups.md)). The words are in [wording.md](wording.md). What an experiment and a session are, what the judge does and what a card asks is in [session](../session/SKILL.md), [judge](../judge/SKILL.md) and [organization](../organization/approvals.md).

## The app is a chat and a judge (target)

The screen follows the product: a person states a goal, an experiment starts, and a coding agent in a session talks to them while the judge's numbers are the only ones that count. There is no mode, no plan step and no separate setup screen. Setup happens in the chat, as a card. The app lists experiments, never sessions; a session is one agent's work inside an experiment and is named on screen by its agent ("Claude Code session").

**Frame.** A fixed 256px sidebar and a main area. Sidebar top (`SidebarTop`): the brand, "New experiment" and Search. Then `SidebarRows`: Pinned first, then Experiments, one line each with a title and one activity icon (what needs you); and an "All experiments" link to the list page. The user menu at the foot (`SidebarFoot`) has Settings, a quick light and dark switch, the host name and Sign out. `Sidebar` is the three together; `SidebarEmpty` is the same chrome with no rows.

An experiment has a **state**, one word and one icon each: Not sealed, Sealed, Watching, Regressed. It also has an **activity**, which comes from its sessions: Working (an open arc), Waiting for you (a half-filled circle), Done (a check in a circle), Failed (a cross in a circle). Each has a word and its own icon shape, never colour alone. The sidebar row shows the activity icon; the experiment page shows both. A session a person stopped is Done, with the line "Stopped by NAME".

An agent's message in the chat carries the Ratchet mark and the name "Ratchet", with the agent that did the work as a quiet second label ("Ratchet · Claude Code", "Ratchet · Codex" in a spawned session). Agent logos stay where an agent is the subject: spawned session cards, the Agent chip, settings rows, receipts and lineage.

## Page types

Every screen is one of six page types. The type owns the header, back, search, the empty, loading and error states and the actions; the screen supplies content and words. A screen that hand-rolls any of those is a finding, not a variant.

| Page type | Anatomy | Examples |
| --- | --- | --- |
| Session | The experiment page, and a spawned session's chat inside it. Header: title, state and activity, a Details link, Stop with an inline confirm, Pin, menu. Under it, once the experiment is sealed, the pinned scoreboard strip. Then the conversation: messages, quiet tool lines, cards. A composer docks at the foot and follows the status. Back: none, the sidebar is the way out. Search: global only. States: a card waits until a person decides; a failure is one calm line, never an empty page. Boards: `SessionStart`, `SessionSeal`, `SessionWorking`, `SessionSpawned`, `SessionWin`, `SessionDelivered`, `SessionNoWin`, `SessionBudget`, `SessionCredential`, `SessionRepo`, `SessionRegression`, `SessionFailed`, `SessionReadOnly` | An experiment, in any activity; Home is the Session page type with no conversation yet |
| ListPage | Title, a count, search, filter and sort where the list is long, one primary action where one exists. Rows, never cards. Back: none at a sidebar root. Empty: one sentence and at most one action. Loading: a skeleton of the rows. Error: what happened and what to do, with Try again | Experiments, the Waiting for you list on Home, Search results |
| DetailPage | Back to the named parent, a title with its state, one primary action, a menu for the rest, then sections of settings rows and quiet tables. Empty: a section with nothing to show says so in a sentence. Read-only: controls disabled with a one-line reason | An experiment (`ExperimentDetail`), a receipt (`Receipt`) |
| Dialog | Over the app, so you keep your place. A title bar with a close button, a dimmed backdrop, focus moves in and Tab stays inside. Esc, the close button or a click on the backdrop close it and return focus to what opened it. Destructive confirms are inline in the page or a small dialog with the verb as its button | Settings, Add platform |
| SettingsSection | One section of the Settings dialog: a rail on the left with a search field (`SettingsRail`), the section on the right. Rows, not cards. See "A setting is a row". Read-only for non-owners | Profile, Appearance, Access tokens, This device, General, Members, Single sign-on, Agent platforms, Approvals, Budgets, Connections, Audit log |
| SignIn | One centred panel on the page ground: the mark, one line of purpose, the provider buttons, nothing else. No sidebar, no search. Errors sit under the buttons in one sentence | Sign-in on Ratchet cloud and on a self-hosted server (`SignIn`, `SignInError`, `SignInNoAccess`) |

### Session (the experiment page)

The experiment page is the chat of its lead session, one agent in one sandbox on one agent platform with any number of repos attached, under the experiment's header and scoreboard strip. Sessions spawned in the experiment appear as cards in the conversation and open as their own chat inside it, with the breadcrumb "Experiment title / Codex session". A session has an activity and nothing else; the experiment owns the state, the scoreboard and the budget.

**Header.** The experiment's title (the agent names it after the first message; a person can rename it), its state and activity with icon and word, a "Details" link to the experiment detail page, Stop, Pin and a menu (Rename, Copy link, Delete experiment). Stop confirms inline: the button becomes "Stop session" and "Keep working", and stops the session being viewed. A stopped session is Done with "Stopped by NAME". A viewer sees the header with Stop and Pin disabled and a one-line reason.

**Scoreboard strip.** Pinned under the header once the experiment is sealed; absent while it is Not sealed. It shows the metric from baseline to best ("1,310 ms to 1,102 ms"), trials, kept, and spend of budget ("$42 of $100") with "Change budget" beside it. An editor or owner changes the budget there at any time; lowering it below what is spent stops new measurements. A viewer sees the figure and no control. Beside the numbers is one small chart of the metric per trial, with the baseline and the noise band. When sessions were spawned the chart draws one line per session, each named by its agent, so who is improving is visible at a glance. The chart never shows a number made in a session, only the judge's. A trial being measured is a hollow point. The strip scrolls with nothing; the conversation scrolls under it.

**The conversation.** In order of time:

- Messages from people and from the agent, in short sentences. Text streams as it is written; the agent's message ends with a cursor bar until done.
- Tool calls shown as one quiet line each ("Read 4 files", "Asked the judge to measure"). A chevron expands the line to what was asked and the first lines of what came back.
- **Long tool output** is collapsed and never stored by Ratchet. Opening it fetches it from the platform: the row shows "Fetching from PLATFORM", then the output in a scrolling code block. When the platform has dropped it the row says "This output has expired on PLATFORM." When the platform cannot be reached it says "Can't reach PLATFORM. Output stays there; try again when it is back." with Try again. Neither is drawn as an empty block.
- Cards. A card is the only place structured output appears. There is no step list and no side wizard.

**The composer.** A text box at the foot, and it follows the status.

- A new experiment (`SessionStart`, and Home): the text box ("What do you want to improve?") and four chips, Repos, Agent, Environment and Budget. Each chip opens a popover; Budget shows the organization's default and a person can change it. Nothing else is asked.
- Working: a person can type to steer, and Send delivers the text to the agent before its next step. "Interrupt" stops the agent's current step and delivers the text at once. "Stop session" ends the session. The agent reads steering as chat; it can change what it tries and never what counts.
- Waiting for you: the composer stays open for an answer, and the card that waits is above it.
- Done and Failed: "Continue this session" starts a new sandbox from the session's branches. Failed also offers "Start again" in the failure line.
- Read-only (`SessionReadOnly`): a viewer reads. The composer is disabled with "Viewers read experiments. Only editors and owners can send."

**Sandbox.** The sandbox is temporary, the conversation is not. When it ends while idle the page says so once, in one line, and sending a message starts a new sandbox from the session's branches.

### Cards

A card sits in the conversation at the moment it is needed. An approval card has a title, what would happen, what it may cost, and two buttons, the approve verb and Decline. Before Approve the card says "Nothing happens until you approve." Nothing is asked when policy already allows it: the card appears already collapsed as "Allowed by policy".

**Collapsed.** An approved or declined card collapses to one line: who, when, and what ("Experiment sealed by Maya Chen · 14:02", "Declined by Maya Chen · 14:02"). The line expands to the card as it was decided, with its buttons gone. A card decided in another client shows "Approved by NAME in another client." A viewer sees an approval card with its buttons disabled and "Only editors and owners can approve."

**The six approval cards.**

| Card | Fields, in order | Actions | Board |
| --- | --- | --- | --- |
| Seal experiment | Metric and its direction; Evaluation command; Protected paths, grouped per repo; Data and holdout (the holdout marked "Restricted to the judge"); Guardrails; Baseline (the judge's measured value); Noise check (for example "±18 ms across 3 repeats"); Cheat checks ("3 of 3 rejected"); Judge platform and environment; Budget, a row with the amount and a control to change it (editors and owners). An experiment that failed a seal check shows the failed line and no Seal experiment button | "Seal experiment" (not "Approve"), "Decline" | `SessionSeal` |
| Raise budget | Now; Requested; What it may cost; Why the agent asks | "Raise budget", "Decline" | `SessionBudget` |
| Add credential | A secure form: Name, Platform, Value, Scope. The note "Typed here, never in the chat. Goes straight to PLATFORM's credential store." The agent never sees the value. Never pre-approved | "Add credential", "Decline" | `SessionCredential` |
| Attach repo | Repo, Branch, "Pushes only to the session's own branch" | "Attach repo", "Decline" | `SessionRepo` |
| Change environment | Image, Setup script, Size, Network, and a one-line diff of what changes | "Change environment", "Decline" | `SessionStart` (the chip), and the card in `SessionWorking` |
| Open pull request | Repos and branches, the receipt in short, the second opinion line, what it costs (nothing) | "Open pull request", "Decline" | `SessionWin` |

When the experiment's budget is spent the judge refuses `measure` and the experiment is Waiting for you with a Raise budget card (`SessionBudget`). A person can also change the budget without waiting for the agent to ask.

**The result cards.**

| Card | What it shows | Action | Board |
| --- | --- | --- | --- |
| Trial | The hypothesis in the agent's words; the outcome tag (Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected, Failed); value against what it started from and the change in percent; what it started from ("Started from trial 11"); commits and files changed, per repo; guardrail values. A trial being measured shows Measuring and a line that the result arrives here. Holdout shows only as an outcome, never a value. A Rejected trial says "Changed a protected path" | Expand | `SessionWorking`, `SessionNoWin` |
| Spawned session | Agent (its logo), platform, status, its best so far | Open (opens that session as its own chat inside the experiment, a person can talk to it) | `SessionWorking`, `SessionSpawned` |
| Win | The receipt in short (experiment version, repeats, noise, trials tried, holdout passed, spend), the second opinion line, "Open pull request". A second opinion flag is shown, never hidden | "Open pull request" (asks, by policy), "View receipt" | `SessionWin` |
| Pull request | One row per repo, the state (Open, Checks running, Checks failed, Approved, Merged, Closed), and review comments and CI failures arriving as lines under it; a rebase says it is being measured again before the receipt is updated | Open on the host | `SessionDelivered` |
| Regression | The experiment, the value now against the delivered value, the noise band | "Start session" | `SessionRegression` |
| Failure | One calm line: what happened and what to do ("The session failed on PLATFORM. Open the platform's log, or start again.") | "Start again" | `SessionFailed` |

A card for an outcome of no win (`SessionNoWin`) is a Trial card with its tag and the scoreboard unchanged; there is no separate "lost" card.

### Home and New experiment

New experiment is a composer on the home page, not a dialog and not a form. Home (`Home`) shows one text box ("What do you want to improve?") and four chips: Repos, Agent, Environment, Budget. Sending starts an experiment with its lead session, Not sealed until a person seals it from the Seal experiment card; it may stay Not sealed and end there. Nothing else is asked before the first message. Below the box:

- **Waiting for you.** The experiments that need a person, a ListPage section: each row has the title, the card it waits on ("Raise budget") and its age; it opens the experiment with the card in view. Empty: the section is absent, not an empty box.
- **Suggestions** under the box come from the attached repos' experiments ("Cut checkout p95 latency" where one exists). A suggestion fills the text box. With none there are no suggestions: never a canned feature tour.

With no experiments at all (`HomeEmpty`) the page is the composer alone and the sidebar is `SidebarEmpty`. A sidebar row's activity icon updates live.

When an experiment is sealed and the budget allows, nothing asks anything: a person types "cut checkout p95" and the agent starts.

### Experiments

**List** (`Experiments`, a ListPage, reached from "All experiments"). Each experiment: its metric, baseline to best, state (Not sealed, Sealed, Watching, Regressed) with word and icon, its activity and last activity. Search and sort by last activity or best. A sealed experiment is never edited; "New version" is on its detail page. Empty (`ExperimentsEmpty`): "No experiments yet." with "New experiment", because an experiment begins in a chat.

**Experiment detail page** (`ExperimentDetail`, a DetailPage), opened from "Details" in the experiment page's header. Back to the experiment. A title with its state and version. Sections of rows and quiet tables:

- Definition: Metric and direction, Evaluation command, Protected paths per repo, Guardrails, Repeats, Acceptance criteria in one sentence. Immutable: "Sealed experiments never change. A change makes a new version, and numbers do not carry across versions."
- Seal checks: the baseline, the noise check, the cheat checks, as they were when sealed.
- Data: the evaluation data and the holdout, the holdout marked "Restricted to the judge".
- Scoreboard: the chart of every trial across all its sessions, with one line per session, and the numbers.
- Budget: spent of budget, and a control to change it (editors and owners).
- Trials: a table of every trial across all its sessions, with hypothesis, outcome, value, session (named by its agent) and what it started from. Failures are included. A row opens that session's chat at that trial.
- Wins and their pull requests, each linking to its receipt.
- Watch: the switch and its trigger (on a schedule, when the default branch moves), and the last measurement.
- Versions: each version with its seal date; numbers are never compared across them.
- Delivered, Pending and Return appear here only when the experiment states a value per unit.

**Receipt** (`Receipt`, a DetailPage). One win: the experiment version and seal, the commits per repo, the values, repeats and noise, the number of trials tried, the holdout result (Passed or Failed, never a value), the second opinion with its notes, what was spent, and "Signed by the judge". Below it the lineage: what each trial started from, as a chain from the baseline to the win, each node a link to its trial.

### Global search

A centred palette (`Search`, `SearchEmpty`) opened from the sidebar, Cmd/Ctrl+K or "/". Scopes: All, Experiments, Sessions, Trials, People. With no query it shows Waiting for you, Recents and Actions. Results are grouped by kind with the match highlighted, and each opens the exact place: a trial opens its experiment with that trial in view, and a session result opens its chat inside the experiment. The footer shows only keys that work.

### Settings

Settings is a dialog, not a page. It opens from the user menu or a Settings result or action in search. It is a large centred dialog with a left rail (`SettingsRail`) and a search field. The first Esc clears the rail search. The rail is grouped by whose setting it is, with a hairline between groups: **You** (Profile, Appearance, Access tokens, This device) and **Organization** (General, Members, Single sign-on, Agent platforms, Approvals, Budgets, Connections, Audit log). It reopens on the section you used last. The rail search filters sections and rows by title, description and keywords, and opens the first section that matches.

Settings are for reading and revoking what was approved. They are not where setup begins. Two things are still set up here, because no experiment can start without them: agent platforms (self-hosted) and members and sign-in.

What each section shows:

- **Profile** (`SettingsProfile`): name, email, sign-in provider, role. **Appearance** (`SettingsAppearance`): theme, light, dark or system. **Access tokens** (`SettingsTokens`, `SettingsTokensEmpty`): a quiet table of your tokens with scope and last used, Create access token, Revoke; a new token is shown once. **This device** (`SettingsDevice`): Install app, notifications for this device, the server address; installed web app only where it applies.
- **General** (`SettingsGeneral`): the organization name, the deployment (Ratchet cloud or Self-hosted) and the server address.
- **Members** (`SettingsMembers`): users and service principals as a quiet table with role (Owner, Editor, Viewer) and status; invite, change role, remove. An organization always keeps at least one owner.
- **Single sign-on** (`SettingsSso`): OpenID Connect and GitHub, each with its status and the domains it admits. No passwords.
- **Agent platforms** (`SettingsPlatforms`, `SettingsPlatformsEmpty`, `SettingsPlatformsManaged`, `AddPlatform`, `PlatformError`): a quiet table of Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service and Google Agent Runtime: the account, the agents on it, a status (Connected, Needs attention when its credential is rejected), and one platform badged "Judge platform". Row actions are Test connection, Update credential and Remove; an "Add platform" button adds one. Environments and connector credentials are listed under Connections, not here. Self-hosted, owners manage all of it and others see it read-only. On Ratchet cloud the section is read-only for everyone (`SettingsPlatformsManaged`): Ratchet manages the platform accounts and bills at cost through each experiment's budget, and the section shows data and retention per platform (for example "United States only, no zero data retention"). A rejected credential is `PlatformError`: the row says Needs attention and what to do.
- **Approvals** (`SettingsApprovals`): one row per card kind with its policy: Always ask, Allow up to a limit, or Allow. Defaults: Seal experiment asks; Raise budget asks above the limit set here; Attach repo asks; Change environment asks; Open pull request asks. Add credential always asks and shows no choice: a person must type the value. Which role may approve which card is read here.
- **Budgets** (`SettingsBudgets`): the default budget per experiment (the figure the composer's Budget chip starts from; the boards show $10 as the example), the monthly cap and spent this month, and a quiet table of experiments with their budgets and spend. No budget can pass the monthly cap. Owners change the default and the cap; editors change a single experiment's budget from the experiment.
- **Connections** (`SettingsConnections`): what was approved in chats, in three groups: Repos, Credentials (names and where each is kept, never values) and Environments. Each row says who added it, when and how many experiments use it, and has Revoke. Connector credentials are read-only and kept in the platform's credential store.
- **Audit log** (`SettingsAudit`): the tool calls that changed something and the approvals: who, what, when, the outcome, as a quiet table with a filter. Never empty: signing in is the first entry.

Read-only is `SettingsReadOnly`.

- **A setting is a row.** The title with a one-line muted description under it on the left, the control on the right (switch, select, segmented control, field, button), and a hairline between rows. A sub-setting is indented under its parent and is disabled while the parent is off. A section can have several headed groups. A list inside Settings (members, access tokens, connections, audit log) is a quiet table: column headers, an icon or avatar and name with a muted second line, a tag for a kind, an icon and word for a status. No cards inside cards.
- **Organization settings are read-only for non-owners, not hidden.** Owners change them. Editors and viewers see the same section with its controls disabled, a one-line explanation ("Only owners can change this. You can read it.") and a lock beside the section in the rail. Revoking a connection is an owner action.
- **Narrow screens.** Below 720px the dialog is full-screen and the rail becomes a search field with a select of sections.

### Reuse the patterns

One detail-page layout, one popover, one palette and one dialog. A new kind of thing uses them; it does not get its own page style. Menus and popovers close on an outside click. Every menu item is a `.mi`, every select uses the `.selw` wrapper, and every control has a disabled style. A new card kind reuses the card frame: title, what would happen, what it may cost, two buttons.

## Rules for every page

- **One set of page states.** Every page uses the same six: loading, empty, failed, refused, partial, not checked. Each says what happened and what to do, in a sentence. Failure is never drawn as empty or zero, and something not checked is never drawn as healthy.
- **The holdout is marked.** A holdout always shows as "Restricted to the judge". An agent never holds the holdout credential and never sees a holdout value, only the outcome ("Kept", "Failed holdout"). No control grants it to an agent: the control is absent, not disabled. The copy says it is enforced by the credentials: an agent's credential leaves it out and only the judge sandbox's includes it. Chat never changes it. (target: experiments are not built yet.) It has its own board, because the screen looks different.
- **Setup happens in the chat, as a card.** Nobody visits a settings page before the first message. A person states a goal; when the agent needs a repo, a credential, an environment, a budget or a seal, a card appears at that moment.
- **A card shows what would happen and what it may cost before Approve.** Confirm after the preview, never before. Nothing is created or spent until a person decides. A destructive step such as Stop session confirms inline.
- **Nothing is asked when policy already allows it.** A pre-approved card collapses at once to "Allowed by policy". Add credential is never pre-approved.
- **Only the judge's numbers are shown as results.** A number made in a session is never drawn as a result.
- **A control says exactly what it does.** "Seal experiment", not "Approve"; "Stop session", not "OK". A control that does nothing is deleted. A footer or menu lists only keys and actions that work.
- **Same in every client.** If the web app, the macOS app, the CLI and MCP disagree about state or words, one is wrong.
- **Status is never colour alone.** Every status and outcome has a word. An activity has its own icon shape: Working, Waiting for you, Done, Failed. An experiment state has its own icon: Not sealed, Sealed, Watching, Regressed. A trial outcome is a word in a tag: Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected, Failed.

### Empty states

One sentence and at most one action, no feature explanation. A list gets its own board only because the screen looks different. The sidebar is `SidebarEmpty` only when the organization has no experiment. A count that is zero stays on screen (spend `$0`, `0 kept`), never hidden.

| Screen | Empty when | Sentence | The one action | Board |
| --- | --- | --- | --- | --- |
| Home | no experiment in the organization | No experiments yet. | none; the composer is the action | `HomeEmpty` |
| Sidebar | no experiment | Experiments you start appear here. | none | `SidebarEmpty` |
| Experiments | no experiment started | No experiments yet. | New experiment | `ExperimentsEmpty` |
| Search | no match | No results for “query”. | none | `SearchEmpty` |
| Settings, Access tokens | you made none | No access tokens yet. | Create access token (row above) | `SettingsTokensEmpty` |
| Settings, Agent platforms | self-hosted, none added | No agent platform is added yet. | Add platform | `SettingsPlatformsEmpty` |

Never empty, so no board: Members (you are in it, and one owner always remains), Audit log (signing in is the first entry), Approvals, Budgets, General, Profile, Appearance and Single sign-on (settings rows, not lists), an experiment (it starts with the composer). Connections is empty until the first card is approved: its row reads "Nothing approved yet." and has no action, and it shares `SettingsConnections` rather than its own board. An experiment with no trials or no wins says "No trials yet." or "No wins yet." inside its section, with no board.

## Colour and type

Midnight Brass. The values are in `design/tokens.toml`; these are the rules.

- Brass is the brand and the only accent. People do not choose a colour.
- Text meets 4.5:1 in both modes. A control's outline meets 3:1, so a control uses `line2`, not the `line` divider.
- Status is never colour alone: a word or a shape comes with it.
- Light and dark are both designed. A change is checked in both, and a board is drawn light, its dark twin generated.
- The areas keep one colour each in diagrams and navigation: the judge is brass, experiments and sessions are blue, the organization is slate (target; slate is not a token until it is added to `tokens.toml` with a contrast check). Green is the success state only (Kept, Done, Connected), never an area. Never use these colours for anything else, and never rely on colour alone to say which area a thing belongs to.
- Tokens are named by meaning (`canvas`, `raised`, `line2`, `ink`, `accent`), and light mode redefines the same names. Never write a raw colour, radius, size or duration in a screen or a board. Type comes from the type classes, never an inline size.

## Clients

The web app and the macOS app (target) share behaviour, words and element ids, never pixels. The clients are the web app, the web app installed on Windows and Linux, the `ratchet` CLI and `ratchet mcp`. `ratchet "cut checkout p95"` starts the same experiment in the terminal (target). The CLI and MCP print the same words and the same cards, as text. If any two disagree about state or words, one is wrong.

The macOS app is SwiftUI, and macOS only; on Windows and Linux the web app installed as a progressive web app is the desktop app. The Mac app stays ruled in, but its boards are removed for now and are owed once the web direction is approved. When they return, its window chrome, menu bar item, native Settings window, native notifications and offline banner follow the same behaviour as this page. The Mac is only a client of the server's API. It runs no agents and no sandboxes.

What the installed web app adds, on Windows and Linux (`WebInstalled`, `WebNotification`, `WebOffline`):

- Its own window and taskbar icon; the page is the same, with no browser toolbar.
- Install is offered once, quietly: a row in Settings, This device ("Install app"), and the browser's own prompt. Never a banner over work. On macOS the row says "Get the Mac app" instead.
- A badge on the icon: the count of experiments that are Waiting for you.
- Push notifications for what needs attention (a win, Waiting for you, a regression, a budget spent), asked for at the moment it helps (after the first message: "Notify me when this needs me"), never on first load. Turned off in Settings, This device.
- An offline screen when the server cannot be reached: what was last known, marked as not current, and Try again. Nothing is cached beyond the app itself.

## Accessibility

Targets, labels and focus are checked on the boards in both light and dark. Every control is reachable by keyboard, the focus ring uses a token, and every icon-only control has a label. A streaming message is announced when it ends, not word by word; a card that needs a person is announced when it appears.

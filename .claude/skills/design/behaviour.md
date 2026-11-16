# Behaviour

How a screen behaves. The web app and the macOS app (target) share behaviour, words and element ids, never pixels. Values are not here: they live in `design/tokens.toml`, and the boards in `design/mockups/` show the result (see [mockups.md](mockups.md)). The words are in [wording.md](wording.md).

## The app is organised around experiments as sessions (target)

The three basics still own everything underneath; the screen is not split by them.

**Frame.** A fixed 256px sidebar and a main area. Sidebar top: Search, Projects, Dashboard, Customize, More (Run history, Settings). A primary "New experiment" button. Then Pinned, then experiments grouped under collapsible project headings: one line each, a run count and one status icon. The six statuses (Running, Draft, Win, Merged, No win, Stopped) each have their own icon shape, never colour alone. The user menu at the bottom has Settings, a quick light and dark switch, host name and Sign out.

## Page types

Every screen is one of six page types. The type owns the header, back, search, the empty, loading and error states and the actions; the screen supplies content and words. A screen that hand-rolls any of those is a finding, not a variant.

| Page type | Anatomy | Examples |
| --- | --- | --- |
| Thread | Header: title, status, primary metric (baseline to best so far), run, trials, spend of budget, elapsed, Stop run with an inline confirm, Pin, menu. Then a chart of the primary metric per trial with the baseline, the noise band and one lane (line) per agent, then the conversation. A composer docks at the foot and follows the state: change the draft, steer the agents or one with @ while running, or start another run with a note. Back: none, the sidebar is the way out. Search: global only. States: a Draft card until Approve; a failed trial is one calm line, never an empty diff | An experiment, in any status |
| ListPage | Title, a count, search, filter and sort where the list is long, one primary action (a New or Add menu). Rows or cards, never both. Back: none at a sidebar root. Empty: one sentence and at most one action. Loading: a skeleton of the rows. Error: what happened and what to do, with Try again | Projects, Run history (Trials and Sessions tabs), Customize (tabs Evaluations, Skills, Connectors, Agents, Plugins; a Yours and Discover toggle) |
| DetailPage | Back to the named parent, a title with its status, one primary action, a menu for the rest, then sections of settings rows and quiet tables. One layout for every kind under Customize: a new kind gets no page style of its own. Empty: a section with nothing to show says so in a sentence. Read-only: controls disabled with a one-line reason | A project, an evaluation, a connector, an agent, a skill |
| Dialog | Over the app, so you keep your place. A title bar with a close button, a dimmed backdrop, focus moves in and Tab stays inside. Esc, the close button or a click on the backdrop close it and return focus to what opened it. Destructive confirms are inline in the page or a small dialog with the verb as its button | Settings, Add connector, Approve a draft |
| SettingsSection | One section of the Settings dialog: a rail on the left with a search field, the section on the right. Rows, not cards. See "A setting is a row". Read-only for non-owners | Profile, Appearance, Access tokens, This device (web app only), General, Users and groups, Single sign-on, Infrastructure, Budgets, Notes, Audit logs |
| SignIn | One centred panel on the page ground: the mark, one line of purpose, the provider buttons, nothing else. No sidebar, no search. Errors sit under the buttons in one sentence | Sign-in on Ratchet cloud and on a self-hosted server |

### Thread

An experiment is one thread. The conversation holds, in order: the request, Ratchet in short sentences, the draft card (expanded until approved, then one summary line), a seven-step stepper coloured by owning area (1 Start run, Experiment; 2 Create workspaces, Compute, one session per agent; 3 Mint scoped credentials, Data; 4 Agents iterate: change, measure, keep or discard, Experiment, a step that repeats and shows counts; 5 Pick the best kept trial, Experiment; 6 Store history, open pull request, Data; 7 Clean up, Compute), an Agents section, expandable trial cards, notes from people, and the result card with lineage in three columns (Experiment, Data, Compute), Open pull request and Run again.

- **Agents section.** One lane card per agent: name, status (Iterating, Measuring trial 9, Paused, Stopped with its reason), facts (trials, kept, best, spend of its share), what it started from if it adopted another agent's trial, its latest kept trial, and Pause agent. A stopped agent shows that its session ended. The chart draws one lane per agent, so who is improving is visible at a glance.
- **Trial card.** Agent, hypothesis in the agent's own words, change, files, measured value against that agent's best and the baseline, delta, one outcome tag, diff, guardrail metrics. Holdout shows only as an outcome ("Kept", "Failed holdout"), never a value to the agent. "Steps for this trial" lists: the hypothesis, wrote the change, checked it in its session (does not count), asked for a measurement, evaluator measured (repeats, noise), beat the agent's best by X, confirmed in a fresh evaluator sandbox with the holdout passed, Kept. A trial being measured shows Measuring.
- **Composer while running** steers the agents: a note goes to all, or to one with @ (for example "@Codex focus on database queries"). Each agent reads it before its next trial.
- **The stepper's status line** counts: "Step 4 of 7 · 2 agents iterating · 28 trials, 4 kept." On the Mac: "Step 4 of 7 · agents iterate in their sessions; the evaluator sandbox measures with the holdout."
- The draft card says where it would run (the agent platforms, and the measuring platform with its environment), that agents get read-only credentials that leave out the holdout, and that the evaluator gets its own. Nothing is created until Approve.
- A trial's diff is the commit on the agent's branch, and its transcript and logs stay on the agent platform; they are fetched when someone opens them, and the card says so. When the platform cannot be reached the trial shows one calm line in the failed state: "Can't reach Claude Managed Agents. Transcripts and logs stay there; try again when it is back." It is never drawn as an empty diff.
- The result card's Data column names dataset versions, skill versions and the credential scope (read-only, holdout to the evaluator only). Its Compute column names the agent platforms, the measuring platform, the environment (image and size), agent versions and that the evaluator signed the numbers.

### New experiment

A dialog that asks one question, "What do you want to improve?", with a text box and pills that open popovers: Project, Evaluation, Skills, Data, Environment, Agents, Budget, Guardrails and paths. Environment picks the measuring platform and an environment (image and size) from those set up under Settings, Infrastructure; a platform not added yet shows "Add platform" and leads there. "Draft experiment" starts a drafting session at once and the experiment is a Draft. No run session starts until Approve.

### Customize

A connector is a data system or the code host (Snowflake, Databricks, DynamoDB, BigQuery, Postgres, Amazon S3, GitHub, GitLab, an MCP server). A cloud account is not a connector; it is set up under Infrastructure. Discover shows a featured banner (a carousel for connectors), a "For you" grid whose "+" becomes a check, and a "New" section with Show all. Configure opens the detail page.

- A connector's page says how access is scoped: that the credential is read-only, registered once by the customer and kept in the platform's credential store, what agents get (read-only, holdout excluded) and what the evaluator sandbox gets. Adding a connector says Ratchet keeps no connector secret.
- An agent's page shows the model provider as a field and the platform it runs on (Claude Code · Claude Managed Agents). The platform holds the model access; there is no key source setting.

### Global search

A centred palette opened from the sidebar, the top bar, Cmd/Ctrl+K or "/". Scopes: All, Experiments, Projects, Datasets, Customize, Runs, People. With no query it shows Needs attention, Recents and Actions. Results are grouped by kind with the match highlighted, and each opens the exact place: a trial opens its thread with that trial expanded. The footer shows only keys that work.

### Settings

Settings is a dialog, not a page. It opens from the user menu, More, or a Settings result or action in search. It is a large centred dialog with a left rail and a search field. The first Esc clears the rail search. The rail is grouped by whose setting it is, with a hairline between groups: **You** (Profile, Appearance, Access tokens, and in the web app This device) and **Organization** (General, Users and groups, Single sign-on, Infrastructure, Budgets, Notes, Audit logs). It reopens on the section you used last. The rail search filters sections and rows by title, description and keywords, and opens the first section that matches.

- General shows the deployment (Ratchet cloud or Self-hosted) and the server address.
- Infrastructure (heading "Infrastructure") has three sections: Agent platforms, Environments and Credentials. Agent platforms is a quiet table of Claude Managed Agents, AWS AgentCore and the OpenAI Agents API: the account, the agents on it, a status (Connected, Needs attention when its credential is rejected), and one platform badged Measuring platform. Row actions are Test connection, Update credential and Remove; an "Add platform" button adds one. Environments lists each one with its platform, image, size and use (Evaluator or Agents). Credentials lists the connector credentials registered once, with where each is kept. Self-hosted, owners manage all of it and others see it read-only. On Ratchet cloud the page is read-only for everyone: Ratchet manages the platform accounts and bills at cost through each run's budget, and the page shows data and retention per platform (for example "United States only, no zero data retention"). The Mac app has no machine section.
- **A setting is a row.** The title with a one-line muted description under it on the left, the control on the right (switch, select, segmented control, field, button), and a hairline between rows. A sub-setting is indented under its parent and is disabled while the parent is off. A section can have several headed groups. A list inside Settings (users, groups, access tokens, per-experiment budgets, audit logs) is a quiet table: column headers, an icon or avatar and name with a muted second line, a tag for a kind, an icon and word for a status. No cards inside cards.
- **Organization settings are read-only for non-owners, not hidden.** Owners change them. Editors and viewers see the same section with its controls disabled, a one-line explanation ("Only owners can change roles. You can read them.") and a lock beside the section in the rail. Notes can be added by editors and owners. An organization always keeps at least one owner. The theme is one setting with two places to change it: Appearance, and the quick switch in the user menu.
- **Narrow screens.** Below 720px the dialog is full-screen and the rail becomes a search field with a select of sections.

### Reuse the patterns

One detail-page layout, one popover, one palette and one dialog. A new kind of thing under Customize uses them; it does not get its own page style. Menus and popovers close on an outside click. Every menu item is a `.mi`, every select uses the `.selw` wrapper, and every control has a disabled style.

## Rules for every page

- **One set of page states.** Every page uses the same six: loading, empty, failed, refused, partial, not checked. Each says what happened and what to do, in a sentence. Failure is never drawn as empty or zero, and something not checked is never drawn as healthy.
- **The holdout is marked.** A holdout dataset always shows as "Restricted to evaluator" and cannot be granted to agents. The grant control is absent for it, not disabled. The copy says it is enforced by the credentials: an agent's credential leaves it out and only the evaluator's includes it. A skill, a plugin or a connector never changes that. (target: datasets and grants are not built yet.) It has its own board, because the screen looks different.
- **A new experiment is a short declaration.** The user states the goal and limits, then picks context and infrastructure that already exist. Setting those up happens elsewhere, once, under Customize. (target. Today New optimization is a wizard over repository, benchmark template, agents and AWS.)
- **A command that spends or changes things shows what it would do first.** Confirm after the preview, never before. In the app that is Draft, then Approve: the draft shows what would run and what it may cost, and nothing is created until Approve. A destructive step such as Stop run confirms inline.
- **A control says exactly what it does.** A control that does nothing is deleted. A footer or menu lists only keys and actions that work.
- **Same in every client.** If the web app, the macOS app, the CLI and MCP disagree about state or words, one is wrong.
- **Status is never colour alone.** Every status and outcome has a word, and a status has its own icon shape.

### Empty states

One sentence and at most one action, no feature explanation. A list gets its own board only because the screen looks different. The sidebar is `SidebarEmpty` only when the organization has nothing at all. A count that is zero stays on screen (the Dashboard shows `$0`, `0×`), never hidden.

| Screen | Empty when | Sentence | The one action | Board |
| --- | --- | --- | --- | --- |
| Projects | no project or experiment in the organization | No experiments yet. | New experiment | `HomeEmpty` |
| Project | the project has no experiment | No experiments in this project yet. | New experiment (header) | `ProjectEmpty` |
| Dashboard | no win yet; tiles show zeros and spend | No wins yet. | New experiment | `DashboardEmpty` |
| Run history, Trials (Sessions the same) | nothing has run | No trials yet. | New experiment | `RunHistoryEmpty` |
| Evaluations, Skills, Agents | none added | No evaluations / skills / agents yet. | Add evaluation / skill / agent | `EvaluationsEmpty`, `SkillsEmpty`, `AgentsEmpty` |
| Plugins, Yours | none installed | No plugins installed. | Add plugin | `PluginsEmpty` |
| Connectors | none added | No connectors yet. | Add connector | `ConnectorsEmpty` |
| Search | no match | No results for “query”. | none | `SearchEmpty` |
| Settings, Access tokens | you made none | No access tokens yet. | Create access token (row above) | `SettingsTokensEmpty` |
| Settings, Notes | no note | No notes yet. | Add note (row above) | `SettingsNotesEmpty` |
| Settings, Infrastructure | self-hosted, no platform added | No agent platform is added yet. | Add platform | `InfrastructureEmpty` |

Never empty, so no board: Users and groups (you are in it, and one owner always remains), Audit logs (signing in is the first entry), Budgets, General and Single sign-on (settings rows, not lists), a thread (it starts as a Draft).

## Colour and type

Midnight Brass. The values are in `design/tokens.toml`; these are the rules.

- Brass is the brand and the only accent. People do not choose a colour.
- Text meets 4.5:1 in both modes. A control's outline meets 3:1, so a control uses `line2`, not the `line` divider.
- Status is never colour alone: a word or a shape comes with it.
- Light and dark are both designed. A change is checked in both, and a board is drawn light, its dark twin generated.
- The basics keep one colour each in diagrams and navigation: optimization brass, context green, infrastructure blue, organization slate (target; slate is not a token until it is added to `tokens.toml` with a contrast check). Never use these colours for anything else, and never rely on colour alone to say which area a thing belongs to.
- Tokens are named by meaning (`canvas`, `raised`, `line2`, `ink`, `accent`), and light mode redefines the same names. Never write a raw colour, radius, size or duration in a screen or a board. Type comes from the type classes, never an inline size.

## Clients

The web app and the macOS app (target) share behaviour, words and element ids, never pixels. The macOS app is SwiftUI, and macOS only; on Windows and Linux the web app installed as a progressive web app is the desktop app. If the two disagree about state or words, one is wrong.

What the Mac adds:

- Native window chrome: the title bar, toolbar and sidebar are the system's.
- A menu bar item: the runs in progress, what needs attention, and Open Ratchet.
- A native Settings window, with the same sections and rows as the web dialog.
- Native notifications, for what needs attention.
- Sign-in through the browser, never a password field in the app.
- An offline banner: the app shows what it last knew, marked as not current, until the server is reachable again.

What the installed web app adds, on Windows and Linux:

- Its own window and taskbar icon; the page is the same, with no browser toolbar.
- Install is offered once, quietly: a row in Settings, This device ("Install app"), and the browser's own prompt. Never a banner over work. On macOS the row says "Get the Mac app" instead.
- A badge on the icon: the count of experiments that need attention.
- Push notifications for what needs attention, asked for at the moment it helps (after starting a run: "Notify me when this run finishes"), never on first load. Turned off in Settings, This device.
- An offline screen when the server cannot be reached: what was last known, marked as not current, and Try again. Nothing is cached beyond the app itself.

The Mac is only a client of the server's API. It runs no agents and no sandboxes. "This machine" is deferred.

## Accessibility

Targets, labels and focus are checked on the boards in both light and dark. Every control is reachable by keyboard, the focus ring uses a token, and every icon-only control has a label.

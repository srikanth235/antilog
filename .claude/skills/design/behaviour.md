# Behaviour

How a screen behaves. The web app and the macOS app (target; its boards are owed) share behaviour, words and element ids, never pixels. Values live in `design/tokens.toml`; the boards in `design/mockups/` show the result and are listed in the index in [`design/mockups/README.md`](../../../design/mockups/README.md) (flow and checks: [mockups.md](mockups.md)). Exact labels are in [wording.md](wording.md). What a session is and what a card asks are in [session](../session/SKILL.md) and [approvals](../organization/approvals.md). Where a board and this page disagree about a screen, the light board wins and this page is fixed. Experiments and the judge are parked ([../judge/SKILL.md](../judge/SKILL.md)); where a board still draws one of their screens, it is archive and no rule here follows it.

A person states a goal in a project, a session starts, and a coding agent in a machine of its own talks to them. There is no mode, no plan step and no setup screen: setup happens in the chat, as a card.

## Shell and sidebar

A fixed 256px sidebar and a main area.

- **Top** (`SidebarTop`): the brand, "New session", Search (a link to `SearchStart`, with the ⌘K hint), and the nav rows Projects and Customize.
- **Rows** (`SidebarRows`): Pinned first, then projects as collapsible headings, each listing its sessions. A session is one line: its title and one status icon (what needs you). A row's status icon updates live.
- **Foot** (`SidebarFoot`): the user menu: Settings, a light and dark switch, the host name, Sign out.
- `SidebarEmpty` is the same chrome with no rows, shown only when the organization has no session; Projects and Customize stay.

A session has a **status**: Working (an open arc), Waiting for you (a half-filled circle), Done (a check in a circle), Failed (a cross in a circle). Each has a word and its own icon shape, never colour alone. A session a person stopped is Done, with "Stopped by NAME".

An agent's message carries the Antilog mark and "Antilog", with the agent as a quiet second label. Agent logos appear only where an agent is the subject: the agent menu and Customize rows.

## Page types

Every screen is one of six types. A new kind of thing under Customize does not make a seventh. The type owns the header, back, search, the empty, loading and error states and the actions; a screen that hand-rolls any of those is a finding.

| Page type | Anatomy | Examples |
| --- | --- | --- |
| Session | One slim header bar, then the conversation with the composer floating at the foot. No back (the sidebar is the way out); global search only. A card waits until a person decides; a failure is one calm line | A session in any status; Home is the Session type with no conversation yet |
| ListPage | Title, a count, search, filter and sort where the list is long, one primary action. Rows, or cards where a board draws them (Discover), never both. No back at a sidebar root. Empty: one sentence and at most one action. Loading: a skeleton of the rows. Error: what happened and what to do, with Try again | Projects, each Customize tab, Waiting for you on Home, Search results |
| DetailPage | Back to the named parent, a title with its state, one primary action, a menu for the rest, then sections of rows and quiet tables. One layout for every item under Customize. An empty section says so in a sentence. Read-only: controls disabled with a one-line reason | A project and every Customize item |
| Dialog | Over the app. A title bar with a close button, a dimmed backdrop; focus moves in and Tab stays inside. Esc, the close button or the backdrop close it and return focus to what opened it. A destructive confirm is inline or a small dialog with the verb as its button | Settings |
| SettingsSection | One section of the Settings dialog, a rail on the left (`SettingsRail`), rows not cards, read-only for non-owners | Profile through Audit log |
| SignIn | One centred panel on the page ground: the mark, one line of purpose, the provider buttons. No sidebar, no search. Errors sit under the buttons in one sentence | `SignIn`, `SignInError`, `SignInNoAccess` |

## The session page

The session page is the chat: one agent in one machine, any number of repos attached, in a project. A session has a status and nothing else; its project owns the roles and the monthly cap.

### Header and panels

One slim bar across the main area, like Claude Code's desktop header.

- **Left.** The breadcrumb (project / session title in bold), a chevron that opens the **title menu** (Rename, Pin or Unpin, Copy link, Stop session, Archive), then the status pill. Stop session confirms inline in the chat.
- **Right.** Icon toggles in order Details, Changes, Terminal, Preview, Files; a separator; More (Export conversation, Keyboard shortcuts, Report a problem); Share (icon and word). Each toggle has `aria-pressed`, an `aria-label` and a `title` with its shortcut.
- **One panel at a time.** A toggle opens its panel beside the chat; the pressed toggle or the panel's close button closes it; another toggle swaps it. About 440px wide, resizable by a drag handle (a separator that takes the arrow keys), with its own header. The chat column narrows. Boards draw panels closed unless they are about that panel.

| Panel | Shows |
| --- | --- |
| **Overview** (the toggle was "Details"; `SessionOverview`) | A calm vertical list of sections, each linking to the page that shows it in full: Environment (the environment's name with its settings link; the repos with "+" to attach and their branch), Context (a usage bar, "44% used", "88k of 200k" and a short legend), Spend (spent, the session's cap, "Change budget"), Connectors (the ones the session may use), Outputs (files, patches and the delivered pull request). A row we cannot fill honestly is left out; the web app omits "+", the branch, Context and Outputs until the API serves them (target) |
| **Changes** | The diff of this session's branch per repo against its base; files grouped by repo with +/- counts, the unified diff below. The header reads "main → <session branch>" with the repo; the main pane is a collapsible unified diff per file with its hunks, the right-hand side lists the commits (All changes first, or one commit) and the files (name, faded folder, +N -N, removed files in red). The runner reports it at the end of each turn and the server keeps the latest ([../../../docs/V0.md](../../../docs/V0.md), Changes tab); the unmodified lines between hunks show their count and do not expand until the server sends fuller context |
| **Terminal** | A shell in this session's machine. Editors and owners type; a viewer reads, with a line saying so. Opening it is recorded in the audit log |
| **Preview** | A browser on a machine port, with address bar, reload, open in a new tab. The button exists only when the environment exposes a port: absent, not disabled |
| **Files** | The attached repos' file tree in the machine, repo as the top level; the open file read-only with a path bar |

- **Share** (a popover, no panel): people in the project with their role marked as inherited; Copy link. Sharing never grants more than a person's project role. A viewer sees only Copy link.
- **Read-only** (`SessionReadOnly`): a viewer opens every panel but cannot type in Terminal; Rename, Pin, Stop session and Archive are disabled, with a one-line reason in the header.

### The conversation

In time order: messages (short sentences; text streams, ending with a cursor bar until done), tool calls (consecutive calls between two messages fold into one quiet group line, "Ran 19 commands (1 failed)", muted with a chevron and no box until opened; open, a bordered list with one row per call, "Ran <command or target>", which expands to the full input and the truncated output in monospace with Copy; a failed call is marked in its row and counted in the title; the latest group is open and reads "Running… 3 commands" while the agent works and folds when it writes; titles say "Ran N commands", "Edited N files", "Read N files", "Called <tool> N times" or, mixed, "Ran N steps"; an Antilog tool shows only its Antilog name; `SessionToolCalls`), timeline lines for what happened (a stop, a compaction), and cards. A card is the only place structured output appears; there is no step list and no wizard.

**Long tool output** is stored cut to 4 KB (shown as it was kept, ending "… cut"); the rest stays on the session's machine. Opening more fetches it from there ("Fetching from the session"), or says the machine has dropped it, or says it cannot be reached, with Try again. Neither is drawn as an empty block.

### The composer

It follows Claude Code's: a rounded box with a soft shadow and a quiet row of settings. On a session it floats over the conversation with a fade above it and no band of its own. Home's start composer is large, because on Home it is the page.

- **Idle, one line.** "+", the text, small icons (Agent, Environment, Mode, the budget ring), mic and send. It grows with the text to about eight lines, then scrolls inside. On focus or with text the options show in full as a quiet row under the box; idle they collapse to icons. Every icon has a label and a tooltip.
- **Grown** (`SessionComposerExpanded`): an attachment strip when something is attached, the text area, and a bottom row: "+" (attach files, images, repos or connectors) left, mic and send right. "/" lists commands. Send is a return arrow, quiet while empty, a filled accent circle with text.
- **Options under the box**, each a popover: Project and Repos (Home only; fixed once it runs), Agent, Environment, Mode, and the budget ring (fills with spend of the session's cap). **Mode** is the approval mode: Ask (every card asks) or Follow policy (cards that policy allows go through; policy still caps it, see [approvals](../organization/approvals.md)). "Auto" names only the agent choice, never the approval mode.
- **By status.** *Working:* a person types to steer; while the agent works send is a stop button ("Interrupt"), as in Claude Code; with text in the box send shows and the text is delivered before the next step. *Waiting for you:* the composer stays open and the waiting card is above it. *Done or Failed:* "Continue this session" starts a new machine; Failed also offers "Start again". *Read-only:* the box is greyed and nothing is active.
- **Agent menu** (`SessionAgentMenu`). A popover above the box: **Auto** first with one line of why, then each agent turned on with its logo and cost tier ("$" to "$$$"), the current one marked. A person can always pick a named agent and no agent is pre-selected by brand. Auto uses the organization's default agent. Home shows "Auto".
- **Context.** Antilog compacts at 80% and says so in a timeline line (target).
- **A new session** (`Home`, `HomeEmpty`): all the settings. Project defaults to the project the person is in (or last used) and lists those where they are an editor or owner. The budget starts from the project's default per session, falling back to the organization's. Send starts the session; nothing else is asked.

## Cards

A card is a frame: a title, what would happen, what it may cost, two buttons; every new kind reuses it. The preview comes before Approve, and nothing is created or spent until a person decides. A pre-approved card collapses at once to "Allowed by policy". The six kinds and their policies are in [approvals](../organization/approvals.md); their fields and words are in [wording](wording.md#cards). What a card does:

- **Raise budget** (`SessionBudget`), **Add credential** (`SessionCredential`), **Attach repo** (`SessionRepo`): the card appears the moment the agent needs it. Add credential is never pre-approved: a person types each value into a secure field, never in the chat.
- **Change environment**: the change from now, as a diff of image, setup script, size and network.
- **Allow this tool?** (`ask_first`): the harness asked for a tool set to ask first; Allow once or Decline.
- **Use a connector tool** (`SessionConnectorTool`): the connector, the tool and the exact input; one approval pays for one call.
- **Failed** (`SessionFailed`) is a calm line, not a card: where it failed and how to start again.

## Home

New session is a composer on the home page, not a dialog or a form (`Home`): the large composer, then **Waiting for you**: a ListPage section of sessions that need a person, each with the card it waits on and its age; a row opens the session with the card in view. With none the section is absent, not an empty box.

Sending starts a session in the chosen project. With no sessions (`HomeEmpty`) the page is the composer and "No sessions yet.", with `SidebarEmpty`.

## Projects

A project groups the sessions for one area of the code; every session belongs to exactly one, and New session asks for it with the Project chip.

**Projects** (`Projects`, a ListPage, from the sidebar row). Every project the person can see, with description, their role (Owner, Editor, Viewer), sessions and spend this month; a person who is not an organization owner sees only their projects. Primary action "New project" (organization owners). Empty: "No projects yet." with "New project"; a brand-new organization sees `HomeEmpty`.

**Project page** (`Project`, a DetailPage). Back to Projects; title, the person's role, description, "New session" (opens the composer with this project chosen). Sections:

- **Sessions**: a table (session, agent, status, spent). Empty (`ProjectEmpty`): "No sessions in this project yet." with "New session".
- **Budget**: spent this month of the project's cap, and the default budget per session (falling back to the organization's). Project owners change both; neither passes the organization's cap.
- **Members**: each member's role in this project; a role set for the project is marked, otherwise the organization role shows as the default. Project owners set roles; a project keeps at least one owner.

A viewer sees the same page with controls disabled and a one-line reason.

## Customize

A sidebar page with tabs **Skills, Connectors, Agents and Plugins**. Each tab is a ListPage with a **Yours** toggle (a link, so it is a URL). Yours lists what the organization has. Discover is not built in v0 for Skills, Connectors or Plugins. Each tab's primary action is "Add skill", "Add connector", "Add agent" or "Add plugin". A row opens the one shared DetailPage. Customize is a library of reusable things; setting one up from a card in a chat adds it here too. Owners and editors add and change items; a viewer reads, with controls disabled and a one-line reason.

- **Skills**: organization skill packs, `SKILL.md` folders with fixed versions. A row shows name, description and latest version. The detail page shows the versions and the files of the latest, and the line that every session gets the latest version. Add: paste or choose a `SKILL.md`, or choose a set of text files; the same name makes the next version. There is no per-project switch and no Discover, source or per-agent reader table in v0 ([docs/V0.md](../../../docs/V0.md)).
- **Connectors**: remote MCP servers a session may call through the server. The code host is not a connector (repos stay under Settings, Connections).
  - *List*: a quiet table; each row has the name, its address, status (Connected, Reconnect, Not checked) as word and icon, and the number of tools.
  - *Detail*: "How access works": the credential is kept sealed on the server, never sent to a session; a session calls the server, which calls the connector. A **Tools** table (name, description, a select On, Off or Ask first). Actions: Check again, Update credential, Remove.
  - *Add* (`AddConnector`): a name, the https address and an optional credential in a secure field, with a line saying what the server does with it.
  - *Error* (`ConnectorError`): what could not be read and what to do, "Check again" and "Update credential".
- **Agents**: Yours lists the agents the organization turned on: logo, cost tier, and a switch for whether Auto may use it. A control sets the organization's default agent, which Auto uses. A row opens `AgentDetail`: provider, model and "What the agent holds" (no model key). There is no agent order, no Pause agent and no per-agent budget share. Model keys stay in Settings, Model keys, which the Add agent dialog links to. Self-hosted, owners manage agents and others read; on Antilog cloud the list is read-only. (A board that still draws figures per trial or a scoreboard is archive: the judge is parked.)
- **Plugins**: an Antilog bundle: a manifest with skill packs and connectors. A row shows what it adds; the detail page lists the parts by kind, each linking to its own page. A plugin has no powers of its own and its page says so; its connectors arrive without a credential and Not checked. Removing a plugin asks in a small dialog whether to keep each part.

## Global search

A centred palette over the app, opened from the sidebar's Search row, Cmd/Ctrl+K or "/". The field is focused. Scopes: All, Projects, Sessions, Customize, People; Left and Right change scope.

- **No query** (`SearchStart`): Needs attention (what waits for you, a connector that needs reconnecting), Recents (last sessions and connectors opened) and Actions (New session, New project, Add connector, Open Settings). Typing replaces it with results.
- **With a query** (`Search`): results grouped by kind in scope order, the match highlighted. A project shows its session count; a session its status, agent and repos; Customize rows say what they are and the group has "Show all N". Each result opens the exact place (a session opens its chat). No match: `SearchEmpty`.
- **Footer keys**, only keys that work: Close Esc, Move Up and Down, Change scope Left and Right, Actions Option+Enter (only where a result has a menu), Open Enter.

## Settings

A large centred dialog, not a page, opened from the user menu or a Settings result or action in search. A left rail (`SettingsRail`) has a search field (the first Esc clears it) and two groups with a hairline between: **You** (Profile, Appearance, Access tokens, This device) and **Organization** (General, Members, Single sign-on, Approvals, Budgets, Connections, Audit log). It reopens on the section last used; the search filters sections and rows by title, description and keywords. Below 720px the dialog is full-screen and the rail becomes a search field with a select.

Settings are for reading and revoking what was approved; setup does not begin here. Only two things are set up here, because no session can start without them: model keys and members with sign-in. Everything reusable is set up once in Customize.

| Section | Shows |
| --- | --- |
| Profile | Name, email, role, sign-in provider, Sign out |
| Appearance | Theme: System, Light or Dark |
| Access tokens | Your tokens (kind, what each can do, last used, expiry, Revoke) and "Create access token"; a new token is shown once |
| This device | Install app, notifications for this device; installed web app only where it applies |
| General | Organization name, deployment (Antilog cloud or Self-hosted), server address, data location |
| Members | Users and service principals: how each signs in and their organization role (Owner, Editor, Viewer), their default in every project. Invite, change role, remove. At least one owner always remains. A project's own roles are on the project page |
| Single sign-on | The sign-in methods: GitHub and Google (each a switch), and, on a self-hosted server only, "Your OpenID Connect provider" (single sign-on), which only an owner sets up (provider, client ID, client secret reference, group mapping, status); how long a sign-in lasts. No passwords. Antilog cloud has no provider row (owner's call, 17 Nov 2026), so there the section shows only GitHub and Google. The boards show the self-hosted view |
| Approvals | One row per card kind (six) with its policy: Always ask, Allow up to a limit, or Allow. Add credential always asks and offers no choice. Only owners change policy |
| Budgets | The organization's monthly cap and default budget per session, spent this month, and the projects with their caps and spend (`GET /budgets`). No session cap passes its project's cap, and no project cap passes the monthly cap. Organization owners change the cap and default; project owners a project's cap and default (on the project page); editors one session's budget |
| Connections | Environments (built): each with its network, size, version and who changed it, "Edit" (a form that saves the next version, with the warning that the agent can read variables) and "Archive"; "Add environment" for owners and editors. What was approved in chats: Repos and Credentials (names and where kept, never values), each with who added it, when, and Revoke (target). A link to Customize, Connectors |
| Audit log | The tool calls that changed something and the approvals: who, what, when, outcome, with an "Approvals only" filter. Never empty: signing in is the first entry |

- **A setting is a row**: title and a one-line muted description on the left, the control on the right, a hairline between rows. A sub-setting is indented and disabled while its parent is off. A list inside Settings is a quiet table with a tag for a kind and an icon and word for a status. No cards inside cards.
- **Read-only for non-owners, not hidden** (`SettingsReadOnly`): editors and viewers see the same section with controls disabled, a one-line reason and a lock beside the section in the rail. Revoking a connection is an owner action.

## Rules for every page

- **One set of page states**: loading, empty, failed, refused, partial, not checked. Each says what happened and what to do. Failure is never drawn as empty or zero, and something not checked is never drawn as healthy.
- **Setup happens in the chat, as a card**, at the moment the agent needs a repo, credential, environment or budget.
- **A card shows what would happen and what it may cost before Approve.** A destructive step such as Stop session confirms inline. Nothing is asked when policy already allows it; Add credential is never pre-approved.
- **A control says exactly what it does** ("Raise budget", not "Approve"). A control that does nothing is deleted. A footer or menu lists only keys and actions that work.
- **Status is never colour alone.** Every status has a word and its own icon shape.
- **Same in every client.** If the web app and the macOS app disagree about state or words, one is wrong.
- **Reuse the patterns.** One detail layout, one popover, one palette, one dialog. Menus and popovers close on an outside click. Every menu item is a `.mi`, every select uses `.selw`, every control has a disabled style.

### Empty states

One sentence and at most one action, no feature explanation. A list gets its own board only because the screen looks different. A count that is zero stays on screen (`$0`).

| Screen | Empty when | Sentence | The one action | Board |
| --- | --- | --- | --- | --- |
| Home | no session | No sessions yet. | none; the composer is the action | `HomeEmpty` |
| Sidebar | no session | Sessions you start appear here. | none | `SidebarEmpty` |
| Search | no match | No results for “query”. | none | `SearchEmpty` |
| Settings, Access tokens | none made | No access tokens yet. | Create access token | `SettingsTokensEmpty` |
| Projects | no project | No projects yet. | New project | none; `HomeEmpty` covers a new organization |
| Project | no session | No sessions in this project yet. | New session | `ProjectEmpty` |
| Customize, Skills | none added | No skills yet. | Add skill | `SkillsEmpty` |
| Customize, Connectors | none added | No connectors yet. | Add connector | `ConnectorsEmpty` |
| Customize, Agents | none added | No agents yet. | Add agent | `AgentsEmpty` |
| Customize, Plugins | none installed | No plugins installed. | Add plugin | `PluginsEmpty` |

Never empty, so no board: Members, Audit log, Approvals, Budgets, General, Profile, Appearance and Single sign-on, and a session (it starts with its prompt). Connections before the first approval says "Nothing approved yet." A connector that cannot be read shows "Not checked" (`ConnectorError`), never an empty table.

## Colour and type

Midnight Brass; values are in `design/tokens.toml`.

- Brass is the brand and the only accent. People do not choose a colour.
- Text meets 4.5:1 in both modes. A control's outline meets 3:1, so a control uses `line2`, not the `line` divider.
- Light and dark are both designed and checked; a board is drawn light and its dark twin generated.
- Areas keep one colour each in diagrams and navigation: sessions blue, the organization slate (target; not a token until added to `tokens.toml` with a contrast check). Green is the success state only (Done, Connected), never an area. Never rely on colour alone to say which area a thing belongs to.
- Tokens are named by meaning (`canvas`, `raised`, `line2`, `ink`, `accent`), and light mode redefines the same names. Never write a raw colour, radius, size or duration in a screen or a board. Type comes from the type classes, never an inline size.

## Clients

The clients are the web app, the web app installed on Windows and Linux, and the macOS app (target). The `antilog` CLI is the runner only and has no screen.

The macOS app is SwiftUI and macOS only; on Windows and Linux the installed web app is the desktop app. Its boards are owed once the web direction is approved; its window chrome, menu bar item, native Settings window, notifications and offline banner follow this page. It is only a client of the server's API and runs no agents.

The installed web app (`WebInstalled`, `WebNotification`, `WebOffline`) has its own window and taskbar icon, with no browser toolbar. Install is offered once, quietly, in Settings, This device ("Install app"; on macOS "Get the Mac app"), never as a banner over work. A badge on the icon counts sessions that are Waiting for you. Push notifications (Waiting for you, a budget spent, Failed) are asked for at the moment they help, never on first load, and turned off in Settings, This device. When the server cannot be reached an offline screen shows what was last known, marked as not current, with Try again; nothing is cached beyond the app itself.

## Accessibility

Targets, labels and focus are checked on the boards in both light and dark. Every control is reachable by keyboard, the focus ring uses a token, and every icon-only control has a label. A streaming message is announced when it ends, not word by word; a card that needs a person is announced when it appears.

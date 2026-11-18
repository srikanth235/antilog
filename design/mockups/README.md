# Mockups

One board per screen state, for the web app (macOS app boards are owed). There is no prototype to maintain: the clickable view is the boards themselves, linked to each other.

## What is here

- `*.dc.html`: boards, each one fixed-size root element. Web boards are 1440 by 900; the rules boards are taller.
- Parts, the shared chrome, on the Components page: `Sidebar`, `SidebarEmpty` (no rows, for an organization with no experiments), `SettingsRail`, and the nested pieces `SidebarRows`, `SidebarTop` and `SidebarFoot`. A part is a plain static board; the generator inserts its content into every board that marks a place for it (see Parts below). The session header varies per screen, so it is board content, not a part.
- `canvas.json`: the pages and where each board sits. Pages: Design rules, Components, Web light, Web dark, Archive. Rows each have a title note (see the index below). Boards in a row are 1440 wide with 80px gaps.
- Session boards are copies of one frame: the slim header bar (`.hbar`), panels and the floating one-line composer. See [mockups.md](../../.claude/skills/design/mockups.md) for which board draws which panel and card.
- Cost and Return boards: `Report` and `AddValueModel` (a dialog over `Project`) in row 6; `Agents` is the agent scoreboard; `ExperimentDetail` and `SettingsBudgets` show where the spend went. Their CSS is the `aims: pages` block in `components.css` (class names `ap-`).
- `tokens.css` (generated), `icons.css` and `components.css` (authored, values only through `var()`). Boards are plain files with no runtime: open one from disk.
- `DesignRules.dc.html` shows every token in light and dark with its contrast ratio, the type scale, radii and button kinds. `Decisions.dc.html` lists the owner's calls, DECIDED with a date or OPEN.
- Values live in `design/tokens.toml` and nowhere else.

## Board index

Every light board, by canvas row (each has a generated `XDark` twin except the rules boards and the parts). `ls design/mockups/*.dc.html | grep -v Dark` lists the same set.

| Row | Boards |
| --- | --- |
| Rules | `DesignRules`, `Decisions` |
| Components (parts) | `Sidebar`, `SettingsRail`, `SidebarRows`, `SidebarTop`, `SidebarFoot`, `SidebarEmpty` |
| 1 Sign in | `SignIn`, `SignInError`, `SignInNoAccess` |
| 2 Home | `Home`, `HomeEmpty` |
| 3 Session | `SessionStart`, `SessionSeal`, `SessionWorking`, `SessionTrials`, `SessionChanges`, `SessionTerminal`, `SessionPreview`, `SessionFiles`, `SessionShare`, `SessionTitleMenu`, `SessionSpawned`, `SessionWin`, `SessionDelivered`, `SessionNoWin`, `SessionBudget`, `SessionCredential`, `SessionRepo`, `SessionRegression`, `SessionFailed`, `SessionReadOnly`, `SessionComposerExpanded`, `SessionAgentMenu`, `SessionKeepGoing`, `SessionHandOff`, `SessionFailover` |
| 4 Experiment and receipts | `ExperimentDetail`, `Receipt` |
| 5 Search | `SearchStart`, `Search`, `SearchEmpty` |
| 6 Projects and Report | `Projects`, `Project`, `ProjectEmpty`, `Report`, `AddValueModel` |
| 6 Customize | `Evaluations`, `EvaluationsEmpty`, `EvaluationsDiscover`, `EvaluationDetail`, `AddEvaluation`, `Skills`, `SkillsEmpty`, `SkillsDiscover`, `SkillDetail`, `AddSkill`, `Connectors`, `ConnectorsEmpty`, `ConnectorsDiscover`, `ConnectorDetail`, `AddConnector`, `AddConnectorMcp`, `ConnectorError`, `DatasetHoldout`, `Agents`, `AgentsEmpty`, `AgentsDiscover`, `AgentDetail`, `AddAgent`, `Plugins`, `PluginsEmpty`, `PluginsDiscover`, `PluginDetail`, `AddPlugin` |
| 8 Agent platforms, approvals, budgets, connections | `SettingsPlatforms`, `SettingsPlatformsEmpty`, `SettingsPlatformsManaged`, `PlatformError`, `AddPlatform`, `SettingsBudgets`, `SettingsApprovals`, `SettingsConnections` |
| 9 Organization | `SettingsProfile`, `SettingsAppearance`, `SettingsTokens`, `SettingsTokensEmpty`, `SettingsDevice`, `SettingsGeneral`, `SettingsMembers`, `SettingsSso`, `SettingsAudit`, `SettingsReadOnly` |
| 10 Installed web app | `WebInstalled`, `WebNotification`, `WebOffline` |
| 11 Landing (antilog.tech) | `Landing`: a responsive page, 1440 wide and never wider than the screen, with its height set by its content; it works at 400px. Its styles are the `lp-` block at the end of `components.css`. |

## Naming

- `X.dc.html`: a web board. Mac boards (`MacX.dc.html`) are owed until the web direction is approved.
- `XDark.dc.html`: generated dark twins. Never edit one.
- A control that leads to another board is `<a href="Y.dc.html">`, styled as the button (never a `<button>` inside it). In the Design canvas the link moves Play to that board; in a browser it navigates. A board with links or parts has `"is_interactive": true` in `canvas.json`. A control for a board not drawn yet is a `<button data-next="Y">`; `--check` lists those as planned and fails once `Y.dc.html` exists, so it gets turned into a link.

## How a board is written

```html
<script src="./support.js"></script>
<x-dc>
<helmet>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Instrument+Sans:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap">
<link rel="stylesheet" href="./tokens.css">
<link rel="stylesheet" href="./icons.css">
<link rel="stylesheet" href="./components.css">
</helmet>
<div class="board app" style="width: 1440px; height: 900px;"> ... </div>
</x-dc>
```

`support.js` does not exist in this folder: the Design canvas supplies its runtime for that path, and a browser ignores the missing file. The `<helmet>` links apply as written. Colours come only from `var(--token)`, type from `.t-hero .t-display .t-title .t-section .t-item .t-body .t-meta .t-label .t-big`, icons from `icons.css`. Copy markup and class names from an existing board. A third-party logo may carry raw colours inside an element marked `data-keep`.

## Parts

A part's source is its board on the Components page (`Sidebar.dc.html`): no `{{ }}`, no `<sc-for>`, no logic. Its content is the inside of the `<div class="board" style="display: contents;">` root in `<x-dc>`. A board marks where it goes:

```html
<!-- part:Sidebar active="customize" -->
<!-- /part -->
```

The generator rewrites everything between the markers to the part's content. Parameters: `active="x"` puts `aria-current="page"` on the element with `data-item="x"` (styles key off `[aria-current="page"]`); any other `key="v"` replaces the text of the element with `data-slot="key"`. Variants are separate parts; parts may contain markers (cycles fail). Edit the part file, never the inserted copy. Dark twins rewrite every `href="X.dc.html"` to `XDark.dc.html`, inserted parts included. A part's links to boards not drawn yet count as planned.

## Commands

```
python3 design/tools/tokens.py           # write tokens.css, the dark twins and their canvas.json entries
python3 design/tools/tokens.py --check   # write nothing; exit 1 on drift or a rule broken
```

Two rules: boards use tokens only, and a generated file is never edited by hand. Edit the light board (or a part), run the generator, commit the boards and their twins together.

## Looking at the boards

Open a board file in a browser, or `canvas.json` in the Design canvas. Links between boards work in both. No server is needed.

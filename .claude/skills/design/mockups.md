# Mockups first

A visual or UX change starts in `design/mockups/`. Boards are static pictures of one screen state each; there is no clickable prototype to maintain. How screens behave is in [behaviour](behaviour.md); this page is the flow and the checks. Every board is listed in the index in [`design/mockups/README.md`](../../../design/mockups/README.md).

## Boards

- One board per screen state. A web board is `X.dc.html` at 1440 by 900. The macOS app's boards (`MacX.dc.html`) are owed once the web direction is approved; until then no Mac board is drawn and no Mac rule is checked.
- A new state gets its own board only when the screen looks different: empty, error, read-only, budget hit. A hover or a focus ring is not a state.
- No phone boards, except `SessionList` (the v0 app shell on a phone, 390 by 844).
- A board uses only `var(--token)` and the type classes from `design/mockups/tokens.css`. A raw colour is a finding. The one exception is a third-party logo, marked `data-keep`.
- **Third-party logos.** Where a row, card or chip names a product (a connector, an agent, a model provider, a cloud or backend, a sign-in provider), its icon slot shows that product's logo: `<span class="lg lg-snowflake" role="img" aria-label="Snowflake"></span>`, from `logos.css`. Antilog's own concepts (projects, sessions, skills, plugins) keep our glyphs. An agent's logo appears only where the agent is the subject; an agent's chat message carries the Antilog mark. Never put a logo in running text. A board that uses one links `./logos.css` after `components.css`.
- Every brand a board names shows its real logo from a credible source, never a text tile, a tracing or a redrawing. Credible means, in this order: the brand's own published assets (a brand page, an official icon package such as AWS Architecture Icons or Azure's service icons, the brand's own site), then Simple Icons (CC0), then a Wikimedia Commons file whose author is the brand. One SVG per brand goes in `design/logos/`, listed in `design/logos/logos.toml` with where it came from; the generator writes `logos.css`. A near-black mark is `mono = true` and drawn in the ink token, so it shows in dark. A full-colour official icon (gradients, its own tile) is `full = true` and drawn exactly as the file is. A brand whose guidelines ask for a multi-colour mark it publishes as an outline gets `paint`, a CSS paint shown through the mark: the Google G is Google's own sign-in asset with its gradient. The check fails on a text tile (`lgt`). To add a brand, put its SVG in `design/logos/`, add a line to `logos.toml` naming the source, and run the generator.
- **Session boards** (`Session*`, one row on the canvas) are copies of one frame, so a change to the frame is made in all of them. Every one draws the slim header bar (`.hbar`) and the floating one-line composer. `SessionWorking` draws Overview open (the panel was called Details); `SessionOverview` draws the Overview panel in full and `SessionToolCalls` the folded tool-call groups (collapsed, expanded with a failed call, running); `SessionChanges`, `SessionTerminal`, `SessionPreview` and `SessionFiles` each draw one panel; `SessionShare` the popover and `SessionTitleMenu` the title menu; `SessionReadOnly` draws Terminal read-only. `SessionComposerExpanded` draws the grown box (four lines, an attachment, the options row) and `SessionAgentMenu` the agent menu. `SessionBudget`, `SessionCredential`, `SessionRepo` and `SessionConnectorTool` draw the cards, and `SessionFailed` the calm failure line. The boards of the parked judge (Seal, Trials, Win, Delivered, NoWin, Regression, KeepGoing, HandOff, Failover, ExperimentDetail, Receipt, Report, AddValueModel, Evaluations, DatasetHoldout) are deleted; a board that still names one is stale and is fixed or removed. The Mode button reads "Ask" on Home and "Follow policy" on session boards.
- `design/mockups/canvas.json` places every board on a canvas page: Design rules, Components, Web light, Web dark, Archive. The macOS pages are added with the Mac boards.
- `DesignRules.dc.html` shows the look. `Decisions.dc.html` lists the owner's calls, each dated and marked DECIDED or OPEN.
- **Spend boards.** `SettingsBudgets` shows the organization's monthly cap, default budget per session and the projects with their caps and spend. `SettingsApprovals` lists the six card kinds. `Project` shows the project's sessions, budget and members; `Projects` lists projects with sessions and spend this month.
- A control that leads to another board is a link, `<a href="X.dc.html">`, styled as the button (never a `<button>` inside it). In the Design canvas, Play follows it; in a browser it navigates. A control for a board not drawn yet is `<button data-next="X">`, and the check asks for a link once `X` exists.
- **Shared chrome is a part**, a plain static board on the Components page that the generator inserts into every board that marks a place for it:
  - Parts: `Sidebar` (inserts `SidebarTop`, `SidebarRows` and `SidebarFoot` with nested markers), `SidebarEmpty` (no rows, for an organization with no session) and `SettingsRail`.
  - A board marks a place with `<!-- part:Sidebar active="projects" --><!-- /part -->`. `active` sets `aria-current="page"` on the `data-item` (it reaches the rows inside nested parts); other keys fill `data-slot` text.
  - Change the sidebar rows in `SidebarRows.dc.html` and the rest in `Sidebar.dc.html`, never inside a board; the check names the part file to edit. The top bar varies per screen, so it is board content.
  - The nav rows carry glyphs from `icons.css` (Projects the folder, Customize the sliders). A project heading folds its sessions (`.ph`, with a chevron); each session is a row under it (`.rec.ses`).
  - The session panel styles are `.wsp`, `.chat`, `.pane`, `.pane-h`, `.dp-sec`, `.dp-row`, `.diff` and `.term` in `components.css`.

## Authored and generated

| Authored (edit these) | Generated (never edit) |
| --- | --- |
| `design/mockups/X.dc.html`, the light boards | `design/mockups/XDark.dc.html`, the dark twin of each light board (same board with `data-theme="dark"`) |
| `design/mockups/canvas.json`, the light entries | the dark entries in `canvas.json` |
| `design/tokens.toml`, the values | `design/mockups/tokens.css` |
| `design/icon/`, the icons | |
| `design/logos/` (third-party SVGs and `logos.toml`) | `design/mockups/logos.css` |
| (target) the words catalogue | (target) the web app's CSS and TypeScript words, the Mac app's Swift colours and words |

## The flow

1. Edit the light board, or add one.
2. Add its entry to `canvas.json` on the light page.
3. Change `design/tokens.toml` only when a value changes. Never put a value in a board.
4. Make every control that leads to another board a link, and mark places for shared chrome with part markers.
5. Run `python3 design/tools/tokens.py`. It fills the part markers, and writes `tokens.css`, every dark twin and its `canvas.json` entry.
6. Commit the light boards, the dark twins and the generated files together.

## The checks

`python3 design/tools/tokens.py --check` fails on drift (a generated file that is not what the generator writes), a raw colour in a board or stylesheet, part content that is not what the generator writes (it names the part file to edit), an unknown or unclosed part marker, any `<dc-import>`, `{{`, `<sc-for>` or `<sc-if>`, a link that names no board, a `<button>` inside a link, a board with links or parts missing `is_interactive`, a board missing from `canvas.json`, and DesignRules swatches that do not match `tokens.toml`. Run it before calling a design change done.

It moves to a Rust `cargo xtask design` with the first code (target), which will also write the web app's CSS and, with the Mac boards, the Mac app's Swift colours. The check then joins the gate; see [land](../land/SKILL.md).

## Looking at boards

Open the file in a browser; no server is needed. Links between boards are the clickable flow; a flow that needs live state is drawn as more boards, not scripted.

## Not here

Behaviour and wording rules are in [behaviour](behaviour.md) and [wording](wording.md). Values are in `design/tokens.toml`, not in any doc.

# Mockups first

A visual or UX change starts in `design/mockups/`. Boards are static pictures of one screen state each; there is no clickable prototype to maintain. How screens behave is in [behaviour](behaviour.md); this page is the flow and the checks.

## Boards

- One board per screen state. A web board is `X.dc.html` at 1440 by 900. A Mac board is `MacX.dc.html` and exists only where the Mac app differs from the web app.
- A new state gets its own board only when the screen looks different: empty, error, read-only, holdout, budget hit. A hover or a focus ring is not a state.
- No phone boards for now.
- A board uses only `var(--token)` and the type classes from `design/mockups/tokens.css`. A raw colour is a finding. The one exception is a third-party logo, marked `data-keep`.
- `design/mockups/canvas.json` places every board on a canvas page: Design rules, Components, Web light, Web dark, macOS light, macOS dark, Archive.
- `DesignRules.dc.html` shows the look. `Decisions.dc.html` lists the owner's calls, each dated and marked DECIDED or OPEN.
- A control that leads to another board is a link, `<a href="X.dc.html">`, styled as the button (never a `<button>` inside it). In the Design canvas, Play follows it; in a browser it navigates. A control for a board not drawn yet is `<button data-next="X">`, and the check asks for a link once `X` exists.
- Shared chrome is a part: `Sidebar`, `SidebarEmpty` (the same chrome with no rows, for an organization with nothing yet), `MacSidebar`, `SettingsRail`, `SidebarTop` (brand, nav, New experiment), `SidebarFoot` (the user menu) and `SidebarRows` (`SidebarTop` plus the projects and pinned rows; `Sidebar` and `MacSidebar` both insert it with a nested marker; `MacSidebar` writes `variant="mac"`, which sets each row's `href` to its `data-mac` board), boards on the Components page, plain static files. A board marks a place with `<!-- part:Sidebar active="connectors" --><!-- /part -->` and the generator fills it (`active` sets `aria-current="page"` on the `data-item`; other keys fill `data-slot` text). Parts may nest, and `active` on a board's marker reaches the rows inside. Change the sidebar rows in `SidebarRows.dc.html`, the rest in `Sidebar.dc.html`, never inside a board; the check names the part file to edit. The top bar and macOS toolbar vary per screen, so they are board content.

## Authored and generated

| Authored (edit these) | Generated (never edit) |
| --- | --- |
| `design/mockups/X.dc.html` and `MacX.dc.html`, the light boards | `design/mockups/XDark.dc.html`, the dark twin of each light board (same board with `data-theme="dark"`) |
| `design/mockups/canvas.json`, the light entries | the dark entries in `canvas.json` |
| `design/tokens.toml`, the values | `design/mockups/tokens.css` |
| `design/icon/`, the icons | |
| (target) the words catalogue | (target) the web app's CSS and TypeScript words, the Mac app's Swift colours and words |

## The flow

1. Edit the light board, or add one.
2. Add its entry to `canvas.json` on the light page.
3. Draw a Mac board only where the Mac differs from the web.
4. Change `design/tokens.toml` only when a value changes. Never put a value in a board.
5. Make every control that leads to another board a link, and mark places for shared chrome with part markers.
6. Run `python3 design/tools/tokens.py`. It fills the part markers, and writes `tokens.css`, every dark twin and its `canvas.json` entry.
7. Commit the light boards, the dark twins and the generated files together.

## The checks

`python3 design/tools/tokens.py --check` fails on drift (a generated file that is not what the generator writes), a raw colour in a board or stylesheet, part content that is not what the generator writes (it names the part file to edit), an unknown or unclosed part marker, any `<dc-import>`, `{{`, `<sc-for>` or `<sc-if>`, a link that names no board, a `<button>` inside a link, a board with links or parts missing `is_interactive`, a board missing from `canvas.json`, and DesignRules swatches that do not match `tokens.toml`. Run it before calling a design change done.

It moves to a Rust `cargo xtask design` with the first code (target), which will also write the web app's CSS and the Mac app's Swift colours. The check then joins the gate; see [land](../land/SKILL.md).

## Looking at boards

Open the file in a browser; no server is needed. Links between boards are the clickable flow; a flow that needs live state is drawn as more boards, not scripted.

## Not here

Behaviour and wording rules are in [behaviour](behaviour.md) and [wording](wording.md). Values are in `design/tokens.toml`, not in any doc.

# Mockups

One board per screen state, for the web app and the macOS app (target). There is no prototype to maintain: the clickable view is the boards themselves, linked to each other.

## What is here

- `*.dc.html`: boards, each one fixed-size root element. Web boards are 1440 by 900; the rules boards are taller.
- Components, imported by boards with `<dc-import name="Sidebar" active="Connectors">`: `Sidebar` (web, and the macOS variant `variant="mac"`), `Topbar`, `SettingsRail`, `MacSidebar`, `MacToolbar`. A component takes props (`active`, `trail`, `title`) and renders inline, so it inherits the theme of the board that imports it.
- `canvas.json`: the pages and where each board sits. Pages: Design rules, Components, Web light, Web dark, macOS light, macOS dark, Archive. Rows are 1100px apart and each has a title note: 0 Rules, 1 Sign in, 2 Start, 3 Thread, 4 Results, 5 History, 6 Customize, 7 Connectors, 8 Infrastructure, 9 Organization. Boards in a row are 1440 wide with 80px gaps.
- `tokens.css` (generated), `icons.css` and `components.css` (authored, values only through `var()`), `support.js` (the canvas runtime, a copy of `design/web/support.js`).
- `DesignRules.dc.html` shows every token in light and dark with its contrast ratio, the type scale, radii and button kinds. `Decisions.dc.html` lists the owner's calls, DECIDED with a date or OPEN.
- Values live in `design/tokens.toml` and nowhere else.

## Naming

- `X.dc.html`: a web board. `MacX.dc.html`: a macOS board, drawn only where the Mac differs from the web.
- `XDark.dc.html` and `MacXDark.dc.html`: generated dark twins. Never edit one.
- A control that leads to another board is `<a href="Y.dc.html">`, styled as the button (never a `<button>` inside it). In the Design canvas the link moves Play to that board; in a browser it navigates. A board with links or imports has `"is_interactive": true` in `canvas.json`. A control for a board not drawn yet is a `<button data-next="Y">`; `--check` lists those as planned and fails once `Y.dc.html` exists, so it gets turned into a link.

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

The `<helmet>` links apply as written. Colours come only from `var(--token)`, type from `.t-hero .t-display .t-title .t-section .t-item .t-body .t-meta .t-label .t-big`, icons from `icons.css`. Copy markup and class names from an existing board. A third-party logo may carry raw colours inside an element marked `data-keep`.

## Commands

```
python3 design/tools/tokens.py           # write tokens.css, the dark twins and their canvas.json entries
python3 design/tools/tokens.py --check   # write nothing; exit 1 on drift or a rule broken
```

Two rules: boards use tokens only, and a generated file is never edited by hand. Edit the light board, run the generator, commit the light board and its twin together.

## Looking at the boards

Components are fetched at runtime, so a board does not render from `file://`. Serve the folder and open a board:

```
python3 -m http.server -d design/mockups
# then http://localhost:8000/ThreadRunning.dc.html
```

or open `canvas.json` in the Design canvas. Links between boards work in both.

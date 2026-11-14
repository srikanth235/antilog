# Ratchet design system

The look is Midnight Brass: calm, exact, warm. A dark instrument panel with one brass accent, and the same palette on warm paper in light mode. The values below are the ones in `design/web/src/style.css` (the `.app.dark` and `.app.light` rules). If they differ, the stylesheet wins and this file is wrong.

## Accent

Brass is the brand and the only accent. People do not choose a colour. Brass was checked for contrast in both modes.

## Colour

| Token | Dark | Light | Used for |
| --- | --- | --- | --- |
| `canvas` | `#0B1019` | `#FBFAF6` | page background |
| `side` | `#080C14` | `#F4F2EB` | sidebar |
| `raised` | `#101724` | `#FFFFFF` | cards, popovers |
| `sunk` | `#0E1420` | `#F7F5EF` | inset areas |
| `line` | `#182031` | `#ECE8DD` | dividers |
| `line2` | `#2A3447` | `#D6D0C1` | outlines of controls you can click or type in |
| `ink` | `#ECEFF4` | `#15181F` | body text |
| `ink2` | `#A7B1C2` | `#4A5363` | labels |
| `ink3` | `#8291A9` | `#5C6676` | meta text |
| `accent` | `#D9B26F` | `#8A5E0B` | accent text and marks |
| `fill` | `#D9B26F` | `#15181F` | primary button fill (`onFill` is the text on it) |
| `good` | `#7FD1AE` | `#187250` | success |
| `infra` | `#8DB8E8` | `#2A5F8F` | infrastructure |
| `warn` | `#E9B85C` | `#8A5A00` | waiting, caution |
| `bad` | `#E58C8C` | `#B4414A` | failure |

Rules:

- Screens read these tokens only. Never write a raw colour in a screen.
- Text meets 4.5:1 in both modes. A control's outline meets 3:1 (WCAG 1.4.11), so it uses `line2`, not `line`.
- Status is never colour alone. A shape or a word comes with it.
- Light and dark are both designed. A change is checked in both.

## The three areas

Each area keeps one colour in diagrams and navigation (target). Optimization is brass (`accent`), Context is green (`good`), Infrastructure is blue (`infra`). The organization is slate. The prototype has no separate slate token yet (target). Never use these colours for anything else, and never rely on colour alone to say which area a thing belongs to.

## Type

Two families: Instrument Sans for everything (system fonts as the fallback) and JetBrains Mono for code, small uppercase labels and inline mono values. There is no serif. Big numbers are Instrument Sans, not mono. Each role has a class in `style.css`; use the class, not an inline size.

| Role | Class | Size / weight | Letter-spacing | Line-height | Use |
|---|---|---|---|---|---|
| Display | `.t-display`, `.h1` | 40 / 600 | -0.025em | 1.1 | Page titles |
| Title | `.t-title` | 28 / 600 | -0.02em | 1.15 | Experiment title and its rename input, detail page titles, banner and empty-state headings |
| Section | `.t-section`, `.h2` | 18 / 600 | -0.01em | 1.3 | Section headings, project card names |
| Item | `.t-item` | 15 / 500 | normal | 1.35 | Names in lists, tables and rows, and notes from people (at weight 400) |
| Body | (default) | 14 / 400 | normal | 1.45 | Text |
| Meta | `.sm`, `.xs` | 12 to 12.5 / 400 | normal | | Descriptions under item names |
| Label | `.lbl` | JetBrains Mono 10.5 to 11 / 500, uppercase | 0.06em to 0.1em | | Small labels |
| Number | `.big` | 40 / 600, tabular figures | -0.02em | 1 | Figures in tiles; the result headline keeps 56px |

## Shape

Radii are 6, 10 and 14px (`--r1`, `--r2`, `--r3`). Icons are a single sprite in `style.css` (`.i-*`, `.g-*`, `.st-*` classes).

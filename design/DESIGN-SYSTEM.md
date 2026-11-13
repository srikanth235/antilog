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

Instrument Sans for text (system fonts as the fallback), Instrument Serif for titles and big numbers only, JetBrains Mono for numbers, code and small labels. Body text is 14px.

## Shape

Radii are 6, 10 and 14px (`--r1`, `--r2`, `--r3`). Icons are a single sprite in `style.css` (`.i-*`, `.g-*`, `.st-*` classes).

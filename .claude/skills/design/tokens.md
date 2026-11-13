# Tokens

Do not copy token tables into docs or code. Link them.

## Where the tokens are

| What | Where |
| --- | --- |
| The web prototype's tokens (the source of truth) | `design/web/src/style.css`, the `.app.dark` and `.app.light` rules |
| The look, colour, type and which token to use when | [`design/DESIGN-SYSTEM.md`](../../../design/DESIGN-SYSTEM.md) |
| What is in `design/` | [`design/README.md`](../../../design/README.md) |

Tokens are named by meaning (`canvas`, `raised`, `line2`, `ink`, `accent`). Light mode redefines the same names. Never write a raw colour, radius, size or duration in a screen.

## Rules

- Light and dark are both designed. A change is checked in both.
- Brass is the brand and the only accent. People do not choose a colour.
- Text meets contrast in both modes.
- A control's outline uses `line2`, not the `line` divider.

## The three basics have fixed colours (target)

In diagrams and in navigation each basic keeps one colour, so a reader knows where they are at a glance.

| Area | Colour |
| --- | --- |
| Optimization | brass (the accent) |
| Context | green |
| Infrastructure | blue |
| Organization | slate |

The prototype has `accent` (brass), `good` (green) and `infra` (blue). Slate for the organization is not a named token yet (target). Add it to both modes in `style.css` and `DESIGN-SYSTEM.md` in one change, with a contrast check. Never use these colours for anything else, and never rely on colour alone to say which area a thing belongs to.

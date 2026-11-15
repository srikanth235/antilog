# The web platform in Ratchet

Read this before writing HTML, CSS or browser-side TypeScript in the web client (target) or a design board. Then find the guide for the job in [general/INDEX.md](general/INDEX.md) and read it **before** writing the code: models default to older patterns, and these guides say what the platform does natively now.

`general/` is a copy of 110 guides from [Modern Web Guidance](https://github.com/GoogleChrome/modern-web-guidance) (Chrome, Edge, Firefox and MDN teams; Apache-2.0, [licence](licenses/modern-web-guidance-LICENSE)). It is the subset a desktop web app for engineers needs; the list and the reasons for what is left out are in [sync.py](sync.py). Each guide ends with the browser support of the features it uses.

## How to use it

1. Name the job as a capability, not a component of ours: "light dismiss dialog", "sortable table", "copy button", "scroll shadows". Find it in the index. For a broad change, also read the discipline guide: `html`, `css`, `forms`, `accessibility`, `performance`, `security`.
2. Read the guide in full and follow it, adapted to React and to the rules below.
3. Before calling the change done, check the code against the guide: its rules, its fallbacks, its accessibility notes.

The upstream guides tell agents to run `npx modern-web-guidance`. Here the guides are local files; do not run the CLI.

## Where Ratchet's rules win

A guide is advice from outside; these are decisions. When they disagree, follow this page, then [../typescript/ratchet.md](../typescript/ratchet.md) and the [design](../../design/SKILL.md) skill.

- **Browser support.** The client supports the current and previous major version of Chrome, Edge, Firefox and Safari on desktop. Baseline Widely available features need no fallback. Baseline Newly available features are allowed with the fallback the guide gives, feature-detected, adding no dependency; a polyfill package is a product call ([../libraries.md](../libraries.md)). Features not yet Baseline only as progressive enhancement: the screen must work and read correctly without them.
- **Tokens, not values.** Guides write colours, sizes and fonts inline to stay self-contained. Ours come from `design/tokens.toml` through the generated tokens CSS. Take the technique, never the values.
- **Themes.** Light and dark are keyed on `[data-theme="dark"]`, and the tokens CSS sets `color-scheme` for each, as the dark-mode guide asks. Do not add `light-dark()` tokens or a second `prefers-color-scheme` palette in a component; a theme change is a change to `tokens.toml`.
- **Components.** Radix primitives are the chosen base for dialog, menu, popover, tabs, tooltip and select ([../typescript/ratchet.md](../typescript/ratchet.md)). Where Radix covers the component, read the guide for its accessibility and styling rules and apply them to the Radix part; do not swap in the native element on your own. Where Radix does not (copy button, progress, spinner, toasts, tables, sticky headers, scroll shadows, accordions with `details`), build it natively as the guide says.
- **Icons.** One mask sprite, coloured by `currentColor`, as the icons guide's CSS icon system describes. Add an icon to the sprite; do not add an icon font or a second set.
- **Words.** Example labels and messages in a guide are placeholders. On-screen words come from the catalogue and follow `docs/GLOSSARY.md` and the design skill's wording rules.
- **Not ours.** Sign-in is OpenID Connect in the browser, so password, passkey and autofill sign-in guides are not taken. The client is client-rendered, so skip server rendering and streaming advice. No analytics beacons: the server records runs, not visitors.

## Updating the guides

```bash
git clone --depth 1 https://github.com/GoogleChrome/modern-web-guidance /tmp/mwg
python3 .claude/skills/stack/web/sync.py /tmp/mwg      # copies the listed guides, rewrites the index
python3 .claude/skills/stack/web/sync.py --check       # offline: files match the list
```

Never edit `general/` by hand. To take or drop a guide, edit `GUIDES` in `sync.py` and sync. The only change the sync makes is turning "run the CLI to retrieve X" into a link to X. Read the diff of an update like any other change: a guide that now contradicts a rule above needs that rule restated here, not the guide edited.

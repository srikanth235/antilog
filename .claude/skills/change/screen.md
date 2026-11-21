# A screen

Mockup first, then the web app and, later, the macOS app (target). The web app is in `web/`.

## 1. The mockup

Start with a board in `design/mockups/`: edit the light board, change `design/tokens.toml` only if a value moves, and run `python3 design/tools/tokens.py`, then `python3 design/tools/tokens.py --check`. The Mac boards are removed for now and are owed once the web direction is approved. See [mockups](../design/mockups.md) and [`design/README.md`](../../../design/README.md). Behaviour and words: [design](../design/SKILL.md).

## 2. The web app

A screen is built for the web client in `web/src/features/`. It is React and TypeScript over the same API and the same behaviour and words, with the board's class names from `design/mockups/components.css`, words from `web/src/words.ts` and its route in `web/src/router.tsx`. What the API does not serve yet is a typed stub marked "(target)", listed in the report. See [stack/typescript/antilog.md](../stack/typescript/antilog.md). If the screen needs new data, do [api-endpoint.md](api-endpoint.md) first.

## 3. The macOS app (target)

SwiftUI, macOS only; Windows and Linux use the installed web app. Its boards are owed. It shares behaviour, words and element ids with the web app, never pixels, and reads colours and words generated from the same sources. See [stack/swift/antilog.md](../stack/swift/antilog.md).

A new kind of thing under Customize (a tab with Yours and Discover) uses the one DetailPage layout and gets no page style of its own. Its boards follow the Customize set: a list, an empty state, Discover, a detail page and an Add page.

Build to the boards' shared patterns: the fixed sidebar and main area (with projects as headings, each listing its sessions), the composer, cards as the only place structured output appears, one popover, one palette for search. The page types are Session, ListPage, DetailPage, Dialog, SettingsSection and SignIn. They are in [design behaviour](../design/behaviour.md); the words are in [wording](../design/wording.md) and the glossary's "On screen" column. A new card kind is also a change to [tool.md](tool.md).

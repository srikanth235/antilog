# A screen

Mockup first, then the web app and, later, the macOS app (target). There is no client app yet.

## 1. The mockup

Start with a board in `design/mockups/`: edit the light board, change `design/tokens.toml` only if a value moves, and run `python3 design/tools/tokens.py`, then `python3 design/tools/tokens.py --check`. The Mac boards are removed for now and are owed once the web direction is approved. See [mockups](../design/mockups.md) and [`design/README.md`](../../../design/README.md). Behaviour and words: [design](../design/SKILL.md).

## 2. The web app (target)

(target) A screen is built for the web client. It is React and TypeScript over the same API and the same behaviour and words. It does not exist yet. See [stack/typescript/ratchet.md](../stack/typescript/ratchet.md). If the screen needs new data, do [api-endpoint.md](api-endpoint.md) first.

## 3. The macOS app (target)

SwiftUI, macOS only; Windows and Linux use the installed web app. Its boards are owed. It shares behaviour, words and element ids with the web app, never pixels, and reads colours and words generated from the same sources. See [stack/swift/ratchet.md](../stack/swift/ratchet.md).

Build to the boards' shared patterns: the fixed sidebar and main area, the composer, cards as the only place structured output appears, one popover, one palette for search. The page types are Session, ListPage, DetailPage, Dialog, SettingsSection and SignIn. They are in [design behaviour](../design/behaviour.md); the words are in [wording](../design/wording.md) and the glossary's "On screen" column. A new card kind is also a change to [tool.md](tool.md).

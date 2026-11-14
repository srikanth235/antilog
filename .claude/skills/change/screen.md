# A screen

Mockup first, then the web app and, where it applies, the macOS app (target). There is no client app yet.

## 1. The mockup

Start with a board in `design/mockups/`: edit the light board, change `design/tokens.toml` only if a value moves, and run `python3 design/tools/tokens.py`, then `python3 design/tools/tokens.py --check`. A Mac board only where the Mac differs. See [mockups](../design/mockups.md) and [`design/README.md`](../../../design/README.md). Behaviour and words: [design](../design/SKILL.md).

## 2. The web app (target)

(target) A screen is built for the web client. It is React and TypeScript over the same API and the same behaviour and words. It does not exist yet. See [stack/typescript.md](../stack/typescript.md). If the screen needs new data, do [api-endpoint.md](api-endpoint.md) first.

## 3. The macOS app (target)

SwiftUI, macOS only; Windows and Linux use the web app. It shares behaviour, words and element ids with the web app, never pixels, and reads colours and words generated from the same sources. See [stack/swift.md](../stack/swift.md).

Build to the boards' shared patterns: one detail-page layout for every kind under Customize, one popover, one palette for global search. The page types are in [design behaviour](../design/behaviour.md); the words are in [wording](../design/wording.md) and the glossary's "On screen" column.

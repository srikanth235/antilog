---
name: design
description: >-
  What does it look like and say? Use for any visual or UX change, a mockup board, light and dark, a design token or icon, a screen's behaviour (page types, navigation, states, the holdout marking, colour, accessibility), the macOS app (target, boards owed), or any word a user reads (strings, the glossary).
---

# Design

Every visual or UX change starts in `design/mockups/`, as a board. The look is Midnight Brass: a calm, exact dark instrument panel with one brass accent, and the same palette on warm paper, both designed from day zero. Values live in `design/tokens.toml` and nowhere else. Behaviour and words live here. The web app and the macOS app (target; its boards are owed) share behaviour, words and element ids, never pixels. Antilog is a chat and a judge: the screen is an experiment page (the chat of its lead session), an experiment detail page, the Projects and Project pages, one Customize page (Evaluations, Skills, Connectors, Agents, Plugins) and a settings dialog, over one sidebar that groups experiments under their project and nests an open experiment's sessions.

| If you are doing | Read |
| --- | --- |
| Changing a board, a token or an icon; running the generator | [mockups.md](mockups.md) |
| Deciding how a screen behaves: page types, the sidebar, Projects, Customize, navigation, states, the holdout, colour and type, clients, accessibility | [behaviour.md](behaviour.md) |
| Writing or changing a word a user reads | [wording.md](wording.md) |
| The look at a glance, and the owner's dated calls | `design/mockups/DesignRules.dc.html`, `design/mockups/Decisions.dc.html` |
| A value (colour, size, radius, type) | `design/tokens.toml` |
| What the user reads for a code name | the "On screen" column of [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md) |
| How the platform does a pattern natively (dialog, popover, select, table, sticky header, scroll shadows, motion, focus) | [stack/web/antilog.md](../stack/web/antilog.md) and its guide index |
| Building a screen in the web app (and later the macOS app, target) | [change/screen.md](../change/screen.md) |
| Deciding what the product should do | [product](../product/SKILL.md) |

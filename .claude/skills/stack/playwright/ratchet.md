# Playwright in Ratchet

`general/` is a copy of Microsoft's `playwright-cli` skill from [microsoft/playwright-cli](https://github.com/microsoft/playwright-cli) (Apache-2.0, [licence](licenses/microsoft-playwright-cli-LICENSE)). Start at [general/INDEX.md](general/INDEX.md). The list is in [sync.py](sync.py).

**This page wins over `general/`.**

## Two different jobs

- **Driving a browser while you work.** Use `playwright-cli` to open the web app or a design board, click through a flow, look at a state, take a screenshot to check your own work. This is what `general/` covers.
- **The test suite.** Tests are `@playwright/test` (version in [../libraries.md](../libraries.md)), written and run as [../../land/tests.md](../../land/tests.md) and [../../land/checks.md](../../land/checks.md) say. Do not turn a `playwright-cli` session into a committed test by pasting its output. Write the test in the suite's style.

## Rules

- **Never run `playwright install`.** In cloud sessions Chromium is already installed. Use it. Elsewhere, say the browser was not available.
- **Screenshots, traces and videos are scratch.** Keep them outside the repo or delete them. The pre-commit check rejects images outside `design/icon`, and a board is HTML, not a picture ([../../design/mockups.md](../../design/mockups.md)).
- **Stay on Ratchet's own pages.** Point it at the local web app or a board on disk. Do not sign in to a customer system or paste a token into a page; state saved by the skill (cookies, storage) holds credentials, so keep it out of the repo.
- **Check what you changed.** Open the screen in light and dark, below 720px wide, and with the keyboard, as [../../design/behaviour.md](../../design/behaviour.md) says. Report what you looked at; do not call a flow verified from a screenshot alone.

## Updating general/

```bash
git clone --depth 1 https://github.com/microsoft/playwright-cli /tmp/playwright-cli
python3 .claude/skills/stack/playwright/sync.py /tmp/playwright-cli
python3 .claude/skills/stack/playwright/sync.py --check
```

Never edit `general/` by hand.

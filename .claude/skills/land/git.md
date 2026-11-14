# Git

- One branch per change. Never push to the default branch.
- Commit only when asked. Open a pull request only when asked.
- Do not force-push a branch someone else is working on.
- Never skip, disable or delete a test, hook or check to get green. Fix the code.
- Do not commit generated files by hand. Regenerate them: `design/mockups/tokens.css` and the dark twins (from `python3 design/tools/tokens.py`), and (target) the OpenAPI document and the generated TypeScript client.
- Never commit a secret, a token or a `.env` file.
- No model names in commits, code, docs or pull request text.

## Commit subjects

A short line, prefixed by the work item or area, then a colon, then lower case text with no full stop: `design: sign-in board for single sign-on`, `server: audit entry on token revoke`. A prefix, a colon, what changed and why in one line. Read `git log --oneline -20` for the current habit. Add trailers only when the session's instructions say to.

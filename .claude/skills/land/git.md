# Git

- One branch per change. Never push to the default branch.
- Commit only when asked. Open a pull request only when asked.
- Do not force-push a branch someone else is working on.
- Never skip, disable or delete a test, hook or check to get green. Fix the code.
- Do not commit generated files by hand. Regenerate them: `api/openapi.yaml`, `design/web/Main.dc.html` (from `design/web/src/assemble.sh`).
- Never commit a secret, a token or a `.env` file.
- `Cargo.lock` is not committed; leave it out.

## Commit subjects

Read from `git log --oneline -20`. The subject is a short imperative or descriptive line, prefixed by the work item or area, then a colon, then lower case text with no full stop: `F6: contrast check calibrated on real pictures`, `P5b: steering field height`, `CI: cache the Rust build`, `Merge F6: app polish after P5b`. Follow that: a prefix, a colon, what changed and why in one line. There is no pull request number suffix.

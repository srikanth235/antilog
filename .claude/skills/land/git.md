# Git

- One branch per change. Never push to the default branch.
- Commit only when asked. Open a pull request only when asked.
- Do not force-push a branch someone else is working on.
- Never skip, disable or delete a test, hook or check to get green. Fix the code.
- Do not commit generated files by hand. Regenerate them: `design/mockups/tokens.css` and the dark twins (from `python3 design/tools/tokens.py`), and the OpenAPI document (`openapi/antilog.json`) and the generated TypeScript client (`web/src/api/schema.d.ts`).
- Never commit a secret, a token or a `.env` file.
- **Hooks.** `.githooks/pre-commit` blocks secrets, build output, files over 1 MB, em dashes, model names and a failing design check; `.githooks/commit-msg` checks the subject. Every Claude Code session turns them on at start (`.claude/settings.json`); anywhere else run `git config core.hooksPath .githooks` once. CI runs both on every push ([ci.md](ci.md)). Keep hooks under a second.
- No model names in commits, code, docs or pull request text. The one exception is vendored upstream data, the price list in `crates/antilog-proxy/data/` (owner-approved 20 Nov), which the hook skips like other upstream text.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `type(scope): summary`.

- **Type**, one of: `feat` (something a user can see or do), `fix`, `docs` (skills, glossary, READMEs), `refactor`, `perf`, `test`, `build` (dependencies, toolchain, images), `ci`, `chore`, `revert`.
- **Scope**, the area touched, optional: `design`, `skills`, `glossary`, `server`, `platform`, `cli`, `web`, `mac`, `db`, `infra`. Leave it out when a change spans several.
- **Summary**: imperative, lower case, no full stop, at most 72 characters for the whole line: `feat(design): draw the empty states`, `fix(server): audit a refused token revoke`.
- **Breaking change**: `!` before the colon, and a `BREAKING CHANGE:` footer saying what to do: `feat(server)!: drop the v0 runs route`.
- **Body**, when the why is not obvious: a blank line, then plain sentences, wrapped at 72.
- A pull request title follows the same form.
- **Enforced** by `.githooks/commit-msg`, which rejects any other subject (merge, revert and fixup commits pass), and again in CI.

Add trailers only when the session's instructions say to.

# Git

- One branch per change. Never push to the default branch.
- Commit only when asked. Open a pull request only when asked.
- Do not force-push a branch someone else is working on.
- Never skip, disable or delete a test, hook or check to get green. Fix the code.
- Do not commit generated files by hand. Regenerate them: `design/mockups/tokens.css` and the dark twins (from `python3 design/tools/tokens.py`), and (target) the OpenAPI document and the generated TypeScript client.
- Never commit a secret, a token or a `.env` file.
- No model names in commits, code, docs or pull request text.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `type(scope): summary`.

- **Type**, one of: `feat` (something a user can see or do), `fix`, `docs` (skills, glossary, READMEs), `refactor`, `perf`, `test`, `build` (dependencies, toolchain, images), `ci`, `chore`, `revert`.
- **Scope**, the area touched, optional: `design`, `skills`, `glossary`, `server`, `runner`, `cli`, `web`, `mac`, `db`, `infra`. Leave it out when a change spans several.
- **Summary**: imperative, lower case, no full stop, at most 72 characters for the whole line: `feat(design): draw the empty states`, `fix(server): audit a refused token revoke`.
- **Breaking change**: `!` before the colon, and a `BREAKING CHANGE:` footer saying what to do: `feat(server)!: drop the v0 runs route`.
- **Body**, when the why is not obvious: a blank line, then plain sentences, wrapped at 72.
- A pull request title follows the same form.

Add trailers only when the session's instructions say to.

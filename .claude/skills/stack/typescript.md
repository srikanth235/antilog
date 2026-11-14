# TypeScript

## What exists today

TypeScript is not part of the product. It is the three demo repositories and the infra tests, left after the cut-over to Rust (`docs/CUTOVER.md`).

- `pnpm-workspace.yaml` lists `infra/tests` and `examples/*`. `package.json` (pnpm 10.28, Node 22 or newer) has `build`, `typecheck` and `test`, each `pnpm -r`.
- `examples/demo-search`, `examples/demo-forecast`, `examples/demo-cli`: scripted demos the simulation runs. `templates/` holds evaluator templates written into a user's repository.
- `infra/tests` is a vitest suite over the OpenTofu policies and images.
- Shared config: `tsconfig.base.json`. Tools: `typescript`, `vitest`, `yaml`.

Check:

```bash
pnpm install --frozen-lockfile
pnpm build && pnpm typecheck && pnpm test
```

Fix the code, not the check. Do not add TypeScript to the product without the owner.

## The web app stack (target)

(target) A desktop-only web app (PWA) serves Windows and Linux. Nothing of it exists. The chosen stack:

| Job | Choice |
| --- | --- |
| UI and language | React, TypeScript |
| Build | Vite |
| Packages | pnpm |
| API client | `openapi-typescript` with `openapi-fetch`, generated from `api/openapi.yaml` |
| Server state and routing | TanStack Query and TanStack Router |
| Components | Radix primitives with CSS modules. No Tailwind. |
| Unit tests | Vitest |
| End to end | Playwright |
| Lint and format | Biome |

Versions are in [libraries.md](libraries.md).

Tokens come from the one design system; see [design/tokens.md](../design/tokens.md). A library outside this list is a product call; see [libraries.md](libraries.md).

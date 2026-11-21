# TypeScript and React in Antilog

The web client is in `web/`; the owner-approved boards are in `design/mockups/`. This page says how the client is written.

## Workflow and done criteria

```bash
pnpm install --frozen-lockfile
pnpm build && pnpm typecheck && pnpm test      # the check
pnpm lint                                       # Biome: format and lint
pnpm e2e                                        # Playwright, against a built client and the server on its fakes
```

**Done** when build, typecheck, lint and tests are green and the generated files are up to date (see below). Fix the code, not the check: do not add `// @ts-ignore`, `any`, a lint suppression or a snapshot update to get green.

The compiler is `strict` with `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes` and `verbatimModuleSyntax`. Node and pnpm versions are in [libraries.md](../libraries.md). Use `pnpm`, not `npm` or `npx`.

## The stack (target)

| Job | Choice |
| --- | --- |
| UI and language | React 19, TypeScript |
| Build | Vite |
| Packages | pnpm |
| API client | `openapi-typescript` with `openapi-fetch`, generated from the API's OpenAPI document |
| Server state and routing | TanStack Query and TanStack Router |
| Components | Base UI (`@base-ui/react`) with CSS modules. No Tailwind. |
| Unit and component tests | Vitest (with Testing Library for components) |
| End to end | Playwright |
| Lint and format | Biome |

Versions are in [libraries.md](../libraries.md). A library outside this list is a product call: ask ([libraries.md](../libraries.md)). It is a client-rendered single page app: no server-side rendering and no server components, so skip any advice that assumes Next.js.

## Constraints

- **Generated files are outputs; never edit them.** There are three: the API client types (from the OpenAPI document), the tokens CSS (from `design/tokens.toml`) and the words module (from the one catalogue, target). Change the source, run the generator, commit the result, and let CI fail on drift. A hand edit to a generated file is a finding.
- **The client calls the API only through the generated client** (`openapi-fetch`) wrapped in TanStack Query hooks. No hand-written `fetch` to the server, no copied response types. A response is typed by the generated schema, and a change to the API shows as a type error here.
- **Tokens, not values.** Colours, type, spacing, radius and motion come from the generated tokens CSS (`var(--...)`) and its type classes, in light and dark. No hex or `rgb()` in a stylesheet or a style prop, no parallel variable table, no font-family override, no second dark theme. The themes are `light` and `dark`, keyed on `[data-theme="dark"]`; with no attribute the page follows the system. A value that is missing is added to `design/tokens.toml` by the owner of the design, not invented in a component. See [design mockups](../../design/mockups.md).
- **Words come from one catalogue** (`web/src/words.ts`, one typed module; the shared catalogue with the Mac app is target). No user-facing string literal in a component: look it up by key. This keeps the web app and the Mac app saying the same thing, and keeps the vocabulary right ("session", "project", "Raise budget"; see `docs/GLOSSARY.md`). A literal string in JSX is a finding. Test ids and ARIA names follow the element ids shared with the Mac app ([../swift/antilog.md](../swift/antilog.md)).
- **CSS modules, no Tailwind.** One `.module.css` per component, class names local, styles written against tokens. Base UI gives behaviour and accessibility (dialog, alert dialog, menu, context menu, popover, tooltip, tabs, select, combobox and autocomplete, checkbox, radio, switch, slider, number field, meter, progress, toast, field and fieldset, scroll area, and a drawer where a board needs one); style it, do not re-implement it. Features import our wrappers, not `@base-ui/react` ([../web/base-ui.md](../web/base-ui.md)). Do not import a second component kit. For HTML, CSS and browser APIs, and for what to build natively where Base UI has no component, read [../web/antilog.md](../web/antilog.md).
- **No `useEffect`.** It is banned in the client, with `useLayoutEffect`, by a Biome `noRestrictedImports` rule (target). Each job it is used for has a better home: server data is a query, a value from props or state is derived in render, work caused by a click or submit runs in the handler, a live-event subscription is TanStack Query's or `useSyncExternalStore`, start-up runs once at the entry module, and resetting state on a new id is a `key`. A third-party widget that needs a DOM node uses a ref callback. If none fits, ask; do not add an exception.
- **Server state belongs to TanStack Query**, not to `useState` plus an effect. One `queryKey` per resource, built by a function, so many components share a request. Writes use `useMutation` and then invalidate the keys they change. Live events (server-sent) update the query cache; they do not create a parallel store. Local UI state (open panel, draft text) is `useState` or the URL.
- **The URL is state.** Routes, ids, filters and the open item live in TanStack Router's typed search params and path params, so a view can be linked to, reloaded and found by the global search. Load route data in the route's `loader` with `queryClient.ensureQueryData` to avoid waterfalls ([general/react-best-practices.md](general/react-best-practices.md)).
- **Product rules are the server's.** The client shows state and sends intent. It does not decide who may do what or compute a spend total: it renders what the API says and shows what is denied. Behaviour is in [design behaviour](../../design/behaviour.md).
- **No customer data in the client's storage.** `localStorage` is for per-viewer conveniences only (theme, collapsed panels), versioned, minimal, and wrapped in `try/catch`. No tokens in storage: the session is a cookie.
- **Accessible by default.** Every control is a real `button`, `a`, `input` or a Base UI component, reachable by keyboard, with a visible focus ring and a text name; colour is never the only signal; motion respects `prefers-reduced-motion`. Tab order follows reading order. Test with the keyboard.
- **Boundaries are validated.** Parse anything from outside the typed API (URL params, `postMessage`, storage, a file) as `unknown` and narrow it before use.
- **Small files.** Split a component or module that does two jobs; keep a component file near one screenful. No barrel `index.ts` that re-exports a folder (it defeats tree shaking and slows the dev server).

## Tests

How tests are written: [land/tests.md](../../land/tests.md). In the client:

- Vitest for logic and components, run in `jsdom` or `happy-dom`. Test behaviour through the rendered output and roles (Testing Library queries by role and name), not class names or implementation. Mock the network at the boundary (a request-interception layer such as MSW is not in [libraries.md](../libraries.md): ask before adding it, or stub `openapi-fetch` with a fake `fetch`).
- Fixtures and type tests: [general/vitest-fixtures.md](general/vitest-fixtures.md), [general/vitest-type-testing.md](general/vitest-type-testing.md). `expect.requireAssertions` is on: a test with no assertion fails.
- Playwright covers the few journeys that matter end to end, against a built client and a test server, in light and dark. No sleeps: wait on a role, a text or a network response.
- A component's loading, empty, error and denied states each have a test; they are in the design behaviour.

## Diagnostics

Fix the first error first; later ones are often its echo.

| Diagnostic | Fix |
| --- | --- |
| TS2532 / TS18048 "possibly undefined" on `a[i]` or `map.get(k)` | `noUncheckedIndexedAccess`: narrow with `if (x === undefined) throw new Error("missing <what>")`, or handle the empty case; do not use `!` |
| TS2322 "not assignable" on a literal typed `: T` | Use `satisfies T` so the literal keeps its narrow type, then fix the property the message names |
| TS7053 "implicitly has an 'any' type" indexing an object | Type the key as `keyof typeof obj`, or make the object a `Record<Key, V>` |
| TS2345 `unknown` / `string` not assignable to a union | Narrow with a guard or a parse at the edge; do not cast |
| Biome `noExplicitAny` | Use `unknown` and narrow, or a generic |
| Biome `useImportType` | `import type { X } from "./x"` as its own statement |
| Biome `useExhaustiveDependencies` (hooks) | Fix the dependency list of the remaining hook (`useMemo`, `useCallback`); derive in render or move the work into the handler. Do not silence it |
| A generated type is missing or wrong after an API change | Regenerate the client from the current OpenAPI document and commit it; never edit the generated file |
| A literal colour, string or font flagged in review | Use the token CSS variable or the catalogue key; add the missing one at its source |
| `Warning: Each child in a list should have a unique "key"` | Key by the item's stable id, never the index when the list can reorder |
| "Cannot update a component while rendering a different component" | A state update ran in render; move it to an event handler or derive the value |
| Hydration or `window is not defined` messages | There is no server rendering here: the code ran at build or in a test environment without a DOM; guard it or move it into a handler |
| Test fails "no assertions" | `expect.requireAssertions` is on: assert something, or `expect.assertions(0)` for a deliberate assertion-free test |

## Pitfalls

- **Tokens have one source, `design/tokens.toml`.** A hardcoded hex, a parallel CSS variable, or a font-family override causes drift between the web app, the Mac app and the boards. After a token edit run the generator and commit its output; the check fails on drift.
- **Theme rules that fail silently.** An appearance preference written as an inline style on `<html>` outranks every theme block, so set `data-theme` only when the member chose one and remove it otherwise. Set it from a tiny script in `index.html` before React mounts, to avoid a flash. When a declaration and the rendered product disagree, the product wins.
- **A guard on a generated-and-committed pair must watch the source.** If a file is generated and also committed, a test pointed at the generated file goes green while the source is wrong, and the next regeneration restores the defect. Assert against the source and, separately, that the generated half matches it. Being in git says nothing about which file a human edits.
- **Reaching for an effect means the data lives in the wrong place.** Fetching is a query; state set from props is derived during render; see the `useEffect` ban above. The `useEffect` and `useEffectEvent` advice in `general/` does not apply here.
- **List keys are the item's id, never the index.** Chat messages and timeline rows stream in and reorder; an index key reattaches state (an open row, a draft) to the wrong item.
- **No manual memoisation by default.** No `useMemo`, `useCallback` or `memo` until a profile shows a render that costs; the React Compiler, if adopted ([../libraries.md](../libraries.md)), does the rest. A memo with a wrong dependency list is a stale-value bug.
- **Never copy query data into state.** `useState(data.title)` is a copy that goes stale when the query refetches. Read from the query; keep only the unsaved draft in state, keyed to the item's id so a new item starts a new draft.
- **A cast is not a check.** `as` on a server-sent event, `JSON.parse`, `postMessage` or storage is a lie the compiler believes. Parse it as `unknown` at the boundary, against the generated API types, and drop or report what does not fit. The chat stream is the place this bites.
- **No floating promises.** Every promise is awaited, returned, or handled with `.catch` that shows the failure; Biome's `noFloatingPromises` is on. A mutation that fails silently looks like a lost approval.
- **Live events and the cache.** A server-sent event handler must clean up on unmount, reconnect with backoff and resume from the last event id, and update or invalidate query keys rather than keep its own copy of the data.
- **Optimistic updates lie when the server refuses.** Roll back on error and show the server's reason. A spend figure and a card's outcome are never optimistic.

## General TypeScript and React guidance

Read [`general/general-practices.md`](general/general-practices.md) for type design, errors, async, modules, testing and performance. Focused references: [`type-patterns.md`](general/type-patterns.md), [`react-best-practices.md`](general/react-best-practices.md) (waterfalls, bundle size, client data fetching, re-renders, rendering, JavaScript performance), [`composition-patterns.md`](general/composition-patterns.md) (compound components, explicit variants, state in providers, React 19 APIs), [`vitest-fixtures.md`](general/vitest-fixtures.md), [`vitest-type-testing.md`](general/vitest-type-testing.md).

The diagnostic and performance guidance adapts [Bama](https://github.com/daveebbelaar/bama) by Dave Ebbelaar (MIT, [`licenses/Bama-LICENSE`](licenses/Bama-LICENSE)); source attributions are in [`general/general-practices.md`](general/general-practices.md). Licences: [typescript-best-practices](licenses/ofershap-typescript-best-practices-LICENSE), [antfu/skills](licenses/antfu-skills-LICENSE), and the [Vercel notice](licenses/vercel-labs-agent-skills-NOTICE.md) for the React digest.

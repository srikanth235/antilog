# React best practices (digest)

Source: [react-best-practices](https://github.com/vercel-labs/agent-skills/tree/main/skills/react-best-practices) in vercel-labs/agent-skills, by Vercel Engineering (version 1.0.0, MIT as declared in the repo's README and skill header; see [`../licenses/vercel-labs-agent-skills-NOTICE.md`](../licenses/vercel-labs-agent-skills-NOTICE.md)). The original has 70 rules in 8 categories, ordered by impact. This digest keeps the client-side rules for a Vite single page app and drops the Next.js and React Server Components parts (server actions, `React.cache`, RSC serialization, `after()`, hydration, `next/dynamic`). Where the original names SWR, read TanStack Query, which this repo uses. [`../ratchet.md`](../ratchet.md) wins where they differ.

Apply these when you write or review components, fetch data, or chase a slow screen. Measure before you rewrite: a rule helps only where the cost is real.

## 1. Eliminate waterfalls (critical)

A waterfall is a chain of awaits or fetches where each starts only after the last one ends. It costs a network round trip per link and is usually the biggest win.

- **Start independent work together.** `const [a, b, c] = await Promise.all([fa(), fb(), fc()])` instead of three awaits in a row (2 to 10 times faster in the source's measurements). In a handler, start promises early and await late: create `const p = fetchConfig()` first, do the dependent step, then await both.
- **Partial dependencies.** When `profile` needs `user` but `config` does not, create the promises first and chain with `.then`, then `Promise.all` at the end, so config does not wait for user. The source suggests the `better-all` package for this; here use plain promises unless the owner approves a dependency.
- **Defer an await to the branch that uses it.** Do not `await` something before an early return that does not need it. Check the cheap synchronous condition before an async flag: `if (cheap) { const flag = await getFlag(); ... }`. Keep the original order when the cheap check is expensive, depends on the flag, or side effects must run in order.
- **In the client app, waterfalls hide in components.** A parent that fetches, renders a child, which then fetches, is a waterfall. Start queries at the route level (a TanStack Router `loader` that calls `queryClient.ensureQueryData`) or hoist them to the parent, and use `useQueries` for sibling queries. Do not fetch inside an effect that waits for another fetch's result when the inputs are known earlier.
- **Show the shell first.** Render layout, sidebar and header at once and put a `<Suspense fallback>` (or a skeleton driven by `isPending`) around only the part that waits for data. Avoid it where the fallback shifts layout badly or the data decides the layout.

## 2. Bundle size (critical)

- **Import from the module you use, not a barrel.** A barrel is an `index` that re-exports many modules; icon and component packages can have thousands of re-exports and cost hundreds of milliseconds to load in dev and slow builds. Import `@mui/material/Button`, not `{ Button } from '@mui/material'`. Check that deep paths ship types before relying on them. In this repo's own code, avoid creating barrel files for the same reason (import each Base UI component from its subpath, such as `@base-ui/react/dialog`, as [../../web/base-ui.md](../../web/base-ui.md) says).
- **Split heavy components.** Use `React.lazy(() => import('./heavy'))` inside `<Suspense>` for a large editor, chart or viewer that is not needed on first paint (the original's `next/dynamic` example, a code editor of about 300 KB, becomes `lazy`). Split by route first: the router should lazy-load route components.
- **Load a feature's code and data only when it is turned on**, with a dynamic `import()` in the handler or effect that needs it, and handle failure.
- **Defer non-critical third parties** (analytics, error reporting) until after first render.
- **Preload on intent.** Start `import('./heavy')` on `onMouseEnter` and `onFocus` of the button that opens it, so the click feels instant. TanStack Router's `defaultPreload: 'intent'` does this for routes.
- **Keep dynamic import paths statically analysable.** `import(map[name]())` with an explicit object of `() => import('./a')` thunks lets the bundler see every target; a path built from a variable makes it include a broad set or give up.

## 3. Client-side data fetching (medium-high)

- **Never hand-roll fetch-in-effect with `useState`.** It has no deduplication, caching, cancellation or revalidation, and invites waterfalls and race conditions. The source recommends SWR; this repo uses TanStack Query: one `queryKey` per resource, so many components share one request; `useMutation` for writes, then invalidate the keys it changes; `staleTime: Infinity` for data that does not change.
- **Share global listeners.** Registering `window.addEventListener` in every instance of a hook gives N listeners; keep one listener and a module-level `Map` of callbacks (or one provider) and fan out.
- **Use passive listeners for scroll, wheel and touch** (`{ passive: true }`) unless the handler calls `preventDefault()`.
- **Version and minimise `localStorage`.** Key with a version (`prefs:v2`), store only the fields the UI needs (never tokens or personal data), wrap every `getItem` and `setItem` in `try/catch` (they throw in private windows, when quota is exceeded or when storage is disabled), and migrate old keys.

## 4. Re-render optimisation (medium)

- **Derive, do not sync.** If a value can be computed from props or state, compute it during render. Do not store it in state and update it from an effect (`fullName = first + ' ' + last`). See React's "You might not need an effect".
- **Put the response to an action in the event handler**, not in state plus an effect. `setSubmitted(true)` then an effect that posts is wrong: it re-runs on unrelated dependency changes and can fire twice.
- **Do not subscribe to state you only read in a callback.** Read `window.location.search` inside the handler instead of subscribing to every search-param change.
- **Subscribe to the derived value.** `useMediaQuery('(max-width: 767px)')` re-renders when the boolean flips; a width hook re-renders on every pixel.
- **Narrow effect dependencies** to primitives (`user.id`, not `user`), and derive a boolean outside the effect when only the threshold crossing matters.
- **Functional `setState`** (`setItems(curr => [...curr, ...new])`) gives stable callbacks without listing the state as a dependency and prevents stale closures.
- **Lazy initial state.** `useState(() => buildIndex(items))`, not `useState(buildIndex(items))`, which runs every render.
- **Memoise with intent.** Extract expensive work into a `memo` component so an early return (loading) skips it. Hoist a default non-primitive prop (`const NOOP = () => {}`) so memoisation is not broken by a new default each render. Do not wrap a cheap primitive expression in `useMemo`. Split a hook that does independent work (filter, then sort) so one dependency does not recompute both. Where React Compiler is enabled, prefer plain code and let it memoise.
- **Never define a component inside a component.** It is a new type every render, so React remounts it and loses its state and DOM. Pass props instead.
- **Keep input responsive.** `startTransition` for non-urgent updates, `useDeferredValue` for an expensive derived render driven by typing (compare `query !== deferredQuery` to show a stale hint), `useTransition` instead of a hand-written `isLoading` flag.
- **Transient, high-frequency values go in a `useRef`** (mouse position, timers) when they do not need to render.

## 5. Rendering (medium)

- **Use a ternary or a boolean for conditional JSX.** `{count && <Badge/>}` renders `0`; write `{count > 0 ? <Badge/> : null}`.
- **Hoist static JSX** out of the component so it is created once.
- **Long lists:** `content-visibility: auto` with `contain-intrinsic-size` makes the browser skip off-screen layout and paint (the source reports about 10 times faster first render for 1000 items). For very long or dynamic lists, virtualise.
- **Show and hide expensive subtrees with `<Activity mode="visible|hidden">`** to keep state and DOM instead of unmounting (React 19.2 and later).
- **Animate a wrapper `div`, not an `svg` element**, where hardware acceleration is missing. Trim SVG coordinate precision (SVGO).
- **Preload with React DOM resource hints** (`preconnect`, `preload`, `preinit`) for known critical resources; put `defer` or `async` on script tags so they do not block parsing.
- **Theme without a flash.** Read the stored theme in a tiny synchronous inline script in `index.html` that sets `data-theme` before React runs, rather than an effect after mount. (The source's hydration rules reduce to this in a client-only app.)

## 6. JavaScript performance (low to medium)

Micro-optimisations. Apply them in hot paths (large lists, per-keystroke work), not everywhere.

- Build a `Map` or `Set` for repeated lookups (`O(1)` instead of `find` or `includes` in a loop).
- Combine several `filter`/`map` passes into one loop; use `flatMap` to map and filter in one pass.
- Return early; check array length before an expensive comparison; use a loop for min and max instead of sorting.
- Hoist `RegExp` creation out of loops; cache property access, function results (a module-level `Map`) and storage reads (`localStorage` reads are synchronous and slow).
- Use `toSorted`, `toReversed` and `toSpliced` instead of mutating state arrays.
- Batch DOM and CSS changes through a class or `cssText`; avoid layout thrashing (reading layout between writes).
- Defer non-critical work with `requestIdleCallback`.

## 7. Advanced patterns (low)

- Do not put a `useEffectEvent` function in a dependency array; list the reactive values only and call the event inside.
- To read the latest callback without re-subscribing, use `useEffectEvent` (or a ref updated in an effect) instead of listing the callback as a dependency.
- Run app-wide start-up once at the entry module (or behind a module-level guard), not in a component's `useEffect([])`, which runs again on remount and twice under Strict Mode in dev.

## Dropped from the source

Server actions and auth, `React.cache`, cross-request LRU caches, RSC prop serialisation and deduplication, parallel server fetching, `after()`, `next/dynamic` and `optimizePackageImports`, and hydration mismatch handling. They belong to a server-rendered React framework; Ratchet's web client is a client-rendered single page app that the server only serves.

# Composition patterns (digest)

Source: [composition-patterns](https://github.com/vercel-labs/agent-skills/tree/main/skills/composition-patterns) in vercel-labs/agent-skills, by Vercel (version 1.0.0, MIT as declared in the skill header; see [`../licenses/vercel-labs-agent-skills-NOTICE.md`](../licenses/vercel-labs-agent-skills-NOTICE.md)). The original has 8 rules in 4 groups. This digest keeps what applies to a Vite single page app and drops the React Native examples. [`../antilog.md`](../antilog.md) wins where they differ, and Base UI components ([../../web/base-ui.md](../../web/base-ui.md)) already follow these patterns: use them before writing your own.

Apply these when a component grows a pile of boolean props, or when you build a part that several screens share.

## 1. Structure (high)

- **No boolean prop pileup.** Every `isThread`, `isEditing` or `isForwarding` doubles the states a component can be in, and the body fills with nested conditionals. If two flags are never both true, they were a mode, not a flag. Compose instead.
- **Compound components.** Split a complex part into small pieces under one name (`Composer.Frame`, `Composer.Input`, `Composer.Submit`) that read shared state from a context, not from props. The caller lists the pieces it wants. No hidden conditionals, no prop drilling.
- **Explicit variants.** Make one small component per use (`ThreadComposer`, `EditMessageComposer`) that assembles the shared pieces. A reader sees what renders without tracing flags. Impossible combinations cannot be written.

## 2. State (high to medium)

- **Lift state into a provider.** Components that need the same state do not have to sit inside the same visual box. They only have to sit inside the same provider, so a Submit button in a dialog footer can act on a form drawn above it.
- **Give the context a generic shape: `state`, `actions`, `meta`.** The pieces read that interface, never a specific hook. Then a provider with local state and a provider backed by a query or the URL can feed the same UI. Swap the provider, keep the UI. Only the provider knows where state lives.
- **Keep the state implementation out of the view.** A piece should not import `useChannelComposerState`; it takes the context.

## 3. Patterns (medium)

- **Children over render props.** Pass static structure as `children`. Use a `renderItem` callback only when the parent must hand data back to the child, as a list does for each row.
- **Do not build this for one use.** Compound parts and providers pay off when there are two or more variants or call sites. A single caller with two flags is fine as it is. Do not wrap the web app's own screens in provider trees for style.

## 4. React 19 (medium)

This repo is on React 19.

- **`ref` is a normal prop.** Do not use `forwardRef`; accept `ref` in the props and pass it on.
- **Read context with `use(Context)`**, not `useContext`. It can also run after a condition or early return.
- **Render `<Context value={...}>`** directly, not `<Context.Provider>`.

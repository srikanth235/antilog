# Base UI in Ratchet

(target) The web client does not exist on this branch. This page says how [Base UI](https://base-ui.com) (`@base-ui/react`, MIT, by the MUI team) is used when it lands.

Base UI's own docs teach the library. This page steers: which component to reach for on our screens, where the docs' demos would lead you away from our rules, and what done looks like. When this page and a Base UI page disagree about how Ratchet looks or is built, this page wins. When they disagree about how a component behaves, Base UI's page wins: it is newer than your memory.

## Before you write a component

1. **Find the job in the map below.** It names the component our boards need. If the job is not there, read the component's "Usage guidelines" to choose, and add a row here in the same change.
2. **Read the component's page.** The index is [base-ui.com/llms.txt](https://base-ui.com/llms.txt); every page is also a `.md` file, for example [dialog.md](https://base-ui.com/react/components/dialog.md). Read the usage guidelines and the API reference (parts, props, data attributes, CSS variables), not only the first demo. For the handbook topics, read [styling](https://base-ui.com/react/handbook/styling.md), [composition](https://base-ui.com/react/handbook/composition.md), [animation](https://base-ui.com/react/handbook/animation.md), [customization](https://base-ui.com/react/handbook/customization.md) and [forms](https://base-ui.com/react/handbook/forms.md) once.
3. **Look for our wrapper first.** If one exists, extend it. Never write a second wrapper for the same component.
4. **Read the matching web guide** (list below) for accessibility and behaviour the component does not cover.
5. **Write the wrapper, then go through the checklist** at the end of this page.

Your training data may know the old package name `@base-ui-components/react` and pre-1.0 APIs. Both are wrong here: the package is `@base-ui/react`, pinned to exactly 1.9.0 ([../libraries.md](../libraries.md)). Radix habits are wrong too: there is no `asChild`.

## Which component for which job

From the boards in `design/mockups/`. "Not" is the near miss to avoid.

| On our screens | Use | Not |
| --- | --- | --- |
| Search (⌘K): type anything, jump to a result | Dialog holding an Autocomplete | Combobox: it remembers a selection and rejects free text |
| Pick one from a long list by typing: a member for a group, a dataset, a connector | Combobox | Select: no text input |
| Pick one from a short list: environment, sort order | Select | a native `<select>` styled by hand |
| New experiment, Add connector, Add agent, Start a runner, Settings | Dialog. Its open state comes from the URL (below) | Drawer: only for gestures and snap points, which no board has |
| Confirm something that cannot be undone: Stop run, Revoke token | Alert dialog | Dialog |
| Add ▾, More, a row's ⋯ actions | Menu. A choice that goes to another page is a `Menu.LinkItem` | a Popover with buttons |
| Filter | Menu with checkbox items (or radio items for one of several) | a Popover with a form |
| Customize tabs (Evaluations, Skills, Connectors, Agents, Plugins), Project tabs | Tabs, each `Tabs.Tab` rendered as our router link with `nativeButton={false}`, because each tab is a URL | Tabs that only swap panels in state |
| Yours and Discover | two router links styled as a segmented control, with `aria-current` on the current one | Toggle group: it holds state, not a location |
| The settings switches | Switch inside a Field (label and description) | Checkbox styled as a switch |
| Budget amounts | Number field inside a Field, formatted with its `format` prop (`Intl.NumberFormatOptions`, currency) | a text input and hand formatting |
| Spend against a budget | Meter: a value within a known range | Progress |
| A long task under way, such as a runner starting | Progress | Meter, or a spinner when the amount is known |
| A short confirmation after an action, such as a copy or a saved setting | Toast, from one `Toast.Provider` at the app root | `window.alert`, or a banner that pushes the layout |
| An icon-only button's hint | Tooltip, under one `Tooltip.Provider` at the root so tooltips share a delay. The button still has its own `aria-label` | a `title` attribute |
| Sidebar project groups that fold, and an open experiment's nested sessions | Collapsible | Accordion: for sets where one panel opening matters to the others |
| A popover with details anchored to a control | Popover | Dialog: it interrupts on purpose |

## Where the demos lead you wrong

Base UI's demos are written to be self-contained. Take their structure, parts and data attributes; leave the rest.

- **Colours, sizes and fonts.** Demos hard-code values, Tailwind first and raw colours even in the CSS-modules version. Ours come only from the generated tokens CSS (`var(--...)`) and its type classes ([../typescript/ratchet.md](../typescript/ratchet.md)). No Tailwind.
- **Dark mode.** Demos add their own `prefers-color-scheme` or `dark:` styles. Never copy them. Our theme is `data-theme` on `<html>`, set before React runs; a portal renders into `<body>`, so popups get the right tokens without a `container` prop. Do not put `data-theme` on any inner element.
- **Icons.** Demos import icon components. Ours come from the one sprite ([ratchet.md](ratchet.md)).
- **Words.** Demo labels are placeholders. Every string comes from the words catalogue by key ([../typescript/ratchet.md](../typescript/ratchet.md)).
- **Links.** Demos use a framework's `Link`. Ours is the TanStack Router link, passed through `render`.
- **Animation libraries.** The animation page shows Motion. We do not use one; CSS transitions only.

## Rules

- **Wrap each component once.** One folder per component in the client's components folder (target; its path is set with the first client code), with its `.module.css`. Features import our wrapper, never `@base-ui/react`. Inside a wrapper, import from the component's subpath: `import { Dialog } from "@base-ui/react/dialog"`. No barrel `index.ts` re-exporting the folder. Type a wrapper's props from Base UI's namespaces (`Dialog.Root.Props`), not by copying them.
- **A wrapper exposes our design, not every prop.** Expose the variants the boards use. Do not pass `className` or `style` through to features, so a screen cannot restyle a part.
- **Style with data attributes and CSS variables.** Key states on the part's data attributes (`[data-open]`, `[data-highlighted]`, `[data-checked]`, `[data-disabled]` and so on; the API reference lists them per part), and size popups with its CSS variables (`--available-height`, `--anchor-width`, `--transform-origin`). Prefer these over a `className` function and never use the `style` prop for looks.
- **Style focus yourself.** Base UI moves focus; showing it is our job. Every interactive part gets the `:focus-visible` ring from the tokens.
- **Animate with transitions.** Put the hidden look on `[data-starting-style]` and `[data-ending-style]` and set a `transition` from the motion tokens. Base UI recommends transitions over keyframes because they reverse smoothly when a popup closes mid-open. Turn motion off under `prefers-reduced-motion` ([motion](general/ui-behaviors/motion.md)).
- **Give the app root `isolation: isolate`**, as Base UI's [quick start](https://base-ui.com/react/overview/quick-start.md) says, so popups stack above the page whatever `z-index` a screen uses. Do not add `z-index` to popups to fix stacking.
- **Compose with `render`.** To render a part as our component, pass `render={<OurLink to="..." />}`. Our component accepts a `ref` and spreads every prop it receives onto its element. When the rendered element is not a `<button>` (a link, for example), set `nativeButton={false}` on parts that have that prop. Read how props merge in [composition](https://base-ui.com/react/handbook/composition.md) before nesting `render`.
- **Navigation is a link.** Anything that changes the page is a real link through the router: `Menu.LinkItem`, a tab with `render`, a plain link. Never navigate from an `onClick` or an `onValueChange`.
- **The URL owns what can be linked.** A dialog that is a place (Settings, Add connector, New experiment) is opened by a route or search param and controlled with `open` and `onOpenChange` from the router. Small popups (menus, tooltips, popovers) stay uncontrolled.
- **Change handlers get a reason.** `onOpenChange(open, eventDetails)` and the other change handlers pass `eventDetails` with a `reason` and `cancel()`. To stop a change (for example, keep a dialog with unsaved input open on an outside click), call `cancel()` for that reason instead of mirroring the state. `preventBaseUIHandler()` is an escape hatch: use it only where no prop exists, with a comment saying why.
- **Every dialog has a title.** Use `Dialog.Title` (or `AlertDialog.Title`) and `Description` where the board has one. Form controls sit in a `Field` with `Field.Label`, and `Field.Description` and `Field.Error` where the board shows them; group related controls in a `Fieldset`.
- **Numbers are formatted by the component.** Number field and Meter take `locale` and `format` and use `Intl.NumberFormat`. Do not format a number into a string first.
- **Nothing outside the pin.** No date inputs: `date-fns` and `@date-fns/tz` are optional peer dependencies and stay uninstalled unless a product call adds dates. No Motion, no Tailwind, no second component kit, no hand-made focus trap or popup positioning.

## Which web guide goes with which wrapper

Each wrapper's top comment names the guide in [general/INDEX.md](general/INDEX.md) it follows: [dialog](general/ui-components/dialog.md), [menu](general/ui-components/menu.md), [tabs](general/ui-components/tabs.md), [progress](general/ui-components/progress.md), [persistent toast notifications](general/ui-components/persistent-toast-notifications.md), [checkbox](general/ui-components/checkbox.md), [switch](general/forms/switch.md), [forms](general/forms/forms.md), [position-aware tooltips](general/ui-atoms/position-aware-tooltips.md). Where a guide and Base UI differ on behaviour, Base UI wins; the guide's accessibility and reduced-motion rules still apply.

## Where Base UI has no component

Build these natively, as the matching guide says: copy button, spinner, tables, sticky headers, scroll shadows. Base UI has Accordion, Collapsible and Scroll Area; use those rather than `details` or a custom scrollbar, and use Scroll Area only where the board draws its own scrollbar.

## Done checklist for a wrapper

- The component's Base UI page was read, including its usage guidelines and API reference.
- Only tokens: no hex, no `rgb()`, no raw size, no font, no Tailwind class, no copied dark-mode block.
- Checked on screen in light and dark.
- Works by keyboard alone: open, move, choose, close with Esc, and focus returns to the trigger. The focus ring shows.
- Motion stops under `prefers-reduced-motion`.
- Every control has a name; every dialog has a title; words come from the catalogue.
- Links are links; the URL opens the dialogs that are places.
- Tests query by role and name and cover open, choose and close ([../typescript/ratchet.md](../typescript/ratchet.md)).
- The top comment names the web guide it follows.

## Upgrading

Change the pin in [../libraries.md](../libraries.md) on purpose, one version range at a time. Read the [release notes](https://base-ui.com/react/overview/releases.md) of every version you cross, search them for the components we wrap, then run the component tests and the end-to-end journeys in light and dark.

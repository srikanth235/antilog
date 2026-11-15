# Base UI in Ratchet

(target) The web client does not exist on this branch. This page says how [Base UI](https://base-ui.com) (`@base-ui/react`, MIT, by the MUI team) is used when it lands. It is our rules, not a tutorial; the component pages are the tutorial.

## Rules

- **Read the component's page before you write it.** The index is [base-ui.com/llms.txt](https://base-ui.com/llms.txt); every page there is also a `.md` file. Its API tables list the parts, props, data attributes and CSS variables. Pin `@base-ui/react` to exactly 1.9.0 and read the [release notes](https://base-ui.com/react/overview/releases/v1-9-0.md) of each version you cross when you upgrade.
- **Wrap each part once.** Wrappers live in the client's components folder (target; its path is set with the first client code), one folder per component with its `.module.css`. Features import our wrapper, never `@base-ui/react`. Inside a wrapper, import per component from its subpath: `import { Dialog } from "@base-ui/react/dialog"`.
- **Style with CSS modules and tokens.** Key styles on Base UI's data attributes (`data-open`, `data-popup-open`, `data-highlighted`, `data-disabled`, `data-checked`, `data-starting-style`, `data-ending-style`) and its CSS variables (`--available-height`, `--anchor-width`, `--transform-origin`). Colours, spacing, radius and motion are `var(--...)` tokens; no inline values and no `style` prop for looks.
- **Animate with transitions.** Set the closed look on `[data-starting-style]` and `[data-ending-style]` and give the popup a `transition`; Base UI keeps it mounted until the transition ends. Wrap the motion in `prefers-reduced-motion` ([motion](general/ui-behaviors/motion.md)). No animation library.
- **Portals stay inside the theme.** Popups render in a portal. Pass the portal part's `container` the element that carries `data-theme`, so the dark tokens reach dialogs, menus and toasts. Give the app root `isolation: isolate` so popups sit above the page ([quick start](https://base-ui.com/react/overview/quick-start.md)).
- **Compose with `render`.** To make a part another element or our component, pass `render={<OurLink />}`; our component must forward the `ref` and spread the props it gets ([composition](https://base-ui.com/react/handbook/composition.md)). There is no `asChild`.
- **Icons come from our sprite** ([web/ratchet.md](ratchet.md)). Never a second icon set, and never an icon package a Base UI demo imports.
- **Name the guide.** Each wrapper's top comment names the guide in [general/INDEX.md](general/INDEX.md) whose rules it follows: [dialog](general/ui-components/dialog.md), [menu](general/ui-components/menu.md), [tabs](general/ui-components/tabs.md), [progress](general/ui-components/progress.md), [persistent toast notifications](general/ui-components/persistent-toast-notifications.md), [checkbox](general/ui-components/checkbox.md), [switch](general/forms/switch.md), [forms](general/forms/forms.md), [position-aware tooltips](general/ui-atoms/position-aware-tooltips.md). Where the guide and Base UI differ on behaviour, Base UI's part wins; the guide's accessibility and reduced-motion rules still apply.
- **No date inputs for now.** Base UI lists `date-fns` and `@date-fns/tz` as optional peer dependencies. Do not install them unless a product call adds date inputs ([../libraries.md](../libraries.md)).

## Where Base UI has no component

Build these natively, as the matching guide says: copy button, spinner, tables, sticky headers, scroll shadows. Base UI has Accordion, Collapsible and Scroll Area; use those rather than `details` or a custom scrollbar.

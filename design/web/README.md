# Web client prototype (target)

The approved direction for the web client of the server product. It is a clickable prototype with seeded sample data and no backend. Nothing in it is wired to a real API, and the web app does not exist yet.

| File | What it is |
| --- | --- |
| `Main.dc.html` | The app: sidebar, experiment threads, New experiment, Projects, Customize, search, Dashboard, Run history, Settings. Generated from `src/`. |
| `SignIn.dc.html` | The sign-in screen. |
| `support.js` | The runtime the two files load. Not edited. |
| `canvas.json` | The Design canvas layout: the two boards and the launch file. |
| `src/` | The parts of `Main.dc.html` and the script that assembles them. |

The shape of the app and the words on it are in [`.claude/skills/design/behaviour.md`](../../.claude/skills/design/behaviour.md) and [`.claude/skills/design/ui-terms.md`](../../.claude/skills/design/ui-terms.md).

## View it

Open `Main.dc.html` or `SignIn.dc.html` in a browser, straight from disk. They need only `support.js`, the Design canvas runtime, which sits next to them in this folder. No server and no canvas are needed. Fonts load from Google Fonts when online and fall back to system fonts offline. A live copy is also at https://claude.ai/code/artifact/a39784c4-f9b3-499c-ab6d-2919bd0ddd04, private to the owner.

`support.js` is the canvas runtime as served with the prototype (it bundles React, MIT). Do not edit it.

## Edit it

Edit the parts in `src/`, then run `bash design/web/src/assemble.sh`. It writes `design/web/Main.dc.html` and runs a syntax check on the script when `node` is installed. Do not edit `Main.dc.html` by hand.

| Part | Holds |
| --- | --- |
| `style.css`, `extra.css` | Styles and tokens. |
| `m1.html` to `m5.html` | Markup, in page order. |
| `logic1.js` to `logic3.js` | State, seeded data and handlers, in order. |
| `strip.py` | Removes leading whitespace from the assembled file. |

`SignIn.dc.html` and `canvas.json` are edited directly.

## What was checked

- Click-through scripts over every page ran with zero browser errors.
- Both files render from disk with the network blocked.
- Screenshots were reviewed in light, in some dark pages and at 1024px width.

## What was not checked

- Some pages in dark mode.
- Nothing is wired to a real API. Sample values, including the access token shown once (`rtk_example_0000000000000000`), are fake display data.
- The click-through scripts and screenshots are not in the repo.

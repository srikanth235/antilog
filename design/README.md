# Design

What is here:

| Path | What it is |
| --- | --- |
| `web/` | The approved web client prototype (target): a clickable prototype with seeded sample data and no backend. Sessions sidebar, experiment thread, Customize, global search. See [`web/README.md`](web/README.md). |
| `icon/` | The logo. `icon/AppIcon.icon` is the Icon Composer source; `icon/ratchet-icon.svg` is the flat 1024 px icon, `icon/ratchet-mark.svg` the full mark, `icon/ratchet-icon-mono.svg` and `icon/ratchet-menubar.svg` single-colour cuts. |
| `DESIGN-SYSTEM.md` | The look (Midnight Brass): colour tokens, type, shape and the three area colours, as used by `web/`. |
| `tools/build-icon.py` | `python3 design/tools/build-icon.py` rewrites the icon layers and the flat `icon/ratchet-icon.svg` from the mark. |

Open `web/Main.dc.html` or `web/SignIn.dc.html` in a browser straight from disk. No server is needed. To change the prototype, edit `web/src/` and run `bash design/web/src/assemble.sh`.

Every visual or UX change starts in `web/`. Behaviour and words are in [`.claude/skills/design/`](../.claude/skills/design/SKILL.md).

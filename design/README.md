# Design

What is here:

| Path | What it is |
| --- | --- |
| `tokens.toml` | The only place values live: colour, type, shape, in light and dark. |
| `tools/` | `tokens.py` generates `mockups/tokens.css`, `mockups/logos.css`, every dark twin and its canvas entry, and fills the part markers (`python3 design/tools/tokens.py`, and `--check` to fail on drift, a raw colour, a dangling link or import, or a board missing from the canvas). `build-icon.py` rewrites the icon layers and the flat icon from the mark. The generator moves to a Rust `cargo xtask design` with the first code (target). |
| `mockups/` | The boards: one static `X.dc.html` per screen state (web, 1440 by 900), `MacX.dc.html` only where the Mac app differs, `canvas.json` with the pages, `DesignRules.dc.html` (the look) and `Decisions.dc.html` (the owner's dated calls). Shared chrome as parts (`Sidebar.dc.html` and others) that the generator inserts at `<!-- part:Name -->` markers; the generated `tokens.css` and dark twins `XDark.dc.html`. Boards are plain static files: open one from disk. |
| `logos/` | Third-party logos for the boards: SVGs from Simple Icons (CC0, licence beside them) and `logos.toml`, which names each brand, its colour, and the brands shown as a text tile. |
| `icon/` | The logo. `icon/AppIcon.icon` is the Icon Composer source; `icon/ratchet-icon.svg` is the flat 1024 px icon, `icon/ratchet-mark.svg` the full mark, `icon/ratchet-icon-mono.svg` and `icon/ratchet-menubar.svg` single-colour cuts. |


Every visual or UX change starts in `mockups/`; the flow and the checks are in [`.claude/skills/design/mockups.md`](../.claude/skills/design/mockups.md). Behaviour and words are in [`.claude/skills/design/`](../.claude/skills/design/SKILL.md).

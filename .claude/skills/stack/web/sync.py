"""Copy the guides Ratchet uses from Modern Web Guidance into general/.

    python3 .claude/skills/stack/web/sync.py <checkout>   copy from a checkout of
        https://github.com/GoogleChrome/modern-web-guidance (the published build)
    python3 .claude/skills/stack/web/sync.py --check      verify general/ against
        the list below, offline

Guides are copied verbatim except one change: a cross-reference that tells the
reader to run the upstream CLI becomes a link to the local copy, or to the
upstream file when the guide is not taken. To take or drop a guide, edit
GUIDES and run the sync; never edit general/ by hand.
"""
import os, re, shutil, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "general")
UPSTREAM = "https://github.com/GoogleChrome/modern-web-guidance"
BLOB = UPSTREAM + "/blob/{commit}/skills/modern-web-guidance/guides/{path}.md"

# Discipline -> guides taken. Left out on purpose: built-in AI, WebAssembly,
# WebMCP, installable web apps, web components (the client is React),
# passkeys and password forms (sign-in is OpenID Connect), payment, address
# and SMS forms, analytics beacons, media loading, carousels, touch gestures,
# 3D, canvas and decorative effects. Ratchet's client is a desktop web app.
GUIDES = {
    "accessibility": "accessibility accessible-error-announcement accessible-generated-content respect-os-text-scale",
    "css": "css css-layout animate-to-intrinsic-sizes atrule-support-conditionals calculate-with-intrinsic-sizes "
           "child-state-based-styling content-based-styling design-token-reactivity dynamic-sibling-styling "
           "fluid-scaling individual-transform-properties overflow-clipping-control reduce-style-repetition "
           "responsive-design selector-atrule-combinations size-aware-styling style-parent-with-has "
           "usage-aware-component-variations",
    "forms": "forms validate-input-after-interaction required-field-feedback select-menu-interaction "
             "branded-select-styling brand-consistent-forms rich-media-picker switch "
             "form-fields-automatically-fit-contents ime-safe-enter-submit",
    "html": "html",
    "js": "calculate-event-differentials sort-high-precision-timestamps",
    "performance": "performance break-up-long-tasks schedule-tasks-by-priority identify-inp-causes "
                   "identify-heavy-scripts interactions-in-complex-layouts interaction-paint-timing "
                   "defer-rendering-heavy-content defer-work-until-scroll-ends deprioritize-background-fetches "
                   "efficient-background-processing faster-spa-view-transitions "
                   "improve-next-page-load-performance conditional-async-dependencies",
    "privacy": "privacy",
    "security": "security sanitize-untrusted-html trusted-types restrict-outbound-connections validate-origins "
                "secure-external-subresources",
    "ui-atoms": "component-specific-light-dark-theme icons position-aware-tooltips responsive-table "
                "scrollability-affordance-hints single-line-overflow state-aware-sticky-headers "
                "viewport-aware-dropdowns scroll-position-aware-elements",
    "ui-behaviors": "motion animate-element-entry-exit animate-to-from-top-layer anchor-positioning-tab-underline "
                    "custom-button-actions declarative-dialog-popover-control light-dismiss-a-dialog "
                    "platform-controls-dismiss-dialog prevent-page-scroll-under-modal-dialog "
                    "interest-triggered-tooltips dynamic-sort sortable-table group-element-transitions "
                    "highlight-text-ranges same-document-transitions scoped-component-transitions "
                    "scroll-target-on-load search-hidden-content",
    "ui-components": "accordion breadcrumbs card checkbox copy-button dark-mode-toggle dialog menu "
                     "persistent-toast-notifications progress spinner tabs toolbar",
    "user-experience": "content-first-noise-reduction natural-interactions",
    "visual-design": "color dark-mode typography browser-ui-theming customize-scrollbar-color-and-thickness "
                     "adapt-scrollbar-to-contrast-preferences improve-text-layout-and-legibility "
                     "precise-text-alignment soft-edge-content-fade visually-stable-font-fallbacks "
                     "visually-stable-mixed-fonts visual-effects",
}
TAKEN = {g: d for d, gs in GUIDES.items() for g in gs.split()}
CLI_REF = re.compile(r'`([a-z0-9-]+)` \(via `npx -y modern-web-guidance@latest retrieve "([a-z0-9-]+)"`\)')


# The upstream README lists use-case guides only; these describe the broad
# discipline guides it leaves out.
DISCIPLINE = {
    "accessibility": "Rules for every screen: semantic HTML, names, focus, keyboard, ARIA, motion and contrast.",
    "forms": "Rules for every form: labels, input types, autocomplete, validation timing and errors.",
    "html": "Rules for every page: semantics, native interactive elements (dialog, popover, details), loading.",
    "performance": "Rules for load and responsiveness: the critical path, LCP, INP, CLS, scheduling work.",
    "security": "Rules for XSS, Content Security Policy, cookies, cross-origin isolation and safe DOM writes.",
}


def sync(checkout):
    commit = subprocess.check_output(["git", "-C", checkout, "rev-parse", "HEAD"], text=True).strip()
    root = os.path.join(checkout, "skills", "modern-web-guidance", "guides")
    upstream = {os.path.basename(f)[:-3]: d for d in os.listdir(root)
                if os.path.isdir(os.path.join(root, d)) for f in os.listdir(os.path.join(root, d))}
    readme = open(os.path.join(checkout, "README.md")).read()
    desc = dict(re.findall(r"^- \*\*\[([a-z0-9-]+)\]\([^)]*\)\*\*: (.*)$", readme, re.M))
    missing = [g for g in TAKEN if upstream.get(g) != TAKEN[g]]
    if missing:
        sys.exit(f"not upstream (moved or removed): {missing}")

    def link(m, here):
        gid = m.group(2)
        if gid in TAKEN:
            target = os.path.relpath(os.path.join(TAKEN[gid], gid + ".md"), here)
        elif gid in upstream:
            target = BLOB.format(commit=commit, path=f"{upstream[gid]}/{gid}")
        else:
            sys.exit(f"cross-reference to unknown guide {gid}")
        return f"[`{m.group(1)}`]({target})"

    shutil.rmtree(OUT, ignore_errors=True)
    for gid, d in TAKEN.items():
        os.makedirs(os.path.join(OUT, d), exist_ok=True)
        text = open(os.path.join(root, d, gid + ".md")).read()
        text = CLI_REF.sub(lambda m: link(m, d), text)
        if "modern-web-guidance@" in text:
            sys.exit(f"{d}/{gid}: a CLI reference the sync does not rewrite")
        open(os.path.join(OUT, d, gid + ".md"), "w").write(text)
    shutil.copy(os.path.join(checkout, "LICENSE"), os.path.join(HERE, "licenses", "modern-web-guidance-LICENSE"))

    lines = [
        "# Modern Web Guidance: index",
        "",
        f"Source: {UPSTREAM} at `{commit[:12]}`, Apache-2.0 ([licence](../licenses/modern-web-guidance-LICENSE)).",
        "Generated by `sync.py`; do not edit. Cross-references to the upstream CLI were rewritten to links.",
        "Read [../ratchet.md](../ratchet.md) first: it says where Ratchet's own rules win.",
        "",
    ]
    for d, gs in GUIDES.items():
        lines += [f"## {d}", "", "| Guide | Use it to |", "| --- | --- |"]
        for g in gs.split():
            text = desc.get(g) or DISCIPLINE[g]
            lines.append(f"| [{g}]({d}/{g}.md) | {text.replace('|', '/')} |")
        lines.append("")
    open(os.path.join(OUT, "INDEX.md"), "w").write("\n".join(lines))
    print(f"{len(TAKEN)} guides from {commit[:12]}")


def check():
    errors = []
    have = {os.path.relpath(os.path.join(dp, f), OUT) for dp, _, fs in os.walk(OUT) for f in fs}
    want = {f"{d}/{g}.md" for g, d in TAKEN.items()} | {"INDEX.md"}
    errors += [f"missing {p}" for p in sorted(want - have)]
    errors += [f"not in GUIDES (remove it or add it and sync): {p}" for p in sorted(have - want)]
    index = open(os.path.join(OUT, "INDEX.md")).read() if "INDEX.md" in have else ""
    errors += [f"INDEX.md does not list {g}" for g, d in TAKEN.items() if f"]({d}/{g}.md)" not in index]
    for p in sorted(have & want):
        if "modern-web-guidance@" in open(os.path.join(OUT, p)).read():
            errors.append(f"{p}: unrewritten CLI reference")
    if not os.path.exists(os.path.join(HERE, "licenses", "modern-web-guidance-LICENSE")):
        errors.append("missing licenses/modern-web-guidance-LICENSE")
    for e in errors:
        print("web guides:", e, file=sys.stderr)
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    if sys.argv[1:] == ["--check"]:
        check()
    elif len(sys.argv) == 2:
        sync(sys.argv[1])
    else:
        sys.exit(__doc__)

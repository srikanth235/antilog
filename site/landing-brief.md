# Antilog landing page: design brief

For a coding agent redesigning `site/index.html` from scratch. Everything below was settled with the owner on 19 Nov 2026. Do not reopen the positioning; the design is yours.

## Goal

A single page at antilog.tech that makes a business reader understand Antilog in ten seconds and ask for access. It is also the "live product on a .tech domain" evidence for the No Excuses Fund (deadline 15 Dec 2026), so it must be fast, polished and never broken.

## What Antilog is (one sentence)

Antilog is an autonomous agent platform for business outcomes. You set the outcome, agents do the work, and every result is independently measured and valued in dollars, so you always know what your AI returned.

## Audience

Business and technology leaders who are buying or running AI agents and want to know what the spend returned. Not only engineers. Plain words; no jargon in headlines.

## Positioning (settled)

- Antilog is a platform. Cloud cost is one example outcome, not the pitch.
- Outcomes that can be quantified: cost down, revenue up, errors down, risk down.
- Spine: **You set the outcome. Agents do the work. Antilog shows the return.**
- Headline: **Autonomous agents, measured by business outcomes.**
- The single idea running through the page: "what did our AI return?". The product's own word for it is **Return** = value delivered divided by everything spent finding it, including experiments that found nothing.
- Trust, said once and in plain words: agents don't grade their own work. An independent judge measures every change against a test the agents can't see or alter. People approve what ships, every action is logged, and spending stops at the cap.

## Page outline (keep the order, restyle freely)

1. **Hero.** Headline above. Subhead: "Tell Antilog what should improve: lower cost, more revenue, fewer errors, less risk. Agents do the work. Every result is independently measured and valued in dollars, so you always know what your AI returned." Primary button **Request access**, secondary **Sign in**. A hero visual: an invented "Monthly report" (four improvements, one per outcome, value delivered, everything spent, Return) stamped "Illustrative example".
2. **The problem.** "Everyone has agents now. Few can say what they returned." Activity is easy to count; whether the business got better is not.
3. **How it works.** Three steps: Set the outcome (what should improve, how it is measured, what each unit is worth, how much you will spend). Agents do the work (experiments on your code and data, unattended, inside the budget). See the return (each improvement measured independently, arrives for approval with its value in dollars).
4. **What you can point it at.** Four outcome groups: Cost down (cloud bill, data pipeline cost), Revenue up (conversion, recommendations), Errors down (forecast error, response time), Risk down (fraud loss, stock-outs). Label as illustrative examples and add: "The first experiments on Antilog are on cost and speed in code."
5. **Why you can trust the figure.** Agents / the judge / your people, plus an illustrative receipt (marked "measured offline").
6. **The scoreboard.** "One figure, every month: Return." The formula, and three controls: a hard budget, approvals and audit, your data stays yours.
7. **Close.** "Put your agents on outcomes." Request access. "Antilog is open by invitation."

## Hard rules

- **Every claim must match what is built.** Built: the judge (seal, holdout, protected paths, signed receipts), value models with Delivered, Pending, At risk and Return, the monthly Report, budgets and caps, approvals and audit log, agents on agent platforms with no tie to one model provider. Phase 1 is AWS only, with one evaluation kind (speed/cost in code) and no data connectors, so outcomes beyond that are examples.
- **Any figure must be labelled "Illustrative example".** No customer names, logos, quotes, funding figures, benchmarks or statistics we cannot back up. Invent nothing that reads as a real result.
- **Never mention services** (engagements, consulting, "we set it up with you"). Optional services exist but are not on this page.
- **Words not to use:** RSI, AutoML, self-improving AI, multi-cloud, ROI (say Return), leaderboard, "contract", "automation". Say "experiment", "judge", "Return", "win", "receipt".
- **No names of AI vendors** as a lineup; only one agent platform is built.
- **Contact** is hello@antilog.tech only. Request access is a mailto link to it (subject "Request access to Antilog"). Never put any personal email in any file.
- **Sign in** links to https://app.antilog.tech (owner's call; leave it).

## Technical constraints

- One static file, `site/index.html`, no build step. Inline CSS and SVG; web fonts only from Google Fonts. No tracking scripts.
- Light and dark, following `prefers-color-scheme`, both designed (tokens on `:root`, dark overrides).
- Accessible: semantic landmarks, skip link, visible focus, text alternatives for every illustration, contrast AA, honours `prefers-reduced-motion`.
- Works at 400px wide with no horizontal scroll; at least 16px side gutters; tap targets at least 44px.
- Published by the owner to Cloudflare Pages. Do not attempt to publish.

## Look and feel (current direction, change what you like)

Warm paper and ink with one brass accent (light), a deep navy with the same brass (dark). Serif display headlines (Newsreader) with italic brass emphasis, a clean sans for body (Instrument Sans), mono for small labels (JetBrains Mono), a handwritten face (Caveat) for margin notes and stamps. The owner liked Greptile's blueprint structure and asked for it in Antilog's own style: thin rails with corner dots, bracketed mono labels like `[ THE PROBLEM ]`, numbered cells, framed illustration panels, a dark band mid-page, hatched spacer bands, chamfered buttons. The owner also liked illustrations on every section. Do not copy Greptile's colours, fonts, artwork or copy.

The logo is the inline SVG mark in the current file's header (reuse it). Brand tokens for the product itself are in `design/tokens.toml`, but this page is a marketing page outside the product's design rules, so it may use its own fonts.

## Where to look

- Current page: `site/index.html` (commit `803f481` on branch `claude/dreamy-ptolemy-vg7uga-messaging`).
- Product truth: `AGENTS.md`, `.claude/skills/product/rulings.md` (see the 19 Nov row), `docs/GLOSSARY.md`.

## Done means

Screenshots checked at 1440px and 400px in light and dark, no horizontal overflow, every claim traceable to the list above, and a commit on the messaging branch only (subject at most 72 characters, `type(scope): summary`, no pull request).

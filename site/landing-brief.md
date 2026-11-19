# Antilog landing page brief

Design and build the landing page for antilog.tech. This brief gives you the facts and the limits. The design is entirely yours: layout, structure, visuals, type, colour, motion and tone of the page. Make it stellar.

## What Antilog is

Antilog is an autonomous agent platform for business outcomes. A team says what should improve and how it is measured: lower cost, more revenue, fewer errors, less risk. AI agents then work on it on their own, as a team. Every change is tested on the whole running system, an independent judge decides whether it won, and every result arrives with its value in dollars. The team always knows what its AI returned.

## Who it is for

Business and technology leaders who are buying or running AI agents and can't yet say what that spend returned. Write for them, not only for engineers.

## The idea to land

Agents have made work cheap and plentiful. Activity is easy to count, but whether the business actually got better is not. Antilog starts from the business outcome and shows the return.

The settled lines (use them, or improve the wording without changing the meaning):

- One line: **You set the outcome. Agents do the work. Antilog shows the return.**
- Headline: **Autonomous agents, measured by business outcomes.**

## What is true (you may say these)

- **Outcomes, not activity.** A team names a business outcome, how it is measured, what one unit of improvement is worth, and a budget.
- **Agents work as a team, unattended.** Given a goal, agent sessions do the work, and a session can start others and hand them parts of it, all inside the budget.
- **People review, then the whole system is tested.** A person reviews the code. Testing deploys the entire solution to a throwaway environment and measures it there, because a few changed lines usually need the whole app running to be tested honestly.
- **No tie to one agent, model or cloud.** Any coding agent can work on a goal (Claude Code, Codex, OpenCode or Pi, for example), on any model provider.
- **An independent judge decides.** Agents never grade their own work. The test is fixed before the work starts, and agents can't see or change the data held back for the final check. Only changes that measurably beat what was there before count, and only measured wins become pull requests.
- **Every win has a receipt and a dollar value.** It is signed by the judge and shows what improved, by how much, what that is worth in dollars a year, and what it cost to find. It ships as a pull request for people to approve.
- **Return** is one figure, every month: the value delivered divided by everything spent to find it, including the experiments that found nothing.
- **Control.** A hard budget (agents stop when it runs out). Nothing ships without approval. Every action is logged. Antilog keeps results, never the customer's code or data.

## What is not true yet (don't imply it)

- Today, experiments run on code: cost and speed. Outcomes like conversion, forecast error or fraud loss are where the platform is going. You may show them only as clearly labelled examples.
- **Most of how the work is done is still being built.** Built today: the judge's measurement and acceptance, receipts, value and Return, approvals and budgets, and the web app. Still being built: agent teams of sessions, support for each coding agent, and testing in throwaway environments. Describe these as how Antilog works, not as a feature list with dates, and make no claim that they are live.
- **Any cloud is the design, not yet shipped.** Antilog's own service starts on AWS. Say "built to run on any cloud" or "no tie to one cloud", never that it runs on every cloud today.
- Name coding agents only as examples. Don't show a lineup of AI vendor logos.
- There are no customers, logos, testimonials, benchmarks or statistics to show. Don't invent any. Any figure on the page must be visibly labelled **"Illustrative example"**.

## Words

- Prefer: outcome, experiment, session, judge, win, receipt, throwaway environment, the whole system, Return (with a capital R, never "ROI").
- Never: RSI, AutoML, self-improving AI, multi-cloud, ROI, leaderboard, sandbox, and the names of agent-hosting platforms.
- Never mention services, consulting or "we set it up with you".

## Actions and links

- Primary action: **Request access**. Antilog is invitation-only. Use `mailto:hello@antilog.tech?subject=Request%20access%20to%20Antilog`.
- Secondary: **Sign in**, linking to `https://app.antilog.tech`.
- The only contact is hello@antilog.tech. Put no other email address anywhere.

## Brand asset

The Antilog logo mark is below. Keep the mark itself; how you use it is up to you.

```html
<svg viewBox="218 338 600 369" width="38" height="24" aria-hidden="true"><defs><linearGradient id="brass" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#F8E4B8"/><stop offset=".6" stop-color="#E2BE7C"/><stop offset="1" stop-color="#BE8F49"/></linearGradient></defs><g fill="#5C6676"><path d="M397.8 434.8L397.8 466.2A31.5 31.5 0 0 1 366.4 434.8Z" fill-opacity=".12"/><path d="M407.7 434.8L407.7 466.2A31.5 31.5 0 0 0 439.2 434.8Z" fill-opacity=".18"/><path d="M361.6 424.9L439.2 424.9A77.6 77.6 0 0 0 361.6 347.3Z" fill-opacity=".28"/><path d="M351.7 471.0L227.9 471.0A123.8 123.8 0 0 1 351.7 347.3Z" fill-opacity=".42"/><path d="M444.0 480.9L444.0 697.0A216.1 216.1 0 0 1 227.9 480.9Z" fill-opacity=".6"/></g><path d="M453.9 342.5L453.9 697.0A354.5 354.5 0 0 0 808.4 342.5Z" fill="url(#brass)"/></svg>
```

## Must work

- **One static HTML file** with no build step. CSS, SVG and images go inline. Web fonts from Google Fonts are fine. No tracking.
- **Light and dark**, following the visitor's system setting.
- **Accessible:** real headings and landmarks, a visible keyboard focus, text alternatives for visuals, readable contrast, and respect for reduced motion.
- **Works on a phone** about 400px wide, with no sideways scrolling.
- **Fast and never broken.** It is the public face of the product, and it is the live-site evidence in a grant application due 15 Dec 2026.

## Deliverable

`index.html`, plus screenshots at desktop and phone width in light and dark. You don't publish it; the owner does.

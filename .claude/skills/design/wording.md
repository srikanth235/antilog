# Wording

The vocabulary source is [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md), including its "On screen" column. Read it before writing a word a user sees. Where it and this page disagree, the glossary wins and this page is fixed.

**Copy is signage, not conversation**: calm, concrete, specific, read at a glance. "Win merged" is the register; a sentence that pads a fact is not.

## Say and never say

| Say | Never say |
| --- | --- |
| experiment | optimization (for the object), automation, job, task |
| run (numbered `#14`) | attempt, session |
| Delivered (yearly value of merged wins) | locked in, realised |
| Pending (open pull request, not counted in Return) | unclaimed, waiting |
| Return (Delivered divided by spent, as `68×`) | ROI |
| Agent | connection (and "harness" outside code) |
| Agent order | priority |
| Note (text that steers all agents, or one with @) | comment, prompt |
| Pause Agent | Pause alone |
| Config branch (`ratchet/config`) | draft PR |
| Try again | Retry |
| connector | source |
| session (an agent's loop on an agent platform; drafting session, run session), evaluator sandbox (where Ratchet measures, no agent in it) | workspace, container, VM; "Create workspace" only as the stepper step |
| trial (one measurement of one agent's change) | candidate (on screen) |
| Kept (a trial's outcome) | Accepted, Beats noise (as an outcome; "Beats noise" stays an evidence badge on the result card) |
| evaluation | benchmark (on screen) |
| primary metric, guardrail metrics | goal metric, limits (on screen) |
| acceptance criteria, result, lineage | accept rule, verdict, receipt (on screen) |
| permission ("Can read", "No access") | grant |
| user, service principal | member, service account (on screen) |
| Restricted to evaluator | evaluator only, hidden |
| Skill (a skill pack) | skill pack on screen; never the repo's guidance files |
| agent platform (Claude Managed Agents, AWS AgentCore, OpenAI Agents API); measuring platform | runner, launcher, backend, agent host, worker |
| Ratchet cloud, Self-hosted | SaaS, on-prem |
| Infrastructure (Settings page: Agent platforms, Environments, Credentials) | Where work runs, Your infrastructure, Ratchet-managed, deployment mode, BYOC, managed cloud |
| Add platform, Test connection, Update credential, Remove | Add backend, Connect runner |
| a readable label ("Experiment created") | a raw action code (`experiment.create`) |

The glossary also defines Chat and Draft. Add each new term to the glossary in the same change that introduces it. Prefer the term Palantir Foundry or Databricks already uses; if a screen needs a word that is not in the glossary, find theirs first and add the row.

"Experiment" is the noun for the object everywhere a user reads it (settled 15 Nov 2026). "Optimization" names the basic, never the object.

The CLI and MCP tools are read by people too. They print the screen words, and may keep code names in flags and JSON keys.

## Infrastructure (target)

| Where | Words |
| --- | --- |
| Page and sections | "Infrastructure"; "Agent platforms", "Environments", "Credentials" |
| Platforms | "Claude Managed Agents", "AWS AgentCore", "OpenAI Agents API" |
| Buttons | "Add platform", "Test connection", "Update credential", "Remove" |
| Measuring platform | Badge "Measuring platform" on one platform row; field "Measuring platform" on an experiment's environment step |
| Environment | "bench-std · AWS AgentCore · 4 vCPU, 16 GiB" |
| Agent with its platform | "Claude Code · Claude Managed Agents" |
| Run history tabs | "Trials", "Sessions" |
| Clean-up state | Running, Removed, Not removed |
| Platform credential rejected | Notification title "Needs attention"; row status "Needs attention" |
| Ratchet cloud page | Read-only; a note that Ratchet manages the platforms and bills at cost through each run's budget, and per platform the data and retention, e.g. "United States only, no zero data retention" |
| Connector credentials | "Read-only. Registered once, kept in the platform's credential store. Ratchet keeps no connector secret." |
| Notification titles | Win, Needs attention, No win, Failed, Stopped, Stopped at the budget |

## The agent loop (target)

| Where | Words |
| --- | --- |
| Trial outcome tags, one per trial | Kept, Within noise, Worse, Guardrail failed, Failed holdout, Failed (did not run), Measuring |
| Agent lane status | Iterating, Measuring trial 9, Paused, Stopped, no gain in 8 trials, Stopped, its budget share is spent |
| Lane facts | "11 trials · 3 kept · best 344 ms · $15 of $20" |
| A stopped agent's budget passed on | "Session ended · its unspent $6.67 went to the others" |
| Adopting | "Started from Claude Code's trial 11" |
| Stepper | 1 Start run, 2 Start sessions, 3 Hand over credentials, 4 Agents iterate: change, measure, keep or discard, 5 Pick the best kept trial, 6 Store history, open pull request, 7 Clean up |
| Stepper status line | "Step 4 of 7 · 2 agents iterating · 28 trials, 4 kept." |
| Steps for this trial | Hypothesis, Wrote the change, Checked it in its session (does not count), Asked for a measurement, Evaluator measured, Beat Claude Code's best by 4.7%, Confirmed in a fresh evaluator sandbox, holdout passed, Kept |
| Composer while running | "Steer the agents, or @ one, e.g. @Codex focus on database queries" |
| Pause one agent | Pause agent |

Trial numbers are per agent ("Claude Code trial 11"). The holdout value is never worded for an agent, only "Kept" or "Failed holdout".

## Budget by surface

| Surface | Budget |
| --- | --- |
| Button, action, menu row | 1 to 2 words, verb first |
| Status | a fragment ("Win merged"), never "successfully" |
| Empty state | one sentence and at most one action; no feature explanation |
| Banner (offline, degraded) | one sentence: the state, plus one action or one consequence |
| Error | what happened and what to do, about 12 words; no apology, no "Please try again" |
| Settings label and description | the label carries the meaning; the description is one sentence or absent |
| Placeholder | a short noun phrase |
| Destructive confirm, consent, security or privacy disclosure | full sentences: the one home for reassurance |

## Banned words

Banned everywhere: "please", "successfully", "simply", "in order to", "you can", "we're sorry".

## Hard caps

By string-key suffix, in words (a `{placeholder}` counts as one):

| Suffix | Words |
| --- | --- |
| `_LABEL`, `_ACTION`, `_BUTTON` | 4 |
| `_NOTE`, `_HINT`, `_STATUS` | 12 |
| `_BODY`, and errors | 25 |
| `_TITLE` | 6 |

A `·` never joins two thoughts in one string; it survives only between short metadata facts ("Edited · 3 days ago").

## Rules

- Sentence case everywhere. No exclamation marks.
- Say what happened and what to do, once, where the user first meets it.
- No exception text, error codes or stack traces in the interface.
- A control states exactly what it does: "Stop experiment", not "OK".
- A word means one thing. Do not give one object two names across pages or clients.
- Money and units are worded the same on every page.
- Defensive reassurance (a fact followed by what was not lost, deleted or sent) belongs only where the risk decision is made, in full sentences. A status or an empty state repeating the promise is noise.
- A cloud account is not a connector. Platform names (Claude Managed Agents, AWS AgentCore, OpenAI Agents API) appear under Infrastructure, beside an agent, and where an experiment names its measuring platform. Never Kubernetes.
- "Session" is an agent's loop on an agent platform, or a drafting session. It is not the experiment thread, which is the experiment.
- Model providers are a field on an agent, not a connector category.

## The words catalogue (target)

One catalogue of strings, generated into the web app's TypeScript and the Mac app's Swift, so both clients say the same thing and the caps above are checked in one place. Until code exists, the words live in the boards, and a board is where a new word is first written.

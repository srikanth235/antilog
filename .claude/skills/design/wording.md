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
| Where (This machine or Cloud) | location |
| Note (text left on a run to steer the next candidate) | comment, prompt |
| Pause Agent | Pause alone |
| Config branch (`ratchet/config`) | draft PR |
| Try again | Retry |
| connector | source |
| sandbox (and "Create workspace" only as the stepper step) | workspace, anywhere else |
| trial | candidate (on screen) |
| evaluation | benchmark (on screen) |
| primary metric, guardrail metrics | goal metric, limits (on screen) |
| acceptance criteria, result, lineage | accept rule, verdict, receipt (on screen) |
| permission ("Can read", "No access") | grant |
| user, service principal | member, service account (on screen) |
| Restricted to evaluator | evaluator only, hidden |
| Skill (a skill pack) | skill pack on screen; never the repo's guidance files |
| launcher; runner when installed for Ratchet cloud | agent host, worker |
| Ratchet cloud, Self-hosted | SaaS, on-prem |
| a readable label ("Experiment created") | a raw action code (`experiment.create`) |

The glossary also defines Chat and Draft. Add each new term to the glossary in the same change that introduces it. Prefer the term Palantir Foundry or Databricks already uses; if a screen needs a word that is not in the glossary, find theirs first and add the row.

"Experiment" is the noun for the object everywhere a user reads it (settled 15 Nov 2026). "Optimization" names the basic, never the object.

The CLI and MCP tools are read by people too. They print the screen words, and may keep code names in flags and JSON keys.

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
- A cloud account is not a connector. Cloud and cluster words (AWS, GCP, Azure, Kubernetes) appear only as sandbox backends under Infrastructure and where a run names its backend.
- "Session" describes an experiment thread in code and in these skills; the screen never says it.
- Model providers are a field on an agent, not a connector category.

## The words catalogue (target)

One catalogue of strings, generated into the web app's TypeScript and the Mac app's Swift, so both clients say the same thing and the caps above are checked in one place. Until code exists, the words live in the boards, and a board is where a new word is first written.

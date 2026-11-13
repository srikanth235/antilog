# Wording

The vocabulary source is [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md). Read it before writing a word a user sees. Where it and this page disagree, the glossary wins and this page is fixed.

## Preferred and banned (from the glossary)

| Say | Never say |
| --- | --- |
| optimization | automation, job, task |
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

The glossary also defines Chat and Draft. Terms for context, infrastructure and organization (dataset, grant, workspace, role) are not in the glossary yet. Add each to the glossary in the same change that introduces it.

What the user reads for those terms, and for the areas themselves, is in [ui-terms.md](ui-terms.md) (target). It borrows the terms of Palantir Foundry and Databricks, and renames "optimization" to "experiment" on screen, which the glossary does not yet say; that conflict is the owner's to settle.

## Rules

- Sentence case everywhere. No exclamation marks.
- Say what happened and what to do, once, where the user first meets it.
- No exception text, error codes or stack traces in the interface.
- A control states exactly what it does: "Stop optimization", not "OK".
- A word means one thing. Do not give one object two names across pages or clients.
- Money and units are worded the same on every page.

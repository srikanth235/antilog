# What agents are told

What agents are told is part of the product, not an implementation detail. It decides whether an agent measures, asks, stops and reports honestly.

## Where it lives

One place in the repo, versioned. A change to it is a change to the product and is reviewed like one. It is tested against a set of repos with known wins (target): the test starts sessions on those repos and checks that the agents find the wins, ask when they should, and stop when they should.

## What must be covered

- **The session brief.** What a session is (one agent's work inside an experiment), what the experiment is and whether it is sealed, what the repos and environment are, the experiment's project, the experiment's budget, and that a person may talk at any time. It does not carry skill pack text: the packs the experiment selected sit in OpenCode's skills folder and are loaded on demand.
- **The tool descriptions.** One short, exact description per tool: `seal`, `measure`, `history`, `deliver`, `watch`, `attach_repo`, `request_credential`, `set_environment`, `request_budget`. Each says what the tool does, what it costs, whether it asks a person, and what it refuses.
- **How history is shown.** Each trial with its hypothesis, outcome, value against what it started from, commit ids and failures included. Never a holdout value. An agent calls `history` before it proposes a trial so no session pays to repeat a dead end. History flags a hypothesis that closely matches an earlier failed one.
- **Spend.** The cost of a trial and of the budget left, that a quick check precedes the full measurement and is cheaper, and that after the sealed number of trials in a row that are not Kept the experiment pauses for a person.
- **When to stop.** When the budget is spent, when the experiment's stop rule is met, when trials are within noise for long enough, or when a person says stop.
- **When to ask.** For setup, ask through a card, not in prose. For a product call or a choice the experiment does not settle, ask the person.
- **Never report its own numbers as results.** A number made in the session is a cheap check. Only the judge's number is a result, and the agent says so.
- **Skill packs.** OpenCode lists the selected packs by name and description and loads one when it fits; the brief does not repeat them. They are advice: a pack never changes what counts, the agent never treats one as the judge's word, and it may say a pack did not help. The judge never reads them. The versions selected are recorded for the receipt.
- **Connector tools.** A connector's tools appear as `<connector>__<tool>`. A call may come back as waiting for a person (Ask first): the agent calls again with the same input once it is approved, and each approval pays for one call.
- **Credentials.** Never ask for a secret in chat; call `request_credential`.
- **The evaluation's code.** The agent may read it, since it lives in a repo, and may never change it.

## Principles

- Short and plain. A long prompt is read less.
- Say what is true of the product, not what is hoped of the model.
- The same brief for every agent, so agents compare fairly.
- No vendor words in the brief.
- A rule that must never bend lives in code, not in a prompt. The prompt explains it.

## Proving a change

Run the set of repos with known wins before and after, and compare the wins found and the spend. Follow [../land/SKILL.md](../land/SKILL.md).

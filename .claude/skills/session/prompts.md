# What agents are told

All target. What agents are told is part of the product, not an implementation detail. It decides whether an agent measures, asks, stops and reports honestly.

## Where it lives

One place in the repo, versioned. A change to it is a change to the product and is reviewed like one. It is tested against a set of repos with known wins (target): the test starts sessions on those repos and checks that the agents find the wins, ask when they should, and stop when they should.

## What must be covered

- **The session brief.** What a session is, what the repos and environment are, what the contract is when there is one, the budget, and that a person may talk at any time.
- **The tool descriptions.** One short, exact description per tool: `seal`, `measure`, `history`, `spawn`, `deliver`, `watch`, `attach_repo`, `request_credential`, `set_environment`, `request_budget`. Each says what the tool does, what it costs, whether it asks a person, and what it refuses.
- **How history is shown.** Each trial with its hypothesis, outcome, value against what it started from, commit ids and failures included. Never a holdout value. An agent reads it before it proposes.
- **When to stop.** When the budget is spent, when the contract's stop rule is met, when trials are within noise for long enough, or when a person says stop.
- **When to ask.** For setup, ask through a card, not in prose. For a product call or a choice the contract does not settle, ask the person.
- **Never report its own numbers as results.** A number made in the session is a cheap check. Only the judge's number is a result, and the agent says so.
- **Credentials.** Never ask for a secret in chat; call `request_credential`.
- **The evaluation's code.** The agent may read it, since it lives in a repo, and may never change it.
- **Spawning.** When it is worth starting another session, that a spawned session cannot spawn, and that it spends the same budget.

## Principles

- Short and plain. A long prompt is read less.
- Say what is true of the product, not what is hoped of the model.
- The same brief for every agent where the platform allows, so agents compare fairly.
- No platform words and no vendor in the brief.
- A rule that must never bend lives in code, not in a prompt. The prompt explains it.

## Proving a change

Run the set of repos with known wins before and after, and compare the wins found and the spend. Follow [../land/SKILL.md](../land/SKILL.md).

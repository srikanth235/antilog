# Glossary

| Term | Means | Not | On screen (target) |
|---|---|---|---|
| Optimization | A goal Ratchet works on, with its schedule, limits and agents | automation, job, task | experiment (OPEN, see below) |
| Run | One execution of an optimization, numbered `#14` | attempt, session | run |
| Delivered | Yearly value of merged wins | locked in, realised | Delivered |
| Pending | Yearly value of wins in an open PR; not counted in Return | unclaimed, waiting | Pending |
| Return | Delivered ÷ spent, as `68×` | ROI | Return |
| Agent | A Claude Code, Codex or Grok login or key that proposes changes | harness (code only), connection | agent |
| Agent order | The fallback sequence, "Claude Code → Codex → Grok · on this machine" | priority | agent order |
| Where | This machine or Cloud: where agents run. This machine is a Mac running the macOS app (target); measuring never runs there | location | Where |
| Note | Text left on a run to steer the next candidate | comment, prompt | note |
| Pause Agent | Stop one agent inside a run | | Pause agent |
| Chat | A conversation with your own agent (Claude Code, Codex or Grok) that acts on Ratchet through its tools | assistant session, thread | Chat |
| Draft | An optimization the chat proposed as a card, not created yet | proposal | Draft |
| Config branch | `ratchet/config`, where a new optimization's files go before any PR | draft PR | config branch |

## Web client (target)

The first table is the vocabulary of the daemon, the CLI and the docs today, with the word the screen reads in its last column. The web app and the macOS app are not built; their words are shown in the boards in [`design/mockups/`](../design/mockups/) and the rules for them are in [`.claude/skills/design/wording.md`](../.claude/skills/design/wording.md). Where the two differ, the daemon, CLI and docs keep their word until a client is built, and a change that adopts a word updates the table above.

**OPEN, the owner's call:** the screen says "experiment" where the CLI and docs say "optimization". Until it is settled, boards follow the web terms below and everything else keeps "optimization".

Code, the API and the skills use the names of the three basics; the screen borrows the words people know from Palantir Foundry, Databricks and MLflow.

| Web term | Means | CLI and docs say today |
|---|---|---|
| Experiment | A goal Ratchet works on, opened as one thread | Optimization |
| Trial | One change an agent proposed and the evaluator measured | Candidate |
| Evaluation | The trusted measurement, added under Customize | Benchmark |
| Connector | A connection to a data system or the code host, with a reference to its admin secret in your secret manager. A cloud account is not a connector. Do not say "source" | Connector (a signed-in account) |
| Skill | A versioned pack of know-how agents read. Not the agent guidance in `.claude/skills/` | none |
| Plugin | A bundle that adds evaluations, skills, connectors and agents for a domain | none |
| Customize | One page with Evaluations, Skills, Connectors, Agents and Plugins | none |
| Draft | An experiment created but not approved; nothing runs until Approve | Draft (a chat card) |
| Settings | A dialog over the app, opened from the user menu or More. A rail of sections in two groups: You (Profile, Appearance, Access tokens) and Organization (General, Users and groups, Single sign-on, Infrastructure, Budgets, Notes, Audit logs) | none |
| Launcher | The one component that starts and removes sandboxes and mints their scoped credentials. Built into the server when self-hosted | none |
| Runner | The launcher installed in your infrastructure for Ratchet cloud. It connects out to the hosted server and opens no inbound port | none |
| Sandbox | One isolated place where an agent's or the evaluator's code runs, started and removed by the launcher | none |
| Sandbox backend | Where sandboxes run: Kubernetes, or VMs on AWS, GCP or Azure. Set up under Settings, Infrastructure | none |
| Scoped credential | A short-lived, read-only connector credential minted for one run. An agent's never reaches the holdout | none |

## On screen: more words

| Code says | Screen says |
|---|---|
| goal metric, limits | primary metric, guardrail metrics |
| subject, what may change | editable paths |
| accept rule, verdict, receipt | acceptance criteria, result, lineage |
| benchmark template | Evaluation, added under Customize and picked in New experiment |
| open question for a person | Needs attention |
| dataset, dataset version | dataset, version |
| grant | permission ("Can read", "No access") |
| marking, holdout | marking, Holdout |
| evaluator only | Restricted to evaluator |
| workspace | agent sandbox (the stepper step is "Create workspace"; elsewhere say sandbox) |
| deployment | Ratchet cloud or Self-hosted |
| spend cap | budget |
| role | Owner, Editor, Viewer |
| member, service account | user, service principal |
| projects, value, history | Projects, Dashboard, More then Run history (tabs Trials, Sandboxes) |
| members, sign-in, grants, audit log | Users and groups, Single sign-on, Permissions, Audit logs |
| own items, catalogue | Yours, Discover |
| running, draft, win, merged win, no win, stopped | Running, Draft, Win, Merged, No win, Stopped |
| candidate beat noise, inside noise, regressed, lost on holdout, being measured | Beats noise, Within noise, Worse, Failed holdout, Measuring |

Unchanged on screen: win, agent, note, pull request, Delivered, Pending, Return. "Skill" on screen means a skill pack; in code and docs say "skill pack" where it could be confused with this repo's guidance files. Experiment, Data and Compute name the owning area only on the eight-step stepper of a thread and as the three lineage columns of a result; they are not navigation groups.

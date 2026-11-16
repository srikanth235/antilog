# Glossary

| Term | Means | Not | On screen (target) |
|---|---|---|---|
| Experiment | A goal Ratchet works on, with its schedule, limits and agents; opened as one thread. The word in the apps, the CLI, the API, the words catalogue and user-facing docs | optimization (for the object), automation, job, task | Experiment |
| Run | One execution of an experiment, numbered `#14` | attempt, session | run |
| Delivered | Yearly value of merged wins | locked in, realised | Delivered |
| Pending | Yearly value of wins in an open PR; not counted in Return | unclaimed, waiting | Pending |
| Return | Delivered ÷ spent, as `68×` | ROI | Return |
| Agent | A model on an agent platform that proposes changes: Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AWS AgentCore (target) | harness (code only), connection | agent |
| Agent order | The fallback sequence, "Claude Code → Codex → Grok · on this machine" | priority | agent order |
| Where | The agent platform an agent runs on (target). Whether a Mac may still be "This machine" is open; measuring never runs there | location | Where |
| Note | Text that steers the agents of a run: all of them, or one with @ ("@Codex focus on database queries") | comment, prompt | note |
| Pause Agent | Stop one agent inside a run; its loop waits until resumed | | Pause agent |
| Chat | A conversation with your own agent (Claude Code, Codex or Grok) that acts on Ratchet through its tools | assistant session, thread | Chat |
| Draft | An experiment the chat proposed as a card, or created but not approved; nothing runs until Approve | proposal | Draft |
| Config branch | `ratchet/config`, where a new experiment's files go before any PR. A drafting session ends by pushing to it, and every run session starts from it | draft PR | config branch |

**"Experiment" is the noun for the thing a user creates and Ratchet works on, everywhere a user reads it:** the web and Mac apps, the CLI, the API, the words catalogue and user-facing docs (settled 15 Nov 2026). "Optimization" names one of the three basics (what "better" means: goal, limits, benchmark, accept rule) and the `optimization` skill. It is never the noun for the object. Still never "automation".

## Web client (target)

The web app and the macOS app are not built; their words are shown in the boards in [`design/mockups/`](../design/mockups/) and the rules for them are in [`.claude/skills/design/wording.md`](../.claude/skills/design/wording.md). The first table is the shared vocabulary; the last column is what the screen reads. The earlier daemon and CLI said "optimization" for the object; that is the old word, kept only in their `today.md` pages.

Code and the API use the names of the three basics for their areas; the screen borrows the words people know from Palantir Foundry, Databricks and MLflow.

| Term | Means | Earlier code and docs said |
|---|---|---|
| Trial | One measurement of one agent's change by the evaluator. A run has many per agent, numbered per agent ("Claude Code trial 11") (target) | Candidate |
| Kept | A trial's outcome: it beat its agent's best by more than the acceptance criteria require, held the guardrails and was confirmed on the holdout. The agent learns only "Kept" or "Failed holdout", never the holdout value (target) | Accepted, Beats noise (as an outcome) |
| Agent's best | The best kept value in one agent's lineage; what its next trial must beat. The run's result is the best kept trial across all agents (target) | none |
| Evaluation | The trusted measurement, added under Customize | Benchmark |
| Connector | A connection to a data system or the code host. A cloud account is not a connector. Do not say "source" | Connector (a signed-in account) |
| Skill | A versioned pack of know-how agents read. Not the agent guidance in `.claude/skills/` | none |
| Plugin | A bundle that adds evaluations, skills, connectors and agents for a domain | none |
| Customize | One page with Evaluations, Skills, Connectors, Agents and Plugins | none |
| Settings | A dialog over the app, opened from the user menu or More. A rail of sections in two groups: You (Profile, Appearance, Access tokens, and in the web app This device) and Organization (General, Users and groups, Single sign-on, Infrastructure, Budgets, Notes, Audit logs) | none |
| Agent platform | A managed service that runs an agent and its sandbox as one session: Claude Managed Agents, the OpenAI Agents API or AWS AgentCore. Set up under Settings, Infrastructure on a self-hosted server; Ratchet cloud manages its own (target) | none |
| Platform account | The account Ratchet uses on an agent platform: yours when self-hosted, Ratchet's on Ratchet cloud, billed to you at cost (target) | none |
| Session | One agent working on a platform, with its own sandbox and transcript. A drafting session works out an experiment with you; a run has one session per agent (target) | none |
| Drafting session | An interactive session where you and one agent work out an experiment. It ends by pushing the experiment's files to its config branch (target) | none |
| Sandbox | One isolated place where an agent's or the evaluator's code runs, on an agent platform, removed when its session or trial ends (target) | none |
| Evaluator sandbox | The sandbox that measures one trial, with no agent in it, on the experiment's measuring platform. The only place with the holdout credential (target) | none |
| Connector credential | A read-only credential an agent's code uses to read a connector, handed over through the platform's credential store. An agent's never reaches the holdout (target) | none |

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
| workspace | agent sandbox (the stepper step is "Create workspaces"; elsewhere say sandbox) |
| deployment | Ratchet cloud or Self-hosted |
| spend cap | budget |
| role | Owner, Editor, Viewer |
| member, service account | user, service principal |
| projects, value, history | Projects, Dashboard, More then Run history (tabs Trials, Sandboxes) |
| members, sign-in, grants, audit log | Users and groups, Single sign-on, Permissions, Audit logs |
| own items, catalogue | Yours, Discover |
| featured pick, suggestions, recent additions, list all | Featured evaluation, Featured skill pack, Featured agent, Featured plugin, Featured connectors (banner labels), For you, New, Show all, Top connectors |
| add from the catalogue, already added, browse the featured connectors | + (Add NAME), check (NAME is added), Explore |
| running, draft, win, merged win, no win, stopped | Running, Draft, Win, Merged, No win, Stopped |
| trial outcome: beat the agent's best and confirmed, inside noise, regressed, a guardrail broke, lost on holdout, did not run, being measured | Kept, Within noise, Worse, Guardrail failed, Failed holdout, Failed, Measuring ("Beats noise" and "Holdout passed" stay as evidence badges on the result card) |
| agent status in a run | Iterating, Measuring trial 9, Paused, Stopped, no gain in 8 trials, Stopped, its budget share is spent |
| an agent building on another's trial | Started from Claude Code's trial 11 |
| an agent's totals | "11 trials · 3 kept · best 344 ms · $15 of $20" |
| a stopped agent's unspent share passed on | "Sandbox removed · its unspent $6.67 went to the others" |
| steering while running | "Steer the agents, or @ one, e.g. @Codex focus on database queries" |
| signed receipt, signature verified | Signed numbers, Verified (Verify again) |
| sandbox clean-up state | Cleaned up: Running, Removed, Not removed |
| the Mac's local runner taking jobs | Accepting work (menu bar), Run agents on this Mac (Settings, This machine) |
| quit after the Mac's agents finish | Quit when idle |
| keep a stopped run stopped | Leave stopped |
| install the web app as a progressive web app (Windows, Linux) | Install app (Settings, This device); "Get the Mac app" on macOS |
| ask for push notifications after starting a run | Notify me when this run finishes |
| notification titles | Win, Needs attention, No win, Failed, Stopped, Stopped at the budget |
| the server cannot be reached | Can't reach acme.ratchet.cloud. (the server's address), Try again |
| a trial's steps | Steps for this trial (hypothesis, wrote the change, checked it in its sandbox (does not count), asked for a measurement, evaluator measured, beat the agent's best, confirmed in a fresh evaluator sandbox, Kept) |
| the settings that apply to this browser only | This device, with the lead "These apply to this browser only." |
| the server cannot be reached, what that blocks | "New experiments can't start until it is back." |
| when the offline view was last fresh | Last updated 10:42 (the time of the last good load) |
| row descriptions on This device | Install app: "Opens in its own window, with a badge for what needs attention." Notifications: "A notice here when an experiment needs you or a run finishes." |

Unchanged between code and screen: win, agent, note, pull request, Delivered, Pending, Return. "Skill" on screen means a skill pack; in code and docs say "skill pack" where it could be confused with this repo's guidance files. Experiment, Data and Compute name the owning area only on the seven-step stepper of a thread and as the three lineage columns of a result; they are not navigation groups.

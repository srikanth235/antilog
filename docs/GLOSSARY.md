# Glossary

Everything here is target; nothing is built. The design is directed by the owner, 16 Nov 2026 (see [rulings](../.claude/skills/product/rulings.md)). The first table is the shared vocabulary; the last column is what the screen reads.

| Term | Means | Not | On screen (target) |
|---|---|---|---|
| Project | A group of experiments for one area of the code. Every experiment belongs to exactly one project. It has members with a role each (owner, editor or viewer; the organization role is the default and a project can set another for that project), a description, an optional monthly cap and an optional default budget per experiment | repo, folder, team | Project |
| Experiment | The sealed goal and everything measured against it. Its definition of better is the metric and its direction, the evaluation, guardrails, the data (evaluation data and a holdout), repeats and the acceptance criteria; around it are its trials, scoreboard, wins, receipts, watch and budget. It belongs to one project and holds one or more sessions. Not sealed until a person seals it, immutable once sealed; a change makes a new version of the same experiment and numbers never carry across versions. State: Not sealed, Sealed, Watching, Regressed. Activity, from its sessions: Working, Waiting for you, Done, Failed | contract, benchmark, run | Experiment |
| Session | One agent's work inside an experiment: a chat with one agent in one sandbox on one agent platform, with any number of repos attached. It always belongs to an experiment and is not a top-level noun; the sidebar lists experiments. Status is Working, Waiting for you, Done or Failed. A session a person stops is Done, "Stopped by NAME" | run, thread, task | Named by its agent: "Claude Code session" |
| Evaluation | Two things. The command and its protected files inside a sealed experiment. And a reusable template in Customize: a kind (Speed, Quality, Size or Cost), command, direction, guardrails, the standard planted cheats and an environment, which a sealed experiment starts from a copy of | benchmark, contract, evaluator | Evaluation |
| Customize | One sidebar page, a library of reusable things, with tabs Evaluations, Skills, Connectors, Agents and Plugins, each with a Yours and a Discover toggle. Every item opens the same detail page. Setting one up in a chat with a card still works, and what a card adds appears here | settings, marketplace | Customize |
| Skill pack | A versioned pack of written know-how that agents read before they propose changes. An experiment selects the packs it uses and records the versions read. The judge never reads one, and it never holds holdout data. Not this repo's `.claude/skills/`, so docs say "skill pack" | note, instruction file, history | Skill |
| Plugin | A bundle that adds evaluations, skill packs, suggested connectors and recommended agents under their own kinds. It has no powers of its own and can never mark, unmark or grant access to a dataset. Removing one asks whether to keep its items; an item in use is never removed | extension, integration | Plugin |
| Seal, sealed | A person approves an experiment that passed the seal checks; its definition can no longer change | lock, freeze | Seal experiment, Sealed |
| Seal checks | The baseline, the noise check (the same commit measured twice) and the cheat checks (planted cheats the evaluation must reject: skip the work, hard-code the answer, return early) | validation | Seal checks, Noise check, Cheat checks |
| Judge | The part of Ratchet that measures. A trait separate from the agent platform trait | evaluator | Judge |
| Judge sandbox | A fresh sandbox, with no agent in it, where one measurement runs. The only place with the holdout credential. Removed afterwards | evaluator sandbox | Judge sandbox |
| Judge platform | The agent platform that hosts judge sandboxes | measuring platform | Judge platform |
| Trial | One `measure` call: a set of commits (one per repo changed), the trial it started from and a one-line hypothesis | candidate, attempt | Trial |
| Measurement | The judge's signed result for a trial under one experiment version | score | Measurement |
| Kept | A trial's outcome: it beat the trial it started from by more than the bar, held every guardrail and was confirmed in a fresh judge sandbox and on the holdout | Accepted | Kept |
| Win | An experiment's best Kept trial, ready to deliver or delivered | | Win |
| Receipt | The signed record of a win. Experiment: version and seal, values, repeats, noise, trials tried, the holdout result, the second opinion, what was spent. Data: dataset versions, commits, credential scopes used. Compute: judge platform and environment, judge image and digest, agents and sessions | | Receipt |
| Scoreboard | Trials for an experiment, by session; pinned under the experiment page's header once it is sealed | leaderboard | Scoreboard |
| Second opinion | An agent from a different model provider than the proposer reads a winning diff and says whether it special-cases the evaluation. It flags and never decides | review | Second opinion |
| Approval, card | A tool call a person approves in the chat. Card kinds: Seal experiment, Raise budget, Add credential, Attach repo, Change environment, Open pull request | confirmation, dialog | card |
| Policy | Which card kinds are pre-approved: Always ask, Allow up to a limit, Allow. Add credential can never be pre-approved | permission | Approvals |
| Spawned session | A session another session started in the same experiment and on the same budget. It cannot spawn. It appears as a card and opens as its own chat inside the experiment | child, sub-agent | Spawned session |
| Watch, regression | Measuring a delivered experiment again, on a schedule or when the default branch moves; a regression is a result worse than delivered by more than the noise | monitor | Watching, Regressed |
| Agent | A model on an agent platform: Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AWS AgentCore, GitHub Copilot on Microsoft Foundry Agent Service, Gemini on Google Agent Runtime (target) | harness (code only), connection | Agent |
| Agent platform | A managed service that runs an agent and its sandbox as one session: Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service or Google Agent Runtime | | Agent platform |
| Platform account | The account Ratchet uses on an agent platform: yours when self-hosted, Ratchet's on Ratchet cloud, billed to you at cost | | Platform account |
| Environment | The image or setup script, size and network policy a session's sandbox starts from | | Environment |
| Clean-up state | Whether a session and its sandbox are gone from the platform. Removed: the platform confirms it. Not removed: Ratchet retries and the cost counts until it is gone | Deleted, Orphaned | Removed, Not removed |
| Connector | A data system or MCP server an agent's code may read: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, Amazon S3, any MCP server. Registered once in Customize, Connectors, with two credentials and the datasets under it. Status: Connected, Reconnect, Not checked. The code host (GitHub, GitLab) is how repos attach and pull requests open. A cloud account is not a connector | source, integration | Connector |
| Dataset | A named table or file set under one connector that an experiment reads. Ratchet records its name, versions, size and marking, never its rows | table, query | Dataset |
| Dataset version | One state of a dataset, with an id and a date. It never changes after it is created; a receipt names the versions read | snapshot, latest | Dataset version ("orders v4") |
| Marking | A label on a dataset that limits who may read it. Holdout is the only one | tag | Marking |
| Agent credential | A read-only credential for one connector, on the views listed, holdout excluded. Registered once, kept in the platform's credential store, handed to a session short-lived and minted per session. Ratchet keeps none | token, key | Agent credential |
| Judge credential | A separate read-only credential for the same connector that also covers the holdout. Kept on the judge platform, minted per judge sandbox. No agent holds it | | Judge credential |
| Holdout | A marking on a dataset: evaluation data only the judge can read. Enforced by the credentials: the agent credential leaves it out and only the judge credential includes it. No control grants it to an agent. An agent never reads it or learns a value from it | test set, hidden | Restricted to the judge |
| Guardrail | A metric that must not get worse | limit | Guardrail |
| Protected path | A file in a repo that an experiment names and a trial may not change. A trial that changes one is Rejected | | Protected path |
| Budget, spend | One per experiment, covering all its sessions and their judge time: model tokens, sandbox time and judge time. A person sets it when starting and an editor or owner can change it at any time. It sits inside its project's optional monthly cap, inside the organization's monthly cap, and no budget passes either. The default per experiment can be set per project and falls back to the organization's, in Settings, Budgets | cap | Budget, Spend |
| Delivered | Yearly value of wins whose pull request is merged, derived per experiment when it states a value per unit | locked in, realised | Delivered |
| Pending | Yearly value of wins in an open pull request; not counted in Return | unclaimed, waiting | Pending |
| Return | Delivered divided by spent, as `68×` | ROI | Return |

"Experiment" is the noun for what a user creates, everywhere a user reads it: the web and Mac apps, the CLI, the API (`experiment_id`), the words catalogue and user-facing docs. A session is one agent's work inside an experiment and is never a top-level noun. "Contract" is retired (see [rulings](../.claude/skills/product/rulings.md)). Never "automation", and never "run" as a noun.

In the chat an agent's message shows the Ratchet mark and the name "Ratchet", with the agent that did the work as a quiet second label ("Ratchet · Claude Code"). Agent logos stay where an agent is the subject: spawned session cards, the Agent chip, settings rows, receipts and lineage.

## Trial outcomes

| Outcome | Means |
|---|---|
| Measuring | The judge is running it |
| Kept | Met the acceptance criteria |
| Within noise | Not better than the noise |
| Worse | Worse than what it started from |
| Guardrail failed | A guardrail got worse |
| Failed holdout | Beat the bar but failed on the holdout. The agent learns only this, never the value |
| Rejected | Changed a protected path |
| Failed | Did not run |

## Screen design (target)

The boards are in [`design/mockups/`](../design/mockups/) and the rules in [`.claude/skills/design/wording.md`](../.claude/skills/design/wording.md).

| Term | Means |
|---|---|
| Frame | A fixed 256px sidebar and a main area. Sidebar: the brand, New experiment, Search, the rows Projects and Customize, Pinned, then experiments grouped under collapsible project headings (one line each with an activity icon; under an experiment that is open or pinned, its sessions nested one level deeper, each with its status icon). The user menu at the foot has Settings, the light and dark switch, the host name and Sign out |
| New experiment | A composer, not a dialog or a form: one text box ("What do you want to improve?") with five chips, Project, Repos, Agent, Environment and Budget. The Project chip defaults to the project you are in. It starts an experiment in that project with its lead session |
| Projects | A list page of all projects, each with the person's role, experiments, wins and spend this month, and New project. Delivered, Pending and Return across experiments sit at the top |
| Project page | A detail page: title, the person's role, counts (experiments, wins), description, New experiment, an Experiments table (experiment, baseline to best, state, activity, budget and spend), Connectors used, Budget (spent this month of the project's cap, default budget per experiment) and Members. A project with no experiment has its own empty page |
| Customize page | Tabs Evaluations, Skills, Connectors, Agents, Plugins, each with Yours and Discover. One DetailPage layout for every kind of item |
| Experiment page | The chat of its lead session: a header (title, state and activity, Details, Stop, Pin, menu), the scoreboard strip once sealed (with Change budget), the conversation of messages and cards, and a composer docked at the foot. Spawned sessions appear as cards and open as their own chat inside the experiment |
| Card | The only place structured output appears. Approval cards (the six kinds above). Result cards: Trial, Spawned session, Win, Pull request, Regression, and a calm one-line failure |
| Experiments | Listed on their project's page (there is no list of all experiments; Projects shows Delivered, Pending and Return across them), and a detail page, reached from Details, whose back link goes to its project (the sealed definition, the seal checks, the data with dataset versions, the holdout shown as "Restricted to the judge", the sessions with their spend and clean-up state, the scoreboard, the history of trials across sessions, wins and their pull requests, watch settings, versions) |
| Experiment state | Not sealed, Sealed, Watching, Regressed |
| Receipt page | One win: everything on the signed receipt, and the lineage (what each trial started from) |
| Settings | A dialog with a rail. You: Profile, Appearance, Access tokens, This device. Organization: General, Members, Single sign-on, Agent platforms (platform accounts only; a link to Customize, Agents), Approvals, Budgets, Connections (repos, credential references and environments approved in chats, with Revoke; a link to Customize, Connectors), Audit log. Read-only for non-owners, not hidden |
| Activity | Working, Waiting for you, Done, Failed, from an experiment's sessions; each has a word and an icon shape, never colour alone |
| Page types | Session (the experiment page), ListPage (Projects, each Customize tab), DetailPage (a project, an experiment, a receipt, and one layout for every Customize item), Dialog, SettingsSection, SignIn |
| Page states | Loading, empty, failed, refused, partial, not checked |
| Role | Owner, Editor, Viewer. The organization role is the default in every project, and a project can set another for a member. A viewer reads experiments and approves nothing. Project owners manage the project's members and budget; organization owners manage organization settings |
| Member, service account | User, service principal |
| Install the web app (Windows, Linux) | Install app (Settings, This device); "Get the Mac app" on macOS |
| The server cannot be reached | Can't reach acme.ratchet.cloud. (the server's address), Try again |
| Notification titles | Win, Waiting for you, Regression, Budget spent |

Unchanged between code and screen: win, agent, pull request, Delivered, Pending, Return, project, plugin.

Project, Customize, Skill (skill pack), Plugin and Evaluation (a template) were cut and brought back on 16 Nov 2026 (see [rulings](../.claude/skills/product/rulings.md)), so they are not retired.

## Retired words

Do not use these except in a `today.md` page about the earlier code or in a dated ruling that says what was replaced.

| Retired | Now |
|---|---|
| Contract | Experiment |
| Run, as the object | Trial (one `measure` call), or an experiment |
| Draft, drafting session, config branch | `seal` and a Seal experiment card |
| Thread | Experiment, or its session |
| Note, as a feature | Cut; talk to the agent |
| Pause agent | Stop the session, or talk to it |
| Evaluator, evaluator sandbox | Judge, judge sandbox |
| Measuring platform | Judge platform |
| Optimization, context and infrastructure as "the three basics" | Experiment, session, judge, organization floor |
| Stepper | Cards in the conversation |
| Dashboard page | Cut; the Projects page holds Delivered, Pending and Return |
| Agent order | Cut; a person or an agent chooses agents in the chat |

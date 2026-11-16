# Glossary

Everything here is target; nothing is built. The design is directed by the owner, 16 Nov 2026 (see [rulings](../.claude/skills/product/rulings.md)). The first table is the shared vocabulary; the last column is what the screen reads.

| Term | Means | Not | On screen (target) |
|---|---|---|---|
| Session | A chat with one agent in one sandbox on one agent platform, with any number of repos attached. Status is Working, Waiting for you, Done or Failed. A session a person stops is Done, "Stopped by NAME" | experiment, run, thread, task | Session |
| Contract | A sealed definition of better: the metric and its direction, the evaluation, guardrails, the data (evaluation data and a holdout), repeats and the acceptance criteria. Reusable by any session. Immutable once sealed; a change makes a new version and numbers never carry across versions | benchmark, experiment | Contract |
| Evaluation | The command and its protected files inside a contract | benchmark, contract | Evaluation |
| Seal, sealed | A person approves a contract that passed the seal checks; it can no longer change | lock, freeze | Seal contract, Sealed |
| Seal checks | The baseline, the noise check (the same commit measured twice) and the cheat checks (planted cheats the evaluation must reject: skip the work, hard-code the answer, return early) | validation | Seal checks, Noise check, Cheat checks |
| Judge | The part of Ratchet that measures. A trait separate from the agent platform trait | evaluator | Judge |
| Judge sandbox | A fresh sandbox, with no agent in it, where one measurement runs. The only place with the holdout credential. Removed afterwards | evaluator sandbox | Judge sandbox |
| Judge platform | The agent platform that hosts judge sandboxes | measuring platform | Judge platform |
| Trial | One `measure` call: a set of commits (one per repo changed), the trial it started from and a one-line hypothesis | candidate, attempt | Trial |
| Measurement | The judge's signed result for a trial under one contract version | score | Measurement |
| Kept | A trial's outcome: it beat the trial it started from by more than the bar, held every guardrail and was confirmed in a fresh judge sandbox and on the holdout | Accepted | Kept |
| Win | A contract's best Kept trial, ready to deliver or delivered | | Win |
| Receipt | The signed record of a win: contract version and seal, commits, values, repeats, noise, trials tried, the holdout result, the second opinion, what was spent | | Receipt |
| Scoreboard | Trials for a contract, by session; pinned under a session's header while it works on a contract | leaderboard | Scoreboard |
| Second opinion | An agent from a different model provider than the proposer reads a winning diff and says whether it special-cases the evaluation. It flags and never decides | review | Second opinion |
| Approval, card | A tool call a person approves in the chat. Card kinds: Seal contract, Raise budget, Add credential, Attach repo, Change environment, Open pull request | confirmation, dialog | card |
| Policy | Which card kinds are pre-approved: Always ask, Allow up to a limit, Allow. Add credential can never be pre-approved | permission | Approvals |
| Spawned session | A session another session started on the same contract and budget. It cannot spawn | child, sub-agent | Spawned session |
| Watch, regression | Measuring a delivered contract again, on a schedule or when the default branch moves; a regression is a result worse than delivered by more than the noise | monitor | Watching, Regressed |
| Agent | A model on an agent platform: Claude on Claude Managed Agents, Codex on the OpenAI Agents API, Gemini or Grok on AWS AgentCore, GitHub Copilot on Microsoft Foundry Agent Service, Gemini on Google Agent Runtime (target) | harness (code only), connection | Agent |
| Agent platform | A managed service that runs an agent and its sandbox as one session: Claude Managed Agents, AWS AgentCore, the OpenAI Agents API, Microsoft Foundry Agent Service or Google Agent Runtime | | Agent platform |
| Platform account | The account Ratchet uses on an agent platform: yours when self-hosted, Ratchet's on Ratchet cloud, billed to you at cost | | Platform account |
| Environment | The image or setup script, size and network policy a session's sandbox starts from | | Environment |
| Connector | A data system or MCP server an agent's code may read: Snowflake, Databricks, DynamoDB, BigQuery, Postgres, Amazon S3, any MCP server. The code host (GitHub, GitLab) is how repos attach and pull requests open. A cloud account is not a connector | source | Connector |
| Connector credential | A read-only credential an agent's code uses to read a connector, from the platform's credential store. It never reaches the holdout | | Credential |
| Holdout | Evaluation data only the judge can read. An agent never reads it or learns a value from it | test set | Restricted to the judge |
| Guardrail | A metric that must not get worse | limit | Guardrail |
| Protected path | A file in a repo that a contract names and a trial may not change. A trial that changes one is Rejected | | Protected path |
| Budget, spend | One budget per session and the sessions it spawns: model tokens, sandbox time and judge time. An organization has a monthly cap | cap | Budget, Spend |
| Delivered | Yearly value of merged wins, derived per contract when it states a value per unit | locked in, realised | Delivered |
| Pending | Yearly value of wins in an open pull request; not counted in Return | unclaimed, waiting | Pending |
| Return | Delivered divided by spent, as `68×` | ROI | Return |

"Session" and "contract" are the nouns for what a user works with, everywhere a user reads them: the web and Mac apps, the CLI, the API, the words catalogue and user-facing docs. "Experiment" is retired as a word; whether it comes back is an open call for the owner (see [rulings](../.claude/skills/product/rulings.md)). Still never "automation".

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
| Frame | A fixed 256px sidebar and a main area. Sidebar: the brand, New session, Search, Sessions (Pinned, then recent, one line each with a status icon), Contracts. The user menu at the foot has Settings, the light and dark switch, the host name and Sign out |
| New session | A composer, not a dialog or a form: one text box with three chips, Repos, Agent and Environment |
| Session page | Header (title, status, Stop, Pin, menu), the scoreboard strip when the session works on a contract, the conversation of messages and cards, and a composer docked at the foot |
| Card | The only place structured output appears. Approval cards (the six kinds above). Result cards: Trial, Spawned session, Win, Pull request, Regression, and a calm one-line failure |
| Contracts | A list page (metric, baseline to best, state, last activity) and a detail page (the sealed definition, the seal checks, the holdout shown as "Restricted to the judge", the scoreboard, the history of trials across sessions, wins and their pull requests, watch settings, versions) |
| Contract state | Sealed, Watching, Regressed |
| Receipt page | One win: everything on the signed receipt, and the lineage (what each trial started from) |
| Settings | A dialog with a rail. You: Profile, Appearance, Access tokens, This device. Organization: General, Members, Single sign-on, Agent platforms, Approvals, Budgets, Connections (repos, credentials and environments that were approved, with Revoke), Audit log. Read-only for non-owners, not hidden |
| Session status | Working, Waiting for you, Done, Failed; each has a word and an icon shape, never colour alone |
| Page types | Session, ListPage, DetailPage, Dialog, SettingsSection, SignIn |
| Page states | Loading, empty, failed, refused, partial, not checked |
| Role | Owner, Editor, Viewer. A viewer reads sessions and contracts and approves nothing |
| Member, service account | User, service principal |
| Install the web app (Windows, Linux) | Install app (Settings, This device); "Get the Mac app" on macOS |
| The server cannot be reached | Can't reach acme.ratchet.cloud. (the server's address), Try again |
| Notification titles | Win, Waiting for you, Regression, Budget spent |

Unchanged between code and screen: win, agent, pull request, Delivered, Pending, Return.

## Retired words

Do not use these except in a `today.md` page about the earlier code or in a dated ruling that says what was replaced.

| Retired | Now |
|---|---|
| Experiment | Session, contract (open call) |
| Run, as the object | Session |
| Draft, drafting session, config branch | `seal` and a Seal contract card |
| Project | Cut; roles are organization-wide |
| Thread | Session |
| Customize | Cut; setup is cards, Settings lists and revokes |
| Plugin | Cut |
| Skill pack, Skill (a pack of know-how) | Cut; `history` is the memory |
| Note, as a feature | Cut; talk to the agent |
| Pause agent | Stop the session, or talk to it |
| Evaluator, evaluator sandbox | Judge, judge sandbox |
| Measuring platform | Judge platform |
| Optimization, context and infrastructure as "the three basics" | Session, judge, organization floor |
| Stepper | Cards in the conversation |

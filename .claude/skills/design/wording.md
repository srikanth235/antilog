# Wording

The vocabulary source is [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md), including its "On screen" column. Read it before writing a word a user sees. Where it and this page disagree, the glossary wins and this page is fixed. The product's own rulings are in [product](../product/rulings.md).

**Copy is signage, not conversation**: calm, concrete, specific, read at a glance. "Win merged" is the register; a sentence that pads a fact is not.

## Say and never say

| Say | Never say |
| --- | --- |
| experiment (the sealed goal and everything measured against it) | contract, benchmark, evaluation (for the whole thing; the evaluation is the command and files inside it), project |
| session (one agent's work inside an experiment, named by its agent: "Claude Code session") | thread, task, workspace, container, VM |
| Seal experiment, sealed experiment, experiment version | Lock, freeze, publish, contract |
| Seal checks (the noise check and the cheat checks) | Validation, sanity checks |
| Judge | grader, scorer |
| Judge sandbox | measuring sandbox |
| Judge platform (badge on one agent platform) | measuring platform, runner |
| trial (one `measure` call) | attempt, candidate, run |
| measurement (the judge's signed result for a trial) | score, benchmark result |
| Kept (a trial's outcome) | Accepted, Passed, Beats noise (as an outcome) |
| Win (an experiment's best Kept trial) | success, improvement |
| Receipt | proof, certificate |
| Scoreboard | leaderboard |
| Second opinion | AI review, audit |
| Open pull request | Create PR, Ship |
| Delivered (yearly value of merged wins) | locked in, realised |
| Pending (open pull request, not counted in Return) | unclaimed, waiting |
| Return (Delivered divided by spent, as `68×`) | ROI |
| Agent | connection (and "harness" outside code) |
| Spawned session | child, sub-agent, worker |
| Approve, Decline | Accept, Reject, Deny |
| Policy (which cards are pre-approved) | rules, automation |
| Watch, Watching, Regression | monitor, alert |
| Waiting for you | Blocked, Needs input, Paused |
| Stop session, "Stopped by NAME" | Pause agent, Cancel, Kill |
| Try again | Retry |
| connector (a data system or MCP server) | source, integration, plugin |
| dataset, dataset version | table (alone), snapshot, latest |
| Agent credential, Judge credential | token, key, secret |
| Removed, Not removed (clean-up state) | Deleted, Orphaned, Leaked |
| agent platform (Claude Managed Agents, AWS AgentCore, OpenAI Agents API, Microsoft Foundry Agent Service, Google Agent Runtime) | runner, launcher, backend, agent host, worker |
| environment (image, setup script, size, network) | machine, instance |
| permission ("Can read", "No access") | grant |
| Connected, Reconnect, Not checked (connector status) | Healthy, Broken, Unknown, Offline |
| user, service principal | member, service account (on screen) |
| Restricted to the judge | holdout only, hidden, evaluator only |
| Ratchet cloud, Self-hosted | SaaS, on-prem |
| Add platform, Add agent, Add connector, Test connection, Check connection, Update credential, Remove | Add backend, Connect runner, Link, Integrate |
| a readable label ("Experiment sealed") | a raw action code (`experiment.seal`) |

Retired on 16 Nov 2026, so never on screen: contract, run (as the noun for a piece of work), draft, drafting session, config branch, project, thread, Customize, plugin, skill pack, note (as a feature), Pause agent, evaluator, measuring platform, stepper, and "optimization", "context" and "infrastructure" as areas of the product. "Evaluation" survives only as the command and files inside an experiment. Add each new term to the glossary in the same change that introduces it. Prefer the term Palantir Foundry or Databricks already uses; if a screen needs a word that is not in the glossary, find theirs first and add the row.

"Experiment" is the noun for what a user creates, in the apps, the CLI, the API, the words catalogue and user-facing docs. A session is one agent's work inside it and is never a top-level noun: the sidebar lists experiments. Never "contract".

The CLI and MCP tools are read by people too. They print the screen words, and may keep code names in flags and JSON keys.

## Experiments and sessions

| Where | Words |
| --- | --- |
| Sidebar | "New experiment", "Search", "Pinned", "Experiments", "All experiments" |
| Experiment states | Not sealed, Sealed, Watching, Regressed |
| Activity (from the sessions) | Working, Waiting for you, Done, Failed |
| Experiment page header | title, state and activity, "Details", "Stop session", "Pin", menu |
| Breadcrumb in a spawned session | "Experiment title / Codex session" |
| An agent's message | the Ratchet mark and "Ratchet", then the agent as a quiet second label: "Ratchet · Claude Code", "Ratchet · Codex" |
| A session a person stopped | Done, with the line "Stopped by NAME" |
| Header actions | "Stop session" (inline confirm: "Stop session", "Keep working"), "Pin", "Unpin", menu: "Rename", "Copy link", "Delete experiment" |
| Composer, new experiment | placeholder "What do you want to improve?"; chips "Repos", "Agent", "Environment", "Budget" (shows the default, for example "$10"); button "Start" |
| Composer, Working | placeholder "Steer it, or @-mention a session: @Codex focus on database queries"; buttons "Send", "Interrupt", "Stop session" |
| Notify prompt, once after the first measurement | "Notify me when this experiment needs me"; buttons "Notify me", "Not now" |
| Composer, Waiting for you | placeholder "Answer, or approve the card above" |
| Composer, Done | placeholder "Continue this session" |
| Composer, read-only | "Viewers read experiments. Only editors and owners can send." |
| Home, waiting list | heading "Waiting for you"; each row names the card ("Raise budget", "Seal experiment") and its age |
| Suggestions under the box | an experiment's goal as a sentence ("Cut checkout p95 latency"), only where an attached repo has an experiment |
| Tool call line | verb and object, quiet: "Read 4 files", "Ran tests", "Asked the judge to measure". Chevron expands it |
| Long tool output | "Show output"; while fetching "Fetching from PLATFORM"; when gone "This output has expired on PLATFORM." |
| Platform unreachable | "Can't reach PLATFORM. Output stays there; try again when it is back." |
| Streaming | the text appears as it is written; a cursor bar ends it; no "typing" line |
| Spawned notice | "Started a session on PLATFORM with AGENT" |
| Session failed line | "The session failed on PLATFORM. Open the platform's log, or start again." |
| Sandbox ended | "The sandbox ended while idle. Sending a message starts a new one from the session's branches." |
| Notification titles | Win, Waiting for you, Regression, Budget spent, Failed |

## The judge

| Where | Words |
| --- | --- |
| Trial outcome tags, one per trial | Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected, Failed |
| Outcome notes | Rejected: "Changed a protected path"; Failed: "Did not run"; Failed holdout: "Failed the holdout" (never a value) |
| Trial card | hypothesis in the agent's words; "1,240 ms from 1,310 ms", "-5.3%"; "Started from trial 11"; "2 repos, 6 files"; guardrails "p99 held", "Error rate held" |
| Trial being measured | "Measuring. The result arrives here." |
| Scoreboard strip | "Baseline 1,310 ms", "Best 1,102 ms", "14 trials", "3 kept", "$42 of $100", "Change budget" |
| Chart | title "p95 latency per trial"; legend "Baseline", "Noise band", one line per session (named by its agent); axis "Trial" |
| Win card | title "Win: 1,310 ms to 1,102 ms"; receipt in short "Experiment v2, 3 repeats, noise ±18 ms, 14 trials tried, holdout passed, $42 spent"; second opinion line "Second opinion from PROVIDER: no special-casing found." or "Second opinion from PROVIDER flagged: SENTENCE."; button "Open pull request" |
| Open pull request preview | per repo: repo, branch, title, "6 files changed"; "Worth $X a year. Stays Pending while open."; "Nothing merges until a person merges it." |
| Spawned session spend | "$14 of $100" |
| Pull request card | "Pull request opened in 2 repos"; states Open, Checks running, Checks failed, Approved, Merged, Closed; arriving lines "Review comment from NAME on FILE" |
| Regression card | "Regression: 1,160 ms against 1,102 ms delivered, outside the 18 ms noise band." Action "Start session" |
| Seal check lines | "Baseline measured: 1,310 ms", "Noise check: ±18 ms across 3 repeats", "Cheat checks: 3 of 3 rejected" |
| Judge note on every measurement | "Measured by the judge. A number from the session never counts." |
| Holdout | "Restricted to the judge", "Agents get a credential that leaves it out"; on the dataset page "Marked Holdout. Enforced by the credentials: only the judge's includes it." |

Trial numbers are per experiment, across its sessions ("trial 11"). The holdout value is never worded for an agent, only "Kept" or "Failed holdout".

## Cards

| Card | Title | Fields, in order | Buttons | Collapsed line |
| --- | --- | --- | --- | --- |
| Seal experiment | "Seal experiment" | Metric and direction, Evaluation command, Protected paths (per repo), Data and holdout, Guardrails, Baseline, Noise check, Cheat checks, Judge platform and environment, Budget (with "Change budget") | "Seal experiment", "Decline" | "Experiment sealed by NAME · 14:02"; "Declined by NAME · 14:02" |
| Raise budget | "Raise budget" | Now, Requested, What it may cost, Why | "Raise budget", "Decline" | "Budget raised to $200 by NAME · 14:02" |
| Add credential | "Add credential" | Connector (logo and name), recipe, "What each one gets", Name, Platform, Value (secure field), Scope | "Add credential", "Decline" | "Credential added by NAME · 14:02" |
| Attach repo | "Attach repo" | Repo, Branch, Access ("Pushes only to the session's branch") | "Attach repo", "Decline" | "Repo attached by NAME · 14:02" |
| Change environment | "Change environment" | Image, Setup script, Size, Network, What changes | "Change environment", "Decline" | "Environment changed by NAME · 14:02" |
| Open pull request | "Open pull request" | Repos, Branches, Receipt, Second opinion, Cost (none) | "Open pull request", "Decline" | "Pull request opened by NAME · 14:02" |

| Where | Words |
| --- | --- |
| Add credential recipe | "Create it in Snowflake first: a read-only role on the views you list, holdout tables not granted." |
| What each one gets | "Agents: read-only on the views you list, holdout excluded. The judge: its own read-only credential that also covers the holdout." Link "Browse connectors" |
| Add credential note | "Typed here, never in the chat. Goes straight to PLATFORM's credential store." |
| Before Approve | "Nothing happens until you approve." |
| What it may cost | "May cost up to $12" or "Costs nothing" |
| Pre-approved by policy | "Allowed by policy" (collapsed line: "Budget raised to $200 · allowed by policy") |
| Waiting on an approver | "Waiting for an owner or editor to approve." |
| Viewer sees an approval card | "Only editors and owners can approve." |
| Card decided elsewhere | "Approved by NAME in another client." |

## Experiments and receipts

| Where | Words |
| --- | --- |
| Experiments list | page title "Experiments"; link "All experiments"; columns metric, baseline to best, state, last activity |
| Experiment detail page | reached from "Details"; sections "Definition", "Seal checks", "Data", "Scoreboard", "Budget", "Sessions", "Trials", "Wins", "Watch", "Versions" |
| Sessions table | columns "Session", "Platform", "Started", "Ended", "Spend", "State", "Clean-up"; state Working, Done, Failed; clean-up Removed, Not removed; Ended "Not ended" while working |
| Session not removed | "The Gemini session was not removed. Ratchet retries, and its cost counts until it is gone." |
| Evaluation kind on the Seal experiment card | kinds "Speed", "Quality", "Size", "Cost"; templates "Load test", "Query latency", "Test suite duration", "Bundle size", "Forecast error", "Your own"; line "Planted cheats: skip the work, hard-code the answer, return early" plus those for the template |
| Definition rows | "Metric", "Direction" (Lower is better, Higher is better), "Evaluation command", "Protected paths", "Guardrails", "Repeats", "Acceptance criteria" |
| Acceptance criteria line | "Kept when it beats what it started from by more than the bar, every guardrail holds, and it is confirmed in a fresh judge sandbox and on the holdout." |
| Immutable | "Sealed experiments never change. A change makes a new version, and numbers do not carry across versions." |
| New version | "New version" (an experiment version) |
| Watch settings | "Watch this experiment", "On a schedule", "When the default branch moves" |
| Delivered, Pending, Return strip on Experiments | "Delivered" (merged), "Pending" (pull request open, not in Return), "Return"; shown only when the experiment states a value per unit; otherwise absent, not zero |
| Receipt page | title "Receipt for TRIAL"; sections "Experiment", "Data", "Compute", "Commits", "Values", "Repeats and noise", "Trials tried", "Holdout", "Second opinion", "Spend", "Lineage"; "Signed by the judge" with "Verify again"; checks "The signature matches", "The signer is the judge", "The image digest matches" |
| Receipt, Data | "Dataset versions" ("orders v4"), "Commits", "Credentials used": "Agents: read-only, holdout excluded", "Judge: its own" |
| Receipt, Compute | "Judge platform and environment", "Judge image" and its digest, "Agents and sessions" with their platforms |
| Holdout on a receipt | "Passed" or "Failed", never a value |
| Lineage | "Baseline, then trial 3, trial 7, trial 11" |

## Agent platforms (target)

| Where | Words |
| --- | --- |
| Section | "Agent platforms" in Settings, Organization |
| Platforms | "Claude Managed Agents", "AWS AgentCore", "OpenAI Agents API", "Microsoft Foundry Agent Service" ("Foundry Agent Service" where space is short), "Google Agent Runtime" |
| Buttons | "Add platform", "Test connection", "Update credential", "Remove" |
| Badge | "Judge platform" on one platform row |
| Status | Connected, Needs attention |
| Environment | "bench-std · AWS AgentCore · 4 vCPU, 16 GiB" |
| Agent with its platform | "Claude Code · Claude Managed Agents" |
| Platform credential rejected | Notification title "Needs attention"; row status "Needs attention" |
| Ratchet cloud section | Read-only; a line that Ratchet manages the platforms and bills at cost through each experiment's budget, and per platform the data and retention, e.g. "United States only, no zero data retention" |
| Self-hosted, none added | "No agent platform is added yet." Action "Add platform" |
| Connector credentials | "Read-only. Registered once, kept in the platform's credential store. Ratchet keeps no connector secret." |
| Agents table | heading "Agents"; columns "Agent", "Platform", "Model", "Parallel limit", "Used in", "Last used"; "Used in 3 experiments"; action "Add agent"; none "No agent is added yet." |
| Agent page | sections "Platform", "Account", "Model", "Parallel limit", "What the agent holds", "Recent trials"; "What the agent holds": "A short-lived git token for its own branch in each repo", "Read-only connector credentials that leave out the holdout", "No model key" |

## Connectors (target)

| Where | Words |
| --- | --- |
| Section | "Connectors" in Settings, Organization, after Agent platforms |
| Connectors | "Snowflake", "Databricks", "BigQuery", "Postgres", "DynamoDB", "Amazon S3", "MCP server" |
| Row | "Snowflake · ANALYTICS", a one-line meaning, status, "Checked 2 hours ago", "Used by 3 experiments" |
| Status | Connected, Reconnect, Not checked |
| Buttons | "Add connector", "Check connection", "Check again", "Update credential", "Remove", "Browse connectors" |
| Catalogue | title "Add connector"; one card per connector with its logo and name; no "Featured", no "For you" |
| How access is scoped | title "How access is scoped"; "Credentials are registered once and kept in the platform's credential store. Ratchet keeps none."; "Agents get a read-only credential on the views listed, holdout excluded."; "The judge gets its own read-only credential that also covers the holdout, kept on the judge platform."; "Short-lived, minted per session." |
| Datasets table | columns "Dataset", "Version", "Size", "Marking", "Agents", "Judge"; "Can read", "No access"; the holdout row "Restricted to the judge" with no control |
| Add connector | "Agent credential", "Judge credential", "What each one gets"; recipes "Create it in Snowflake first: a read-only role on the views you list, holdout tables not granted." and "Judge credential: a separate read-only role that also covers the holdout."; field note "Typed here. Goes straight to the platform's credential store." |
| Error | "Can't read the credential for orders-db."; "Update the credential, then check again."; datasets "Not checked"; "3 experiments cannot measure until then." |
| Dataset page | "Versions", "v4 · today", "v3 · 3 weeks ago", "v2 · last month"; "A version never changes."; "Used by 2 experiments" |
| Empty | "No connectors yet." Action "Add connector" |

## Settings

| Where | Words |
| --- | --- |
| Rail groups | "You": Profile, Appearance, Access tokens, This device. "Organization": General, Members, Single sign-on, Agent platforms, Connectors, Approvals, Budgets, Connections, Audit log |
| Approvals | rows per card kind with the policy "Always ask", "Allow up to $10", "Allow". Add credential shows "Always asks. A person must type the value." |
| Budgets | "Default budget per experiment", "Monthly cap", "Spent this month"; a table of experiments with budget and spend; "Spend by session" |
| Budget changes | "Change budget" on the scoreboard strip, the experiment detail page and the Seal experiment card; editors and owners only |
| Connections | groups "Repos", "Credentials", "Environments"; link "Data systems are under Connectors"; action "Revoke"; line "Added by NAME · date · used by 3 experiments" |
| Audit log | columns "Who", "What", "When", "Outcome"; filter "Approvals only" |
| Read-only for non-owners | "Only owners can change this. You can read it." |
| Access tokens | "Create access token"; "Shown once. Copy it now." |
| Members | roles "Owner", "Editor", "Viewer"; "Only owners can change roles. You can read them." |
| This device | "Install app", "Get the Mac app" (target), "Notify me when an experiment needs me" |

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
| Card | a title, one line of what would happen, one line of what it may cost, two buttons |
| Destructive confirm, consent, security or privacy disclosure | full sentences: the one home for reassurance |

## Empty and error strings

| Screen | Sentence | Action |
| --- | --- | --- |
| Home, no experiments | "No experiments yet." | none; the composer is the action |
| Sidebar, no experiments | "Experiments you start appear here." | none |
| Experiments | "No experiments yet." | "New experiment" |
| Search, no match | "No results for “QUERY”." | none |
| Access tokens | "No access tokens yet." | "Create access token" |
| Agent platforms, self-hosted, none | "No agent platform is added yet." | "Add platform" |
| Connectors | "No connectors yet." | "Add connector" |
| Agents | "No agent is added yet." | "Add agent" |
| Connections | "Nothing approved yet." | none |
| Experiment, no trials | "No trials yet." | none |
| Experiment, no wins | "No wins yet." | none |
| Session, platform unreachable | "Can't reach PLATFORM. Output stays there; try again when it is back." | "Try again" |
| Sign-in refused | "This account has no access to HOST. Ask an owner to invite you." | "Use another account" |
| Offline | "Can't reach HOST. Showing what was last known." | "Try again" |

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
- A control states exactly what it does: "Stop session", not "OK"; "Seal experiment", not "Approve".
- A word means one thing. Do not give one object two names across pages or clients.
- Money and units are worded the same on every page.
- Defensive reassurance (a fact followed by what was not lost, deleted or sent) belongs only where the risk decision is made, in full sentences. A status or an empty state repeating the promise is noise.
- A cloud account is not a connector, and neither is the code host. Platform names appear under Agent platforms, beside an agent, and on the Seal experiment card as the judge platform. Never Kubernetes.
- A session is one agent's work inside an experiment, named by its agent. A spawned session is a session another session started in the same experiment.
- An agent's message is signed "Ratchet", with the agent as a quiet second label. An agent's logo appears only where the agent is the subject.
- Model providers are a field on an agent, not a connector category.
- Only the judge's numbers are worded as results. A number from a session is "checked in the session" and never shown as a result.

## The words catalogue (target)

One catalogue of strings, generated into the web app's TypeScript and, with the Mac app, its Swift, so every client says the same thing and the caps above are checked in one place. Until code exists, the words live in the boards, and a board is where a new word is first written.

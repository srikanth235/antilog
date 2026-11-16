# Wording

The vocabulary source is [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md), including its "On screen" column. Read it before writing a word a user sees. Where it and this page disagree, the glossary wins and this page is fixed. The product's own rulings are in [product](../product/rulings.md).

**Copy is signage, not conversation**: calm, concrete, specific, read at a glance. "Win merged" is the register; a sentence that pads a fact is not.

## Say and never say

| Say | Never say |
| --- | --- |
| experiment (the sealed goal and everything measured against it) | contract, benchmark, evaluation (for the whole thing; the evaluation is the command and files inside it, or a reusable template) |
| project (a group of experiments for one area of the code) | repo, folder, workspace, team |
| Customize (the library of Evaluations, Skills, Connectors, Agents and Plugins) | marketplace, store, settings |
| Skill (on screen), skill pack (in docs and code) | note, prompt, playbook |
| Plugin (a bundle of evaluations, skills, connectors and agents) | extension, add-on, integration |
| evaluation template (Evaluations, in Customize) | benchmark, preset |
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
| connector (a data system or MCP server) | source, integration |
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

Retired on 16 Nov 2026, so never on screen: contract, run (as the noun for a piece of work), draft, drafting session, config branch, thread, note (as a feature), the dashboard page, Pause agent, agent order, evaluator, measuring platform, stepper, and "optimization", "context" and "infrastructure" as areas of the product. Project, Customize, Skill ("skill pack" in docs), Plugin and Evaluation (a reusable template, and the command inside a sealed experiment) were cut and brought back the same day, so they are words again. Add each new term to the glossary in the same change that introduces it. Prefer the term Palantir Foundry or Databricks already uses; if a screen needs a word that is not in the glossary, find theirs first and add the row.

"Experiment" is the noun for what a user creates, in the apps, the CLI, the API, the words catalogue and user-facing docs. An experiment belongs to a project. A session is one agent's work inside an experiment and is never a top-level noun: the sidebar lists experiments under their project and nests an open experiment's sessions. Never "contract".

The CLI and MCP tools are read by people too. They print the screen words, and may keep code names in flags and JSON keys.

## Experiments and sessions

| Where | Words |
| --- | --- |
| Sidebar | "New experiment", "Search", "Projects", "Customize", "Pinned"; each project is a heading that folds; a session row is "Claude Code session", "Codex session" with its status |
| Experiment states | Not sealed, Sealed, Watching, Regressed |
| Activity (from the sessions) | Working, Waiting for you, Done, Failed |
| Experiment page header | breadcrumb "Project / Experiment title", title menu, state and activity, panel toggles, "More", "Share" |
| Breadcrumb in a spawned session | "Project / Experiment title / Codex session" |
| An agent's message | the Ratchet mark and "Ratchet", then the agent as a quiet second label: "Ratchet · Claude Code", "Ratchet · Codex" |
| A session a person stopped | Done, with the line "Stopped by NAME" |
| Title menu (button "Session menu") | "Rename", "Pin" or "Unpin", "Experiment details", "Copy link", "Hand off to...", "Stop session" (inline confirm in the chat: "Stop session", "Keep working"), "Archive" |
| Panel toggles (label, then tooltip with the shortcut) | "Details" ("Details (⌘⇧D)"), "Trials" (⌘⇧R), "Changes" (⌘⇧G), "Terminal" (⌘⇧T), "Preview" (⌘⇧P), "Files" (⌘⇧F); a panel's close button "Close Details panel"; its drag handle "Resize panel" |
| More menu (button "More") | "Export conversation", "Keyboard shortcuts", "Report a problem" |
| Details panel | sections "Experiment" ("Sealed", "v1", "Experiment details"), "Environment" ("Change environment"), "Repos" ("Attach a repo"), "Agent", "Budget" ("$42 of $100", "Change budget"), "Sessions", "Context" ("44% used", "88k of 200k") |
| Trials panel | filter "All sessions", then the agents by name; "Show 6 earlier trials"; a row reads "Trial 12", the title, the outcome tag, the agent, "1,102 ms", "from 1,170 ms", "-5.8%" |
| Changes panel | picker "Trial 12, Kept" or "Working copy"; "2 repos, 6 files"; "Protected: a change here is Rejected" |
| Terminal panel | "Agent sandbox · Claude Managed Agents"; "The judge never runs here. Measurements happen in a fresh judge sandbox."; "Type a command and press Return"; a viewer sees "Read-only. Only editors and owners can type here." |
| Preview panel | "Reload", "Open in a new tab", "Port 3000"; "Port 3000 in the agent sandbox. Only people in this experiment can open it." |
| Files panel | "In the agent sandbox", "Read-only" |
| Share popover (button "Share") | "Share", each person with "Owner", "Editor" or "Viewer" and "From Checkout platform"; "Anyone in the organization with the link can view"; "Sharing never gives more than a person's role in the project."; "Copy link" |
| Composer, new experiment | placeholder "What do you want to improve? Type / for commands"; send button "Send" (a return arrow; it starts the experiment); "+" "Attach files, images, repos or connectors"; mic "Dictate"; the row under the box: "Project" (for example "Checkout platform"), "Repos", "Agent" ("Auto"), "Environment" ("bench-std"), "Mode" ("Ask" or "Auto") and the budget ring ("$0 of $10") |
| Composer, Working | placeholder "Steer, @ a session, or / for commands"; the send button becomes a stop button with the label "Interrupt" (the "Stop session" action stays in the title menu); the box is one line while idle, with icon buttons "Agent: Claude Code · Default", "Environment: Checkout staging", "Mode: Auto" and "Budget: $42 of $100" (each with a tooltip of the same words); on focus or with text the row under the box shows "Agent", "Environment", "Mode" and the budget ring ("$42 of $100"); an attachment shows as a thumbnail with its file name as its label |
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
| Trial card | hypothesis in the agent's words; "1,240 ms from 1,310 ms", "-5.3%"; "Started from trial 11"; "2 repos, 6 files"; guardrails "p99 held", "Error rate held"; the cost "$5.20" (while measuring "$1.70 so far"); a trial that stopped at the quick check adds the tag "Quick check", "Stopped after 1 repeat" and "Guardrails not checked" |
| Trial being measured | "Measuring. The result arrives here." |
| Scoreboard strip | "Baseline 1,310 ms", "Best 1,102 ms", "14 trials", "3 kept", "$42 of $100", "Change budget" |
| Chart | title "p95 latency per trial"; legend "Baseline", "Noise band", one line per session (named by its agent); axis "Trial" |
| Win card | title "Win: 1,310 ms to 1,102 ms"; receipt in short "Experiment v2, 3 repeats, noise ±18 ms, 14 trials tried, holdout passed"; value lines "Worth $7,488 a year (208 ms at $36 per ms)", "Value model: each 100 ms of checkout p95 is worth $3,600 a year. Source: SOURCE. Set by NAME, project owner." and "The win cost $42, including the 11 trials that were not kept."; second opinion line "Second opinion from PROVIDER: no special-casing found." or "Second opinion from PROVIDER flagged: SENTENCE."; button "Open pull request" |
| Open pull request preview | per repo: repo, branch, title, "6 files changed"; "Worth $X a year. Stays Pending while open."; "Nothing merges until a person merges it." |
| Spawned session spend | "$14 of $100" |
| Pull request card | "Pull request opened in 2 repos"; states Open, Checks running, Checks failed, Approved, Merged, Closed; arriving lines "Review comment from NAME on FILE" |
| Regression card | "Regression: 1,160 ms against 1,102 ms delivered, outside the 18 ms noise band."; "At risk: $2,088 a year" with "58 ms at $36 per ms, taken out of the $7,488 delivered until a fix is Kept and merged."; action "Start session" |
| Value model on the Seal card | "Each 1 point of forecast error is worth $2,500 a year"; "Source: SOURCE. Set by NAME, project owner."; "Propose a different value"; "Needs a project owner's approval on this card."; no model: "No value set" |
| Expected return, cost estimate | "Goal: 14.0%, worth about $11,000 a year. Budget $10."; "One measurement costs about $0.30 (3 repeats on Sales history v7). This budget buys about 12 trials." |
| Scoreboard strip, cost | "$42 of $100", "$14.00 per kept trial" (absent while nothing is Kept) |
| Keep going? card | title "Keep going?"; "The last 6 trials did not beat 1,102 ms. $48 of $100 is left."; rows "What was tried" ("Copy as a note") and "Patience" ("6 trials in a row, set when the experiment was sealed."); "Settings, Approvals can let an experiment go on without asking."; actions "Keep going", "Stop here"; timeline line "The last 6 trials in a row were not Kept" |
| Continue on another agent? card | title "Continue on another agent?"; "The platform behind Codex is rate-limited. Your work is safe on BRANCH."; rows "Auto would choose" (agent, platform, "Lowest cost per win on Speed experiments here among the agents that are available, $21"), "It starts with", "Policy" ("Ask before continuing on another agent."); actions "Continue on Gemini", "Choose another agent", "Wait and try again"; timeline line "OpenAI Agents API is rate-limited. Tried again 3 times." |
| Agent menu | heading "Agent for this session"; "Auto" with "Auto would choose Codex: lowest cost per win on Speed experiments here, $18"; each agent with its tier ("$", "$$", "$$$") and "About $2.70 per trial here" ("Default · " before it for the default agent); "Hand off to..."; note "Tiers run from $ to $$$. Cost per trial is measured by the judge on your code." |
| Hand off | timeline line "Priya Raman handed the Claude Code session off to Codex. Same experiment, same branches."; the ended session "Done", "Handed off to Codex"; the summary card "Summary from Claude Code session" with rows "Goal", "Done", "From history", "Where it stopped" |
| Details panel, Context | "44% used", "88k of 200k", "Compact", "Fresh session with summary"; timeline line "Compacted the conversation automatically at 80% of the context window. 44% used now." |
| Ideas on Home | heading "Ideas" with "Ratchet suggests, ranked by expected return. You start."; columns "Experiment", "Worth a year", "Costs about", "Ratio" ("95×"); button "Start" |
| Seal check lines | "Baseline measured: 1,310 ms", "Noise check: ±18 ms across 3 repeats", "Cheat checks: 3 of 3 rejected" |
| Judge note on every measurement | "Measured by the judge. A number from the session never counts." |
| Holdout | "Restricted to the judge", "Agents get a credential that leaves it out"; on the dataset page "Marked Holdout. Enforced by the credentials: only the judge's includes it." |

Trial numbers are per experiment, across its sessions ("trial 11"). The holdout value is never worded for an agent, only "Kept" or "Failed holdout".

## Cards

| Card | Title | Fields, in order | Buttons | Collapsed line |
| --- | --- | --- | --- | --- |
| Seal experiment | "Seal experiment" | Metric and direction, Evaluation command (with "Started from Load test" when it came from a template), Protected paths (per repo), Data and holdout, Skill packs, Guardrails, Baseline, Noise check, Cheat checks, then the group "Cost and value" (drawn open, "Part of the sealed definition": Value model, "Propose a different value" ("Needs a project owner's approval on this card."), Expected return, Budget, Quick check ("On", "Never decides Kept"), Patience ("trials in a row"), Cost estimate), Judge platform and environment, Budget (with "Change budget") | "Seal experiment", "Decline" | "Experiment sealed by NAME · 14:02"; "Declined by NAME · 14:02" |
| Raise budget | "Raise budget" | Now, Requested, What it may cost, Why | "Raise budget", "Decline" | "Budget raised to $200 by NAME · 14:02" |
| Add credential | "Add credential" | Connector (logo and name), recipe, "What each one gets", Name, Platform, Value (secure field), Scope | "Add credential", "Decline" | "Credential added by NAME · 14:02" |
| Attach repo | "Attach repo" | Repo, Branch, Access ("Pushes only to the session's branch") | "Attach repo", "Decline" | "Repo attached by NAME · 14:02" |
| Change environment | "Change environment" | Image, Setup script, Size, Network, What changes | "Change environment", "Decline" | "Environment changed by NAME · 14:02" |
| Open pull request | "Open pull request" | Repos, Branches, Receipt, Second opinion, Cost (none) | "Open pull request", "Decline" | "Pull request opened by NAME · 14:02" |

| Where | Words |
| --- | --- |
| Add credential recipe | "Create it in Snowflake first: a read-only role on the views you list, holdout tables not granted." |
| What each one gets | "Agents: read-only on the views you list, holdout excluded. The judge: its own read-only credential that also covers the holdout." Link "Browse connectors" (opens Customize, Connectors, Discover) |
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
| Experiments | no list page and no "All experiments" link; a project lists its experiments, and search has an "Experiments" scope |
| Experiment detail page | reached from "Details"; sections "Definition", "Seal checks", "Data", "Scoreboard", "Budget", "Sessions", "Trials", "Wins", "Watch", "Versions" |
| Sessions table | columns "Session", "Platform", "Started", "Ended", "Spend", "State", "Clean-up"; state Working, Done, Failed; clean-up Removed, Not removed; Ended "Not ended" while working |
| Session not removed | "The Gemini session was not removed. Ratchet retries, and its cost counts until it is gone." |
| Evaluation kind on the Seal experiment card | kinds "Speed", "Quality", "Size", "Cost"; templates "Load test", "Query latency", "Test suite duration", "Bundle size", "Forecast error", "Your own"; line "Planted cheats: skip the work, hard-code the answer, return early" plus those for the template |
| Definition rows | "Metric", "Direction" (Lower is better, Higher is better), "Evaluation command", "Protected paths", "Guardrails", "Repeats", "Acceptance criteria" |
| Acceptance criteria line | "Kept when it beats what it started from by more than the bar, every guardrail holds, and it is confirmed in a fresh judge sandbox and on the holdout." |
| Immutable | "Sealed experiments never change. A change makes a new version, and numbers do not carry across versions." |
| New version | "New version" (an experiment version) |
| Watch settings | "Watch this experiment", "On a schedule", "When the default branch moves" |
| Delivered, Pending, Return strip on Projects | "Delivered" (merged), "Pending" (pull request open, not in Return), "Return"; shown only when the experiment states a value per unit; otherwise absent, not zero |
| Receipt page | title "Receipt for TRIAL"; sections "Experiment", "Data", "Compute", "Commits", "Values", "Repeats and noise", "Trials tried", "Holdout", "Second opinion", "Spend", "Lineage"; "Signed by the judge" with "Verify again"; checks "The signature matches", "The signer is the judge", "The image digest matches" |
| Receipt, Data | "Dataset versions" ("orders v4"), "Skill pack versions" ("Postgres query tuning v3"), "Commits", "Credentials used": "Agents: read-only, holdout excluded", "Judge: its own" |
| Receipt, Compute | "Judge platform and environment", "Judge image" and its digest, "Agents and sessions" with their platforms |
| Holdout on a receipt | "Passed" or "Failed", never a value |
| Lineage | "Baseline, then trial 3, trial 7, trial 11" |

## Projects (target)

| Where | Words |
| --- | --- |
| Sidebar row and page title | "Projects" |
| Projects page | columns "Project", "Your role", "Experiments", "Wins", "Spend this month"; actions "Report", "New project"; strip "Delivered", "Pending", "Spent" ("includes $79 on experiments with no win"), "Return" |
| Project page | counts "4 experiments", "2 wins"; "New experiment"; sections "Experiments", "Ideas", "Value", "Connectors used", "Budget", "Members"; "Your role: Editor" |
| Project, Ideas and Value | Ideas columns "Idea", "Metric", "Expected value", "Expected cost", "Ratio", action "Start"; Value columns "Metric", "Value model", "Source", "Set by", "Used by", action "Add value model", "No value set"; dialog "Add value model" with "One unit", "Is worth, dollars a year", "Source", "Link (optional)", "Set by" |
| Report | title "Report", picker "Project" and "Month", strip "Spent", "Delivered", "Pending", "At risk", "Return"; sections "Where the spend went", "Agent scoreboard", "Top wins", "Experiments stopped with no win"; actions "Export PDF", "Export CSV", "Email this monthly" |
| Where the spend went | rows "Kept trials", "Within noise", "Worse", "Guardrail failed", "Rejected", "Failed", "Not measured"; columns "Outcome", "Spent", "Share"; on Budgets also "Cost per win by project" |
| Agent scoreboard | on Customize, Agents and AgentDetail: columns "Experiments", "Trials", "Kept", "Wins delivered", "Cost per win", "Median time to a win", "Delivered", "Auto"; filters "Evaluation kind", "Project", "Range"; note "Measured by the judge on your code. Not a vendor benchmark."; "Default agent"; tier "$" to "$$$" with "about $2.40 per trial here". Never "leaderboard" |
| Experiments table | columns "Experiment", "Baseline to best", "State", "Activity", "Budget and spend" |
| Budget | "Spent this month", "Monthly cap", "Default budget per experiment"; "Inside the organization cap" |
| Members | column "Role in this project"; a role set for the project reads "Set for this project", otherwise "Organization role"; "A project always keeps one owner." |
| Read-only | "Only project owners can change this. You can read it." |

## Customize (target)

| Where | Words |
| --- | --- |
| Page and tabs | "Customize"; tabs "Evaluations", "Skills", "Connectors", "Agents", "Plugins"; toggle "Yours", "Discover" |
| Actions | "Add evaluation", "Add skill", "Add connector", "Add agent", "Add plugin" |
| Evaluations row | name, kind "Speed", "Quality", "Size", "Cost", direction, "Used in 3 experiments" |
| Skills row | name, "v3", "Used in 2 experiments"; on the page "Agents read it. The judge never reads a skill pack. It never holds holdout data." |
| Plugin page | "Adds" with groups "Evaluations", "Skills", "Connectors", "Agents"; "A plugin has no powers of its own. It can never mark, unmark or grant access to a dataset." |
| Remove a plugin | "Keep the items it added?"; buttons "Keep items", "Remove items"; "Items in use by an experiment stay." |
| Empty | "No evaluations yet.", "No skills yet.", "No connectors yet.", "No agent is added yet.", "No plugins installed." |

## Global search

| Where | Words |
| --- | --- |
| Sidebar row | "Search" with the hint ⌘K; it opens `SearchStart` |
| Field placeholder | "Search projects, experiments, sessions, trials and more" |
| Scopes | "All", "Projects", "Experiments", "Sessions", "Trials", "Datasets", "Customize", "People" |
| No query | groups "Needs attention", "Recents", "Actions"; actions "New experiment", "New project", "Add connector", "Open Settings" |
| Results | groups are the scope names; each row's right-hand label is its kind ("Project", "Experiment", "Session", "Trial", "Evaluation", "Skill pack") or, for a holdout, "Restricted to the judge"; the Customize group has "Show all N" |
| Footer | "Close", "Move", "Change scope", "Actions", "Open" with their keys |
| No match | "No results for “QUERY”." |

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
| Agents table | tab "Agents" in Customize; columns "Agent", "Platform", "Model", "Parallel limit", "Used in", "Last used"; "Used in 3 experiments"; action "Add agent"; none "No agent is added yet." |
| Agent page | sections "Platform", "Account", "Model", "Parallel limit", "What the agent holds", "Recent trials"; "What the agent holds": "A short-lived git token for its own branch in each repo", "Read-only connector credentials that leave out the holdout", "No model key" |

## Connectors (target)

| Where | Words |
| --- | --- |
| Tab | "Connectors" in Customize, with "Yours" and "Discover" |
| Connectors | "Snowflake", "Databricks", "BigQuery", "Postgres", "DynamoDB", "Amazon S3", "MCP server" |
| Row | "Snowflake · ANALYTICS", a one-line meaning, status, "Checked 2 hours ago", "Used by 3 experiments" |
| Status | Connected, Reconnect, Not checked |
| Buttons | "Add connector", "Check connection", "Check again", "Update credential", "Remove", "Browse connectors" |
| Discover | one card per connector with its logo and name; no "Featured", no "For you" |
| How access is scoped | title "How access is scoped"; "Credentials are registered once and kept in the platform's credential store. Ratchet keeps none."; "Agents get a read-only credential on the views listed, holdout excluded."; "The judge gets its own read-only credential that also covers the holdout, kept on the judge platform."; "Short-lived, minted per session." |
| Datasets table | columns "Dataset", "Version", "Size", "Marking", "Agents", "Judge"; "Can read", "No access"; the holdout row "Restricted to the judge" with no control |
| Add connector | "Agent credential", "Judge credential", "What each one gets"; recipes "Create it in Snowflake first: a read-only role on the views you list, holdout tables not granted." and "Judge credential: a separate read-only role that also covers the holdout."; field note "Typed here. Goes straight to the platform's credential store." |
| Error | "Can't read the credential for orders-db."; "Update the credential, then check again."; datasets "Not checked"; "3 experiments cannot measure until then." |
| Dataset page | "Versions", "v4 · today", "v3 · 3 weeks ago", "v2 · last month"; "A version never changes."; "Used by 2 experiments" |
| Empty | "No connectors yet." Action "Add connector" |

## Settings

| Where | Words |
| --- | --- |
| Rail groups | "You": Profile, Appearance, Access tokens, This device. "Organization": General, Members, Single sign-on, Agent platforms, Approvals, Budgets, Connections, Audit log |
| Approvals | eight rows, one per card kind (the six, then "Keep going?" and "Continue on another agent?"), with the policy "Always ask", "Allow up to $10", "Allow". Add credential shows "Always asks. A person must type the value." |
| Budgets | "Default budget per experiment", "Monthly cap", "Spent this month"; a table of projects and experiments with budget and spend; "Spend by session" |
| Budget changes | "Change budget" on the scoreboard strip, the experiment detail page and the Seal experiment card; editors and owners only |
| Connections | groups "Repos", "Credentials", "Environments"; link "Data systems are under Customize, Connectors"; action "Revoke"; line "Added by NAME · date · used by 3 experiments" |
| Audit log | columns "Who", "What", "When", "Outcome"; filter "Approvals only" |
| Read-only for non-owners | "Only owners can change this. You can read it." |
| Access tokens | "Create access token"; "Shown once. Copy it now." |
| Members | roles "Owner", "Editor", "Viewer"; column "Projects"; "Only owners can change roles. You can read them." |
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
| Projects | "No projects yet." | "New project" |
| Project | "No experiments in this project yet." | "New experiment" |
| Evaluations | "No evaluations yet." | "Add evaluation" |
| Skills | "No skills yet." | "Add skill" |
| Plugins | "No plugins installed." | "Add plugin" |
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
- A cloud account is not a connector, and neither is the code host. Platform names appear under Agent platforms in Settings, beside an agent, and on the Seal experiment card as the judge platform. Never Kubernetes.
- A session is one agent's work inside an experiment, named by its agent. A spawned session is a session another session started in the same experiment.
- An agent's message is signed "Ratchet", with the agent as a quiet second label. An agent's logo appears only where the agent is the subject.
- Model providers are a field on an agent, not a connector category.
- Only the judge's numbers are worded as results. A number from a session is "checked in the session" and never shown as a result.

## The words catalogue (target)

One catalogue of strings, generated into the web app's TypeScript and, with the Mac app, its Swift, so every client says the same thing and the caps above are checked in one place. Until code exists, the words live in the boards, and a board is where a new word is first written.

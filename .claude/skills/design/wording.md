# Wording

The vocabulary source is [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md), including its "On screen" column and its list of retired words. Read it before writing a word a user sees. Where it and this page disagree, the glossary wins and this page is fixed. Where a light board and this page disagree about an exact label, the board wins. The product's own rulings are in [product](../product/rulings.md).

**Copy is signage, not conversation**: calm, concrete, specific, read at a glance. "Win merged" is the register; a sentence that pads a fact is not.

"Experiment" is the noun for what a user creates, in the apps, the CLI, the API, the words catalogue and user-facing docs. An experiment belongs to a project. A session is one agent's work inside an experiment, never a top-level noun. The CLI and MCP tools print the screen words and may keep code names in flags and JSON keys.

## Say and never say

| Say | Never say |
| --- | --- |
| experiment (the sealed goal and everything measured against it) | benchmark; "evaluation" for the whole thing (the evaluation is the command and files inside it, or a reusable template) |
| project (a group of experiments for one area of the code) | repo, folder, workspace, team |
| Customize (the library of Evaluations, Skills, Connectors, Agents and Plugins) | marketplace, store |
| Skill (on screen), skill pack (in docs and code) | note, prompt, playbook |
| Plugin (a bundle of evaluations, skills, connectors and agents) | extension, add-on, integration |
| evaluation template (Evaluations, in Customize) | benchmark, preset |
| session (one agent's work inside an experiment, named by its agent: "Claude Code session") | thread, task, workspace, container, VM |
| Seal experiment, sealed experiment, experiment version | Lock, freeze, publish |
| Seal checks (the baseline, the noise check and the cheat checks) | Validation, sanity checks |
| trial (one `measure` call) | attempt, candidate |
| measurement (the judge's signed result for a trial) | score, benchmark result |
| Kept (a trial's outcome) | Accepted, Passed, Beats noise (as an outcome) |
| Win (an experiment's best Kept trial) | success, improvement |
| Receipt | proof, certificate |
| Scoreboard, Agent scoreboard | (see the glossary's retired words) |
| Second opinion | AI review, audit |
| Open pull request | Create PR, Ship |
| Delivered (yearly value of merged wins, measured offline; its line reads "measured offline") | locked in, realised |
| Pending (open pull request, not counted in Return) | unclaimed, waiting |
| Return (Delivered divided by spent, as `68×`) | (see the glossary's retired words) |
| Agent | connection |
| Auto (the agent choice only) | Auto as the approval mode |
| Ask, Follow policy (the approval mode) | Auto, automatic |
| Approve, Decline | Accept, Reject, Deny |
| Policy (which cards are pre-approved) | rules |
| Watch, Watching, Regression | monitor, alert |
| Waiting for you | Blocked, Needs input, Paused |
| Stop session, "Stopped by NAME" | Pause agent, Cancel, Kill |
| Try again | Retry |
| connector (a data system or MCP server) | source, integration |
| dataset, dataset version | table (alone), snapshot, latest |
| Agent credential, Judge credential | token, key, secret |
| Removed, Not removed (clean-up state) | Deleted, Orphaned, Leaked |
| environment (image, setup script, size, network) | machine, instance |
| permission ("Can read", "No access") | grant |
| Connected, Reconnect, Not checked (connector status) | Healthy, Broken, Unknown, Offline |
| user, service principal | member, service account (on screen) |
| Restricted to the judge | holdout only, hidden |
| Antilog cloud, Self-hosted | SaaS, on-prem |
| a readable label ("Experiment sealed") | a raw action code (`experiment.seal`) |

Add each new term to the glossary in the same change that introduces it. Prefer the term Palantir Foundry or Databricks already uses; if a screen needs a word that is not in the glossary, find theirs first and add the row.

## Sidebar and session header

| Where | Words |
| --- | --- |
| Sidebar | "New experiment", "Search" (hint ⌘K), "Projects", "Customize", "Pinned"; sessions "Claude Code session", "Codex session" |
| States, activities | Not sealed, Sealed, Watching, Regressed; Working, Waiting for you, Done, Failed |
| Header | breadcrumb "Project / Experiment title", "More", "Share" |
| An agent's message | the Antilog mark and "Antilog", then the agent as a quiet label: "Antilog · Claude Code" |
| Stopped session | Done, with "Stopped by NAME" |
| Title menu (button "Session menu") | "Rename", "Pin" or "Unpin", "Experiment details", "Copy link", "Hand off to...", "Stop session" (inline confirm: "Stop session", "Keep working"), "Archive" |
| Panel toggles | "Overview", "Trials", "Changes", "Terminal", "Preview", "Files", each with a tooltip naming its shortcut ("Terminal (⌘⇧T)"); "Close Overview panel"; "Resize panel" |
| More menu | "Export conversation", "Keyboard shortcuts", "Report a problem" |
| Overview panel | sections "Experiment" ("Experiment details"), "Environment" ("Environments", "Attach a repo"), "Context" ("44% used", "88k of 200k"), "Budget" ("$42 of $100", "Change budget"), "Connectors", "Sessions", "Outputs" ("Trials", "14 measured, 3 kept") |
| Tool calls | group "Ran 19 commands (1 failed)", "Edited 2 files", "Read 3 files", "Called seal 3 times", "Ran 4 steps"; running "Running… 3 commands"; row "Ran cargo test", "Edited src/a.rs", "Failed"; detail labels "Command", "Input", "Output", "Copy" |
| Trials panel | "14 across 3 sessions"; filter "All sessions"; "Show 6 earlier trials" |
| Changes panel | "Against the base of BRANCH"; "Trial 12, Kept" or "Working copy"; "Protected: a change here is Rejected." |
| Terminal panel | "Agent machine"; "The judge never runs here. Measurements run the sealed test in a fresh checkout."; "Type a command and press Return"; viewer: "Read-only. Only editors and owners can type here." |
| Preview, Files | "Reload", "Open in a new tab", "Port 3000 in the session's machine. Only people in this experiment can open it."; "In the session's machine", "Read-only" |
| Share popover | roles "Owner", "Editor", "Viewer" with "From Checkout platform"; "Anyone in the organization with the link can view"; "Sharing never gives more than a person's role in the project."; "Copy link" |

## Composer and conversation

| Where | Words |
| --- | --- |
| New experiment | placeholder "What do you want to improve? Type / for commands"; "Send"; "Attach files, images, repos or connectors"; "Dictate"; options "Project", "Repos", "Agent" ("Auto"), "Environment", "Mode" ("Ask") and the budget ring ("$0 of $10") |
| Working | placeholder "Steer, @ a session, or / for commands"; stop button "Interrupt"; icon buttons "Agent: Claude Code · Default", "Environment: Checkout staging", "Mode: Follow policy", "Budget: $42 of $100" |
| Mode values | "Ask" (every card asks), "Follow policy" (cards that policy allows go through). Never "Auto" |
| Other placeholders | Waiting for you "Answer, or approve the card above"; Done or Failed "Continue this session"; read-only "Viewers read experiments. Only editors and owners can send." |
| Notify prompt (once, after the first measurement) | "Notify me when this experiment needs me"; "Notify me", "Not now" |
| Tool line | verb and object, quiet ("Read 4 files", "Asked the judge to measure") |
| Long tool output | "Fetching from the session"; "This output has expired."; "Can't reach the session's machine. Output stays there; try again when it is back." |
| Started, failed, ended | "Started a session with AGENT"; "The session failed. Start again." ("Start again"); "The machine was released while idle. Sending a message starts a new one from the session's branches." |
| Compaction | "Compacted the conversation automatically at 80% of the context window. 44% used now." |
| Agent menu | "Agent for this session"; "Auto" with "Auto would choose Codex: lowest cost per win on Speed experiments here, $18"; tiers "$", "$$", "$$$" with "About $2.70 per trial here"; "Hand off to..."; "Tiers run from $ to $$$. Cost per trial is measured by the judge on your code." |
| Hand off | "Priya Raman handed the Claude Code session off to Codex. Same experiment, same branches."; "Handed off to Codex"; "Summary from Claude Code session" with "Goal", "Done", "From history", "Where it stopped" |
| Home | "Ideas" ("Antilog suggests, ranked by expected return. You start."; columns "Experiment", "Worth a year", "Costs about", "Ratio"; "Start"); "Waiting for you" |
| Notification titles | Win, Waiting for you, Regression, Budget spent, Failed |

## Judge words in the chat

| Where | Words |
| --- | --- |
| Outcome tags | Measuring, Kept, Within noise, Worse, Guardrail failed, Failed holdout, Rejected, Failed. Notes: "Changed a protected path", "Did not run", "Failed on the holdout" (never a value) |
| Trial card | "1,102 ms from 1,170 ms", "-5.8%", "Started from trial 7", "2 repos, 6 files", "p99 held", cost "$5.20" ("$1.70 so far"); quick check adds "Quick check", "Stopped after 1 repeat", "Guardrails not checked"; while measuring "Measuring. The result arrives here." |
| Scoreboard strip | "Checkout p95 latency, lower is better", "1,310 ms to 1,102 ms", "-15.9%", "14 trials", "3 kept", "$42 of $100", "$14.00 per kept trial", "Change budget" |
| Chart | "p95 latency per trial"; "Baseline", "Noise band"; axis "Trial" |
| Win card | "Win: 1,310 ms to 1,102 ms"; "Worth $7,488 a year (208 ms at $36 per ms)"; "Value model: each 100 ms of checkout p95 is worth $3,600 a year. Source: SOURCE. Set by NAME, project owner."; "The win cost $42, including the 11 trials that were not kept."; "Second opinion from PROVIDER: no special-casing found." (or "flagged: SENTENCE."); "View receipt"; "Open pull request" |
| Open pull request | "Stays Pending while open."; "Nothing merges until a person merges it." |
| Pull request card | "Pull request opened in 2 repos"; Open, Checks running, Checks failed, Approved, Merged, Closed; "Open on GitHub"; "Review comment from NAME on FILE" |
| Regression card | "Regression: 1,160 ms against 1,102 ms delivered, outside the 18 ms noise band."; "At risk: $2,088 a year" with "58 ms at $36 per ms, taken out of the $7,488 delivered until a fix is Kept and merged."; "Start session" |
| Seal check rows | "Baseline", "Noise check", "Cheat checks" ("3 of 3 rejected") |
| Cost and value | "Cost and value", "Part of the sealed definition"; "Value model"; "Propose a different value" ("Needs a project owner's approval on this card."); "No value set"; "Expected return" ("Goal: 14.0%, worth about $11,000 a year. Budget $10."); "Quick check" ("On", "Never decides Kept"); "Patience" ("trials in a row"); "Cost estimate" ("One measurement costs about $0.30 (3 repeats on Sales history v7). This budget buys about 12 trials.") |
| Keep going? | "The last 6 trials did not beat 1,102 ms. $48 of $100 is left."; "What was tried" ("Copy as a note"); "Patience" ("6 trials in a row, set when the experiment was sealed."); "Settings, Approvals can let an experiment go on without asking."; "Keep going", "Stop here"; timeline "The last 6 trials in a row were not Kept" |
| Continue on another agent? | "Codex is rate-limited. Your work is safe on BRANCH."; "Auto would choose", "It starts with", "Policy" ("Ask before continuing on another agent."); "Continue on Gemini", "Choose another agent", "Wait and try again"; timeline "The model is rate-limited. Tried again 3 times." |
| Judge note | "Measured by the judge. A number from the session never counts." |
| Holdout | "Restricted to the judge"; "Marked Holdout. Enforced by the credentials: only the judge's includes it." |

Trial numbers are per experiment, across its sessions. The holdout value is never worded for an agent, only "Kept" or "Failed holdout".

## Cards

| Card | Fields, in order | Buttons | Collapsed line |
| --- | --- | --- | --- |
| Seal experiment | Metric and direction, Evaluation command, Protected paths, Data and holdout, Skill packs ("Recorded on the receipt"), Guardrails, Baseline, Noise check, Cheat checks, Evaluation kind (planted cheats), "Cost and value" | "Seal experiment", "Decline" | "Experiment sealed by NAME · 14:02" |
| Raise budget | Now, Requested, What it may cost, Why | "Raise budget", "Decline" | "Budget raised to $200 by NAME · 14:02" |
| Add credential | Connector, "Create it first", Agent credential, Judge credential, "What each one gets" | "Add credential", "Decline" | "Credential added by NAME · 14:02" |
| Attach repo | Repo, Branch, Access ("Pushes only to the session's own branch") | "Attach repo", "Decline" | "Repo attached by NAME · 14:02" |
| Change environment | Image, Setup script, Size, Network, What changes | "Change environment", "Decline" | "Environment changed by NAME · 14:02" |
| Open pull request | Repos and branches, Title, Files changed, Value, Cost | "Open pull request", "Decline" | "Pull request opened by NAME · 14:02" |

Every title is the card's name. A declined card reads "Declined by NAME · 14:02". Keep going? and Continue on another agent? are the seventh and eighth kinds (above).

| Where | Words |
| --- | --- |
| Add credential | recipe "Create it first: in Snowflake, a read-only role on the views you list, with the holdout tables not granted. Then a second, separate read-only role that also covers the holdout, for the judge."; "Typed here, never in the chat. Each value goes straight to the server's sealed store."; link "Browse connectors" |
| Before Approve | "Nothing happens until you approve." |
| Cost line | "May cost up to $50" or "Costs nothing" |
| Pre-approved | "Allowed by policy" (collapsed: "Budget raised to $200 · allowed by policy") |
| Approver states | "Waiting for an owner or editor to approve."; viewer: "Only editors and owners can approve."; "Approved by NAME in another client." |

## Experiment page and receipt

| Where | Words |
| --- | --- |
| Experiment detail | "New version", "Start session", "See sessions"; strip "Best", "Trials", "Spent", "Per kept trial", "Delivered", "Pending", "Return"; sections "Definition", "Seal checks", "Data", "Value", "Scoreboard", "Where the spend went", "Sessions", "Trials", "Wins", "Watch", "Versions" |
| Definition rows | "Metric", "Direction" (Lower is better, Higher is better), "Evaluation command", "Protected paths", "Guardrails", "Repeats", "Quick check", "Patience", "Value model", "Acceptance criteria" |
| Acceptance criteria | "Kept when it beats what it started from by more than the bar, every guardrail holds, and it is confirmed in a fresh checkout and on the holdout." |
| Immutable | "Sealed experiments never change. A change makes a new version, and numbers do not carry across versions." |
| Evaluation kinds | "Speed", "Quality", "Size", "Cost"; templates "Load test", "Query latency", "Test suite duration", "Bundle size", "Forecast error", "Your own"; "Planted cheats": "Skip the work", "Hard-code the answer", "Return early" |
| Value section | "Worth", "Cost so far", "Return", "How At risk works." |
| Where the spend went | "Kept trials", "Within noise", "Worse", "Guardrail failed", "Rejected", "Failed", "Not measured"; columns "Outcome", "Spent", "Share" |
| Sessions table | "Session", "Started", "Ended", "Spend", "State", "Clean-up"; "Lead session"; Removed, Not removed; "Running", "When it ends" |
| Session not removed | "The Gemini session was not removed. Antilog retries, and its cost counts until it is gone." |
| Trials table | "Trial", "Hypothesis", "Session", "Outcome", "Value", "Cost", "Started from"; "Show all 14 trials" |
| Watch, Versions | "Watch this experiment", "On a schedule", "When the default branch moves", "Last measurement"; "Current", "Superseded"; "Numbers are never compared across versions." |
| Receipt | "Receipt for trial 12"; "Signed by the judge", "Verify again"; "The signature matches", "The signer is the judge", "The environment image matches"; sections "Experiment", "Data", "Compute", "Commits", "Values", "Repeats and noise", "Trials tried", "Holdout", "Second opinion", "Spend", "Lineage"; "Worth of this win", "Cost of the win", "Return of the win"; "Dataset versions", "Repo commits", "Agent credential", "Judge credential"; "Environment version", "Agents and sessions", "Measured on"; holdout "Passed" or "Failed", never a value; "Baseline, then trial 3, trial 7, trial 12. Each started from the one before it." |

## Projects and Report

| Where | Words |
| --- | --- |
| Projects | columns "Project", "Your role", "Experiments", "Wins", "Spend this month"; "Report", "New project"; strip "Delivered", "Pending", "Spent" ("includes $79 on experiments with no win"), "Return" |
| Project | counts "3 experiments · 2 wins"; "New experiment"; sections "Experiments" (columns "Experiment", "Baseline to best", "State", "Activity", "Spent of budget"), "Ideas" ("Idea", "Metric", "Expected value", "Expected cost", "Ratio", "Start", "No value set"), "Value" ("Metric", "Value model", "Source", "Set by", "Used by", "Add value model"), "Connectors used", "Budget", "Members" |
| Add value model | "One unit", "Is worth, dollars a year", "Source", "Link (optional)", "Set by" |
| Budget, Members | "Spent this month", "project cap", "Default per experiment"; "Experiment budgets sit inside the project cap, and the project cap sits inside the organization cap."; "Add member"; "A project always keeps one owner." |
| Read-only | "Only project owners can change this. You can read it." |
| Report | "Report", pickers "Project" and "Month"; strip "Spent", "Delivered", "Pending", "At risk", "Return"; "Where the spend went", "Agent scoreboard", "Top wins", "Experiments stopped with no win"; "Export PDF", "Export CSV", "Email this monthly" |

## Customize

| Where | Words |
| --- | --- |
| Page | "Customize"; tabs "Evaluations", "Skills", "Connectors", "Agents", "Plugins"; "Yours", "Discover"; shelves "Featured", "For you", "New", "Show all" (connectors "Featured connectors", "Top connectors"); "Add evaluation", "Add skill", "Add connector", "Add agent", "Add plugin"; row action "Configure" |
| Evaluations | columns "Evaluation", "Kind", "Seal checks", "Used in"; "Seal checks passed today"; "The template", "Check again", "Experiments using it" |
| Skills | columns "Skill pack", "Source", "Version", "Used in"; "Your team", "By Antilog"; "Versions", "What agents get", "Who can read it", "On", "Off"; "Agents read it. The judge never reads it, so a skill pack cannot change how a result is measured." |
| Plugins | columns "Plugin", "Adds", "Status" ("Added"); "A plugin runs no code and has no powers of its own. It can never mark, unmark or grant access to a dataset."; "Remove plugin" opens "Keep the items it added?" with "Keep items", "Remove items"; "Items in use by an experiment stay." |
| Connectors | columns "Connector", "Status", "Used by"; "Snowflake · ANALYTICS", "Checked 2 min ago", "Used by 3 experiments", "Not used yet"; "Snowflake", "Databricks", "BigQuery", "Postgres", "DynamoDB", "Amazon S3", "MCP server"; "The code host is under Connections"; buttons "Check connection", "Check again", "Update credential", "Remove", "Browse connectors" |
| Datasets table | "Dataset", "Version", "Marking", "Agents", "Judge"; "Can read", "No access"; holdout row "Restricted to the judge", no control; "A dataset version never changes after it is created." |
| Connector error | "Can't read the credential for orders-db."; "2 experiments cannot measure until then:"; "Not checked" |
| Agents | "Agent scoreboard"; "Default agent"; filters "Evaluation kind", "Project", "Range"; columns "Agent", "Experiments", "Trials", "Kept", "Wins delivered", "Cost per win", "Median time to a win", "Delivered", "Auto"; "about $2.90 per trial here"; "Measured by the judge on your code. Not a vendor benchmark." |
| Agent page | "Model", "Parallel limit", "Cost tier", "Turned on for Auto", "What the agent holds" ("Git", "Connectors", "Model key"), "Recent trials", "Remove agent" |

## Global search

| Where | Words |
| --- | --- |
| Field | "Search" (hint ⌘K); placeholder "Search projects, experiments, sessions, trials and more" |
| Scopes | "All", "Projects", "Experiments", "Sessions", "Trials", "Datasets", "Customize", "People" |
| No query | "Needs attention", "Recents", "Actions" ("New experiment", "New project", "Add connector", "Open Settings") |
| Results | each row's label is its kind ("Project", "Experiment", "Session", "Trial", "Evaluation", "Skill pack") or "Restricted to the judge"; "Show all N" |
| Footer | "Close", "Move", "Change scope", "Actions", "Open" with their keys |
| No match | "No results for “QUERY”." |

## Sign-in and Settings

| Where | Words |
| --- | --- |
| Sign-in | "Continue with GitHub", "Continue with Google", and on a self-hosted server "Continue with single sign-on"; "Sign-in refused."; "No access" ("This account has no access to HOST. Ask an owner to invite you."), "Use another account", "Sign out" |
| Settings, Single sign-on | "Sign-in methods", "GitHub", "Google", "Your OpenID Connect provider" (self-hosted only; "Only an owner sets it up."), "Stay signed in for" |
| Rail | "You": Profile, Appearance, Access tokens, This device. "Organization": General, Members, Single sign-on, Approvals, Budgets, Connections, Audit log |
| Approvals | "Policy by card"; "Always ask", "Allow up to a limit", "Allow"; "Always asks. A person must type the value."; "Allow never pauses" |
| Budgets | "Default budget per experiment", "Monthly cap", "Spent this month"; "Experiments, with budget and spend"; "Sessions of EXPERIMENT"; "Where the spend went"; "Cost per win by project"; "Change budget" (also on the strip, the experiment page and the Seal card; editors and owners only) |
| Connections | "Repos", "Credentials", "Environments"; "Add environment", "Edit", "Archive", "Save as version 4"; "Version 3 · changed by NAME · date"; "No network", "Limited network, 3 hosts", "Full network"; "Revoke"; "Nothing approved yet." |
| Audit log | "Who", "What", "When", "Outcome"; "All", "Approvals only" |
| Access tokens, Members | "Create access token", "Shown once. Copy it now."; Members columns "Member", "Signed in through", "Role"; "Owner", "Editor", "Viewer"; "Only owners can change roles. You can read them." |
| This device | "Install app", "Get the Mac app" (target), "Notify me when this experiment needs me" |
| Read-only | "Only owners can change this. You can read it." |

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

The empty sentences and their one action are in [behaviour](behaviour.md#empty-states). The rest:

| Screen | Sentence | Action |
| --- | --- | --- |
| Experiment, no trials or wins | "No trials yet." / "No wins yet." | none |
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
- A word means one thing. Do not give one object two names across pages or clients. "Auto" is the agent choice and nothing else; the approval mode is "Ask" or "Follow policy".
- Money and units are worded the same on every page.
- Defensive reassurance (a fact followed by what was not lost, deleted or sent) belongs only where the risk decision is made, in full sentences. A status or an empty state repeating the promise is noise.
- A cloud account is not a connector, and neither is the code host. Never Kubernetes.
- An agent's message is signed "Antilog", with the agent as a quiet second label. An agent's logo appears only where the agent is the subject.
- Model providers are a field on an agent, not a connector category.
- Only the judge's numbers are worded as results. A number from a session is "checked in the session" and never shown as a result.

## The words catalogue (target)

One catalogue of strings, generated into the web app's TypeScript and, with the Mac app, its Swift, so every client says the same thing and the caps above are checked in one place. Until code exists, the words live in the boards, and a board is where a new word is first written.

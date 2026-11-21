# Wording

The vocabulary source is [`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md), including its "On screen" column and its list of retired words. Read it before writing a word a user sees. Where it and this page disagree, the glossary wins and this page is fixed. Where a light board and this page disagree about an exact label, the board wins. The product's own rulings are in [product](../product/rulings.md).

**Copy is signage, not conversation**: calm, concrete, specific, read at a glance. "Win merged" is the register; a sentence that pads a fact is not.

"Session" is the noun for what a user starts, in the apps, the API, the words catalogue and user-facing docs. A session belongs to a project. Experiments and the judge are parked ([../judge/SKILL.md](../judge/SKILL.md)): their words are not used on any screen.

## Say and never say

| Say | Never say |
| --- | --- |
| session (one agent's work in a project, titled by its prompt) | thread, task, workspace, container, VM |
| project (a group of sessions for one area of the code) | repo, folder, workspace, team |
| Customize (the library of Skills, Connectors, Agents and Plugins) | marketplace, store |
| Skill (on screen), skill pack (in docs and code) | note, prompt, playbook |
| Plugin (a bundle of skills and connectors) | extension, add-on, integration |
| Agent | connection |
| Auto (the agent choice only) | Auto as the approval mode |
| Ask, Follow policy (the approval mode) | Auto, automatic |
| Approve, Decline | Accept, Reject, Deny |
| Policy (which cards are pre-approved) | rules |
| Waiting for you | Blocked, Needs input, Paused |
| Stop session, "Stopped by NAME" | Pause agent, Cancel, Kill |
| Try again | Retry |
| connector (a remote MCP server) | source, integration |
| Removed, Not removed (clean-up state) | Deleted, Orphaned, Leaked |
| environment (image, setup script, size, network) | machine, instance |
| Connected, Reconnect, Not checked (connector status) | Healthy, Broken, Unknown, Offline |
| user, service principal | member, service account (on screen) |
| Monthly cap, Budget (a session's own cap) | limit, quota |
| Antilog cloud, Self-hosted | SaaS, on-prem |
| a readable label ("Session stopped") | a raw action code (`session.stop`) |

Add each new term to the glossary in the same change that introduces it. Prefer the term Palantir Foundry or Databricks already uses; if a screen needs a word that is not in the glossary, find theirs first and add the row.

## Sidebar and session header

| Where | Words |
| --- | --- |
| Sidebar | "New session", "Search" (hint ⌘K), "Projects", "Customize", "Pinned"; sessions are titled by their prompt |
| Statuses | Working, Waiting for you, Done, Failed |
| Header | breadcrumb "Project / Session title", "More", "Share" |
| An agent's message | the Antilog mark and "Antilog", then the agent as a quiet label: "Antilog · Claude Code" |
| Stopped session | Done, with "Stopped by NAME" |
| Title menu (button "Session menu") | "Rename", "Pin" or "Unpin", "Copy link", "Stop session" (inline confirm: "Stop session", "Keep working"), "Archive" |
| Panel toggles | "Overview", "Changes", "Terminal", "Preview", "Files", each with a tooltip naming its shortcut ("Terminal (⌘⇧T)"); "Close Overview panel"; "Resize panel" |
| More menu | "Export conversation", "Keyboard shortcuts", "Report a problem" |
| Overview panel | sections "Environment" (the environment's name, "Attach a repo"), "Context" ("44% used", "88k of 200k"), "Spend" ("$4 of $10", "Change budget"), "Connectors", "Outputs" ("Changes", "Files", "Pull request") |
| Tool calls | group "Ran 19 commands (1 failed)", "Edited 2 files", "Read 3 files", "Called seal 3 times", "Ran 4 steps"; running "Running… 3 commands"; row "Ran cargo test", "Edited src/a.rs", "Failed"; detail labels "Command", "Input", "Output", "Copy" |
| Changes panel | "Against the base of BRANCH" |
| Terminal panel | "Agent machine"; "Type a command and press Return"; viewer: "Read-only. Only editors and owners can type here." |
| Preview, Files | "Reload", "Open in a new tab", "Port 3000 in the session's machine. Only people in this project can open it."; "In the session's machine", "Read-only" |
| Share popover | roles "Owner", "Editor", "Viewer" with "From Checkout platform"; "Sharing never gives more than a person's role in the project."; "Copy link" |

## Composer and conversation

| Where | Words |
| --- | --- |
| New session | placeholder "What should the agent do? Type / for commands"; "Send"; "Attach files, images, repos or connectors"; "Dictate"; options "Project", "Repos", "Agent" ("Auto"), "Environment", "Mode" ("Ask") and the budget ring ("$0 of $10") |
| Working | placeholder "Steer, or / for commands"; stop button "Interrupt"; icon buttons "Agent: Claude Code · Default", "Environment: Checkout staging", "Mode: Follow policy", "Budget: $4 of $10" |
| Mode values | "Ask" (every card asks), "Follow policy" (cards that policy allows go through). Never "Auto" |
| Other placeholders | Waiting for you "Answer, or approve the card above"; Done or Failed "Continue this session"; read-only "Viewers read sessions. Only editors and owners can send." |
| Notify prompt (once, after the first session) | "Notify me when this session needs me"; "Notify me", "Not now" |
| Tool line | verb and object, quiet ("Read 4 files", "Asked to attach a repo") |
| Long tool output | "Fetching from the session"; "This output has expired."; "Can't reach the session's machine. Output stays there; try again when it is back." |
| Started, failed, ended | "Started a session with AGENT"; "The session failed. Start again." ("Start again"); "The machine was released while idle. Sending a message starts a new one." |
| Compaction (target) | "Compacted the conversation automatically at 80% of the context window. 44% used now." |
| Agent menu | "Agent for this session"; "Auto" with "Auto uses your organization's default agent, Codex"; tiers "$", "$$", "$$$" |
| Home | "Waiting for you" |
| Notification titles | Waiting for you, Budget spent, Failed |

## Cards

| Card | Fields, in order | Buttons | Collapsed line |
| --- | --- | --- | --- |
| Raise budget | Now, Requested, What it may cost, Why | "Raise budget", "Decline" | "Budget raised to $20 by NAME · 14:02" |
| Add credential | Connector, Scope, Why, the secure field | "Add credential", "Decline" | "Credential added by NAME · 14:02" |
| Attach repo | Repo, Branch, Access | "Attach repo", "Decline" | "Repo attached by NAME · 14:02" |
| Change environment | Environment (with what approving does to the version), Change (a diff grouped Network, Variables, Setup script, Repositories, each saying when it applies) | "Change environment", "Decline" | "Environment changed by NAME · 14:02" |
| Allow this tool? | Tool, What it would do | "Allow once", "Decline" | "Tool allowed by NAME · 14:02" |
| Use a connector tool | Connector, Tool, Input | "Allow once", "Decline" | "Tool allowed by NAME · 14:02" |

Every title is the card's name. A declined card reads "Declined by NAME · 14:02".

| Where | Words |
| --- | --- |
| Add credential | "Typed here, never in the chat. Each value goes straight to the server's sealed store."; link "Browse connectors" |
| Before Approve | "Nothing happens until you approve." |
| Cost line | "May cost up to $50" or "Costs nothing" |
| Pre-approved | "Allowed by policy" (collapsed: "Budget raised to $20 · allowed by policy") |
| Approver states | "Waiting for an owner or editor to approve."; viewer: "Only editors and owners can approve."; "Approved by NAME in another client." |

## Projects

| Where | Words |
| --- | --- |
| Projects | columns "Project", "Your role", "Sessions", "Spend this month"; "New project" |
| Project | counts "3 sessions"; "New session"; sections "Sessions" (columns "Session", "Agent", "Status", "Spent"), "Budget", "Members" |
| Budget, Members | "Spent this month", "project cap", "Default per session"; "Session budgets sit inside the project cap, and the project cap sits inside the organization cap."; "Add member"; "A project always keeps one owner." |
| Read-only | "Only project owners can change this. You can read it." |

## Customize

| Where | Words |
| --- | --- |
| Page | "Customize"; tabs "Skills", "Connectors", "Agents", "Plugins"; "Yours", "Discover"; "Add skill", "Add connector", "Add agent", "Add plugin"; row action "Configure" |
| Skills | columns "Skill pack", "Source", "Version"; "Your team", "By Antilog"; "Versions", "What agents get"; "Every session gets the latest version." |
| Plugins | columns "Plugin", "Adds", "Status" ("Added"); "A plugin runs no code and has no powers of its own."; "Remove plugin" opens "Keep the items it added?" with "Keep items", "Remove items" |
| Connectors | columns "Connector", "Status", "Tools"; "Checked 2 min ago"; "MCP server"; "The code host is under Connections"; buttons "Check connection", "Check again", "Update credential", "Remove", "Browse connectors" |
| Connector error | "Can't read the credential for orders-db."; "Not checked" |
| Agents | "Default agent"; "Turned on for Auto"; "Agent" and "Cost tier" columns |
| Agent page | "Model", "Cost tier", "Turned on for Auto", "What the agent holds" ("Connectors", "Model key"), "Remove agent" |

## Global search

| Where | Words |
| --- | --- |
| Field | "Search" (hint ⌘K); placeholder "Search projects, sessions and more" |
| Scopes | "All", "Projects", "Sessions", "Customize", "People" |
| No query | "Needs attention", "Recents", "Actions" ("New session", "New project", "Add connector", "Open Settings") |
| Results | each row's label is its kind ("Project", "Session", "Skill pack"); "Show all N" |
| Footer | "Close", "Move", "Change scope", "Actions", "Open" with their keys |
| No match | "No results for “QUERY”." |

## Sign-in and Settings

| Where | Words |
| --- | --- |
| Sign-in | "Continue with GitHub", "Continue with Google", and on a self-hosted server "Continue with single sign-on"; "Sign-in refused."; "No access" ("This account has no access to HOST. Ask an owner to invite you."), "Use another account", "Sign out" |
| Settings, Single sign-on | "Sign-in methods", "GitHub", "Google", "Your OpenID Connect provider" (self-hosted only; "Only an owner sets it up."), "Stay signed in for" |
| Rail | "You": Profile, Appearance, Access tokens, This device. "Organization": General, Members, Single sign-on, Approvals, Budgets, Connections, Audit log |
| Approvals | "Policy by card"; "Always ask", "Allow up to a limit", "Allow"; "Always asks. A person must type the value."; "Allow never pauses" |
| Budgets | "Default budget per session", "Monthly cap", "Spent this month"; "Projects, with cap and spend"; "Change budget" (editors and owners only) |
| Connections | "Repos", "Credentials", "Environments"; "Add environment", "Edit", "Archive", "Save as version 4"; "Version 3 · changed by NAME · date"; "No network", "Limited network, 3 hosts", "Full network"; "Revoke"; "Nothing approved yet." |
| Audit log | "Who", "What", "When", "Outcome"; "All", "Approvals only" |
| Access tokens, Members | "Create access token", "Shown once. Copy it now."; Members columns "Member", "Signed in through", "Role"; "Owner", "Editor", "Viewer"; "Only owners can change roles. You can read them." |
| This device | "Install app", "Get the Mac app" (target), "Notify me when this session needs me" |
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
- A control states exactly what it does: "Stop session", not "OK"; "Raise budget", not "Approve".
- A word means one thing. Do not give one object two names across pages or clients. "Auto" is the agent choice and nothing else; the approval mode is "Ask" or "Follow policy".
- Money and units are worded the same on every page.
- Defensive reassurance (a fact followed by what was not lost, deleted or sent) belongs only where the risk decision is made, in full sentences. A status or an empty state repeating the promise is noise.
- A cloud account is not a connector, and neither is the code host. Never Kubernetes.
- An agent's message is signed "Antilog", with the agent as a quiet second label. An agent's logo appears only where the agent is the subject.
- Model providers are a field on an agent, not a connector category.

## The words catalogue (target)

One catalogue of strings, generated into the web app's TypeScript and, with the Mac app, its Swift, so every client says the same thing and the caps above are checked in one place. Until code exists, the words live in the boards, and a board is where a new word is first written.

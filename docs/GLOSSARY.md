# Glossary

The words of v0: cloud sessions only ([V0.md](V0.md)). The first table is the shared vocabulary; the last column is what the screen reads. Words of the parked judge are at the end.

| Term | Means | Not | On screen |
|---|---|---|---|
| Project | A group of sessions for one area of the code, as Claude Code's sidebar groups sessions under a project. A session belongs to exactly one project. It has members with a role each (owner, editor or viewer; the organization role is the default and a project can set another for that project), an optional monthly cap and an optional default budget per session. Creating one is an organization-owner action | repo, folder, team | Project |
| Session | A chat with one agent in a machine of its own, with any number of repos attached, started in a project. Its repos, environment version, agent and model are fixed when it starts. Status: Queued, Working, Waiting for you, Done, Stopped, Failed. A session a person stops is Stopped, "Stopped by NAME"; one no runner holds yet is Queued. Its title is the first line of its prompt unless one is given | run, thread, task | Session, titled by its prompt |
| Organization | The top owner of members, projects, sessions and settings. It has a monthly cap on spend and a cap on running sessions | a GitHub org, a cloud account | Organization |
| Customize | One sidebar page, a library of reusable things, with tabs Skills, Connectors, Agents and Plugins. Every session gets all of the organization's connectors and the latest version of each skill; there is no selection per session | settings, marketplace | Customize |
| Skill pack | A versioned `SKILL.md` folder of written text that an agent loads on demand. Every session gets the latest version of every organization skill. Not this repo's `.claude/skills/`, so docs say "skill pack" | note, instruction file | Skill |
| Plugin | An Antilog bundle (a manifest with skill packs and connectors). Adding one unpacks it into those parts, each governed as usual; it has no powers of its own. Removing one asks whether to keep each part | extension, integration | Plugin |
| Connector | A remote MCP server, added in Customize, Connectors with an address and an optional credential. The server calls it for a session, so the credential never reaches a machine. Each tool is On, Off or Ask first. Status: Connected, Reconnect, Not checked. The code host is not a connector | source, integration | Connector |
| Agent | A model run by a harness (OpenCode in v0). It runs in the session's machine and never holds a model key. `auto` picks the organization's default agent | harness (code only), connection | Agent |
| Model | `provider/model`, such as an Anthropic or OpenAI model, an OpenCode Zen model (`opencode/...`) or an OpenCode Go model (`opencode-go/...`). A session's model comes from its request, else the server's `ANTILOG_MODEL`. The customer's key for the provider stays with the proxy. A person picks it, and its Effort level (an OpenCode variant such as Medium or High), under the composer; both can change while the session runs and apply from the next message | | Model, Effort |
| Model key | An organization's own key for one provider, set by an owner, sealed on the server and used only by the proxy | access token | Model key |
| Environment | The default repos, network level (none, limited with exact hosts, full), network secrets, plain variables, setup script and size a session starts from. Every change is a new version, and a session keeps the version it started on; archiving stops offering it to new sessions | | Environment |
| Approval, card | A tool call a person approves in the chat. Card kinds (six): Raise budget, Add credential, Attach repo, Change environment, Allow this tool? (`ask_first`), Use a connector tool | confirmation, dialog | card |
| Policy | Which card kinds are pre-approved: Always ask, Allow up to a limit, Allow. Add credential can never be pre-approved | permission | Approvals |
| Mode | How a session's cards are approved, set on its composer: Ask (every card asks a person) or Follow policy (cards that policy allows go through, each still audited). Policy is the ceiling. Stored on the session as `followPolicy` | autonomy level | Composer, Settings, Approvals |
| Budget, spend | Money the model calls and machine time cost. Caps nest: a session's own optional cap is at most its project's monthly cap, which is at most the organization's monthly cap. A new session's cap defaults to the project's default budget per session, then the organization's. The proxy refuses model calls with a 402 once the session's, project's or organization's cap is spent. Spend kinds: tokens and machine | bill | Budget, Spend |
| Monthly cap | The most a project or the organization may spend in a calendar month | budget | Monthly cap |
| Running sessions cap | The most sessions an organization has on machines at once (10 by default); queued sessions wait | budget | |
| Runner, queue | An `antilog runner` on a host we keep polls the server's queue, claims a session with a lease and runs it in a container. A runner serves one organization at a time | agent, worker | |
| Role | Owner, Editor or Viewer. The organization role is the default in every project, and a project can set another for a member. A viewer reads and approves nothing; any editor of a project may start, message and stop its sessions and decide its cards | job title | Role |
| Member, service account | User, service principal | | User, service principal |
| Audit log | An append-only record of every tool call that changed something and every approval: who, what, when, outcome | log | Audit log |
| Auto | Only the agent choice (never an approval mode) | smart, default model | Auto |

"Session" is the noun for what a user starts, everywhere a user reads it: the web and Mac apps, the API (`sessionId`), the words catalogue and user-facing docs. In the chat an agent's message shows the Antilog mark and the name "Antilog", with the agent that did the work as a quiet second label ("Antilog · Claude Code").

## Screen design

The boards are in [`design/mockups/`](../design/mockups/) and the rules in [`.claude/skills/design/wording.md`](../.claude/skills/design/wording.md).

| Term | Means |
|---|---|
| Frame | A fixed 256px sidebar and a main area. Sidebar: the brand, New session, Search, the rows Projects and Customize, then projects as collapsible headings, each listing its sessions (one line each with a status icon). The user menu at the foot has Settings, the light and dark switch, the host name and Sign out |
| New session | A composer, not a dialog or a form: one text box with chips for Project, Repos, Agent, Environment and Budget. The Project chip defaults to the project you are in |
| Projects | A list page of all projects, each with the person's role, sessions and spend this month, and New project |
| Project page | A detail page: title, the person's role, description, New session, a Sessions table, Budget (spent this month of the project's cap, default budget per session) and Members |
| Session page | The chat: a header bar, the conversation of messages and cards, and a composer docked at the foot, with panels for Details, Changes, Terminal, Preview and Files |
| Tool call group | The tool calls an agent made between two messages, folded into one quiet line with a chevron; open, one row per call with its command or target, its input and its cut output. Failed calls are marked and counted | step list, log | "Ran 19 commands (1 failed)", "Edited 2 files", "Called seal 3 times", "Running… 3 commands" |
| Card | The only place structured output appears: the six approval cards above and a calm one-line failure |
| Settings | A dialog with a rail. You: Profile, Appearance, Access tokens, This device. Organization: General, Members, Single sign-on, Approvals, Budgets, Connections (environments, with a link to Customize, Connectors), Model keys, Audit log. Read-only for non-owners, not hidden |
| Activity | Queued, Working, Waiting for you, Done, Stopped, Failed, from a session's status; each has a word and an icon shape, never colour alone |
| Page types | Session, ListPage (Projects, each Customize tab), DetailPage (a project and every Customize item), Dialog, SettingsSection, SignIn |
| Page states | Loading, empty, failed, refused, partial, not checked |
| Notification titles | Waiting for you, Budget spent, Failed |

## Parked with the judge

Experiments and the judge are parked, not redesigned (owner, 21 Nov 2026). Their code is deleted from this branch; commit `daa445f` keeps it. These words are not in the product now and no screen may use them: experiment, evaluation and evaluation template, seal and seal checks, trial, measurement, Kept, win, receipt, scoreboard, second opinion, watch and regression, dataset, dataset version, marking, holdout, agent and judge credentials, guardrail, protected path, Delivered, Pending, At risk, Return, value model, quick check, patience, cost per trial, Ideas, Report, agent scoreboard, hand off, failover. When the judge returns, restore their definitions from `daa445f`.

## Retired words

Do not use these. This is the only list of retired words.

| Retired | Now |
|---|---|
| Contract | (parked: experiment) |
| Thread | Session |
| Note, as a feature | Talk to the agent |
| Stepper | Cards in the conversation |
| Dashboard page | (none) |
| Agent order | A person or an agent chooses agents in the chat, or Auto does |
| Leaderboard | (parked: agent scoreboard) |
| Automation | Nothing; say what it does |
| ROI | (parked: Return) |
| Pause agent | Stop the session, or talk to it |

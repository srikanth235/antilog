# Screen words (target)

Code, the API and these skills use the names of the three basics. The screen does not. It borrows the words people already know from popular data platforms (Palantir Foundry, Databricks, MLflow), so a new user recognises each page before reading it. Do not invent a friendlier word where one of those products already has a term.

This table is the owner's ruling for the server product's clients (target). The CLI and MCP output today still say "optimization"; follow the glossary ([`docs/GLOSSARY.md`](../../../docs/GLOSSARY.md), which also carries the web terms in a target section). Use this table for new screens and mockups, and update the glossary in the same change that adopts a word.

## Navigation

The web client (target, mockup in `design/web/`) is organised around experiments as sessions and one Customize page. It replaces the earlier four navigation groups (Experiments, Data, Compute, Admin). The three basics still own everything underneath.

| Code says | Screen says | Borrowed from |
| --- | --- | --- |
| optimization sessions | Experiments (a list in the sidebar, grouped under project headings) | Claude app sessions, MLflow |
| project | Projects | Foundry |
| value | Dashboard | Databricks |
| find anything | Search | |
| evaluations, skill packs, sources, agents, plugins | Customize, with tabs Evaluations, Skills, Connectors, Agents, Plugins | Claude app |
| own items, catalogue | Yours, Discover | Claude app |
| history | More, then Run history (tabs Trials, Instances) | Databricks |
| machines | Instances | cloud providers |
| settings | Settings, a dialog opened from the user menu or More. Group You: Profile, Appearance, Access tokens. Group Organization: General, Users and groups, Single sign-on, Budgets, Notes, Audit logs | Claude app, Databricks |
| one setting | a settings row: title, one-line description, control on the right | Claude app |
| datasets | Datasets (found by Search and chosen in New experiment; no page of its own yet) | Foundry |
| grants | Permissions | Databricks, Foundry |
| budgets | Budgets | Databricks |
| members | Users and groups | Databricks |
| sign-in | Single sign-on | Databricks |
| access tokens | Access tokens | Databricks |
| audit log | Audit logs | Databricks, Foundry |

Experiment, Data and Compute still name the owning area, but only as labels: on the eight-step stepper of a thread and as the three lineage columns of a result. They are not navigation groups. Each carries its area colour in those two places (see [tokens.md](tokens.md)).

## Terms

| Code says | Screen says | Borrowed from |
| --- | --- | --- |
| optimization | experiment | MLflow |
| run | run | MLflow |
| candidate | trial | Databricks AutoML |
| goal metric | primary metric | experimentation platforms |
| limits | guardrail metrics | experimentation platforms |
| subject, what may change | editable paths | |
| benchmark | evaluation | Databricks, MLflow |
| accept rule | acceptance criteria | |
| verdict | result | |
| receipt | lineage | Foundry, Databricks |
| source | connector | Claude app, Databricks |
| skill pack | Skill | Claude app |
| plugin | Plugin | Claude app |
| benchmark template | Evaluation (added under Customize, picked in New experiment) | Databricks, MLflow |
| session, optimization thread | experiment, opened as a thread | Claude app, MLflow |
| unapproved optimization | Draft | |
| open question for a person | Needs attention | |
| dataset, dataset version | dataset, version | Foundry, Delta |
| grant | permission ("Can read", "No access") | Databricks |
| marking, holdout | marking, Holdout | Foundry markings |
| evaluator only | Restricted to evaluator | Foundry |
| evaluator | evaluator | |
| workspace | runner | CI systems |
| environment | environment | Databricks |
| spend cap | budget | Databricks |
| organization, project | organization, project | Foundry |
| role | role: Owner, Editor, Viewer | Foundry |
| member | user | Databricks |
| service account | service principal | Databricks |
| access token | access token | Databricks |

### Statuses and outcome tags

Each status has its own icon and a word. Outcome tags are on trial cards, one per trial.

| Code says | Screen says |
| --- | --- |
| running, drafting not yet approved, win, merged win, no win, stopped | Running, Draft, Win, Merged, No win, Stopped |
| pinned experiment | Pinned (a section of the sidebar, a Pin action) |
| candidate beat noise | Beats noise |
| candidate inside noise | Within noise |
| candidate regressed | Worse |
| candidate lost on holdout | Failed holdout |
| candidate being measured | Measuring |
| open a pull request, run again | Open pull request, Run again |

"Session" is how the code and these skills describe an experiment thread. The screen never says it; wording.md still bans "session" for a run.

Unchanged: win, agent, note, pull request, Delivered, Pending, Return.

## Rules

- Prefer the term Foundry or Databricks already uses. If a screen needs a word that is not in this table, find their word first and add the row.
- "Workspace" is not used on screen. It means something else in Databricks, and here it would name a machine.
- Audit logs show a readable label ("Experiment created", "Permission changed"), not a raw action code such as `optimization.create`.
- The command line and MCP tools are read by people too. They use the screen words in what they print, and may keep code names in flags and JSON keys.
- "Skill" on screen means a skill pack, a product object that belongs to Context. It is not the agent guidance in this repo's `.claude/skills/`. In code and docs say "skill pack" where the two could be confused.
- Model providers are a field on an agent, not a connector category.
- One screen word per thing, the same in the web app (target), the CLI and MCP.

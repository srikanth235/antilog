# Glossary

| Term | Means | Not |
|---|---|---|
| Optimization | A goal Ratchet works on, with its schedule, limits and agents | automation, job, task |
| Run | One execution of an optimization, numbered `#14` | attempt, session |
| Delivered | Yearly value of merged wins | locked in, realised |
| Pending | Yearly value of wins in an open PR; not counted in Return | unclaimed, waiting |
| Return | Delivered ÷ spent, as `68×` | ROI |
| Agent | A Claude Code, Codex or Grok login or key that proposes changes | harness (code only), connection |
| Agent order | The fallback sequence, "Claude Code → Codex → Grok · on this machine" | priority |
| Where | This machine or Cloud: where agents run | location |
| Note | Text left on a run to steer the next candidate | comment, prompt |
| Pause Agent | Stop one agent inside a run | |
| Chat | A conversation with your own agent (Claude Code, Codex or Grok) that acts on Ratchet through its tools | assistant session, thread |
| Draft | An optimization the chat proposed as a card, not created yet | proposal |
| Config branch | `ratchet/config`, where a new optimization's files go before any PR | draft PR |

## Web client (target)

The table above is the vocabulary of the daemon, the CLI and the docs today. The web client is not built; its words are settled in [`.claude/skills/design/ui-terms.md`](../.claude/skills/design/ui-terms.md) and shown in [`design/web/`](../design/web/README.md). Where the two differ, the daemon, CLI and docs keep their word until the web client is built, and a change that adopts a word updates the table above.

| Web term | Means | CLI and docs say today |
|---|---|---|
| Experiment | A goal Ratchet works on, opened as one thread | Optimization |
| Trial | One change an agent proposed and the evaluator measured | Candidate |
| Evaluation | The trusted measurement, added under Customize | Benchmark |
| Connector | A connection to an outside system that datasets come from | Connector (a signed-in account) |
| Skill | A versioned pack of know-how agents read. Not the agent guidance in `.claude/skills/` | none |
| Plugin | A bundle that adds evaluations, skills, connectors and agents for a domain | none |
| Customize | One page with Evaluations, Skills, Connectors, Agents and Plugins | none |
| Draft | An experiment created but not approved; nothing runs until Approve | Draft (a chat card) |
| Settings | A dialog over the app, opened from the user menu or More. A rail of sections in two groups: You (Profile, Appearance, Access tokens) and Organization (General, Users and groups, Single sign-on, Budgets, Notes, Audit logs) | none |

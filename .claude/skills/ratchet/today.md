# Ratchet CLI: the earlier daemon

This is reference from the earlier code on the branch `claude/peaceful-lamport-leak6p`. None of it exists on this branch. The words here (optimization, run, chat) are the earlier code's; the target words are experiment and session. The target skill is [SKILL.md](SKILL.md).

`ratchet` was a client of the Ratchet daemon (`ratchetd`), the same one every client talked to, over a unix socket with a per-boot bearer token. If the daemon was down, on a Mac with the LaunchAgent installed the command asked launchd to start it; if that failed it printed what to do (`ratchetd serve`, or `ratchetd launchagent install`).

## Rules that held

- Always add `--json`. Exit 0 is ok, 1 a failure (read `error.message` and `error.hint`), 2 a usage error or a command that needs `--yes`.
- `run`, `stop`, `pr` and `env end` spent money or changed things. Without `--yes` they printed what they would do and exited 2.
- To try things without real data, a simulation: `ratchetd serve --mode sim` in one terminal, then `RATCHET_HOME=<its folder> ratchet status --json`.

## Commands

| Command | Did |
| --- | --- |
| `ratchet status --json` | The daemon and the optimizations at a glance |
| `ratchet list --json` | All optimizations with ids, status, best result, spend |
| `ratchet show <id> --json` | One optimization: agents, baseline vs best, receipt |
| `ratchet run owner/name --budget 50 --time-limit 2h --yes` | Create an optimization and start it (`--demo` ran a simulation) |
| `ratchet stop <id> --yes` | Stop one; wins so far kept |
| `ratchet pr <id> --yes` | Open the pull request with the verified wins |
| `ratchet report <id>` | The markdown report |

Others: `candidates`, `history`, `spend`, `activity`, `agent pause|resume|message|log`, `agents show|set`, `events`, `env list|end`, `cost reconcile`, `repos`, `inspect`, `overview`, `home`, `value`, `schedule`, `pr-merged`, `connectors`, and the offline `validate`, `init`, `baseline`, `templates`.

## Chat and MCP

- `ratchet chat new|send|show|stop|end` talked to Ratchet's assistant: the user's own Claude Code, Codex or Grok acting on Ratchet through its tools. `chat end` needed `--yes`. Where chat and drafting run in the target is an open question for the owner (see [product rulings](../product/rulings.md)).
- `ratchet mcp` served every command as an MCP tool on stdio; tools that needed `--yes` took `confirm: true`. Registered with `claude mcp add ratchet -- ratchet mcp`.

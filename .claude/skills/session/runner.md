# The runner

Where an agent runs (built; the proxy and a real code host are not). The decisions are in [docs/V0.md](../../../docs/V0.md), the interfaces in [docs/V0-contracts.md](../../../docs/V0-contracts.md).

## The queue

Each session has a machine row (`machines`): `queued`, `claimed`, `running`, `idle`, `released`, `done` or `failed`. `session::start` queues it with the repos and environment it asked for, and its first words as a command. A runner polls `POST /runner/poll` with the runner key (`ANTILOG_RUNNER_KEY`, compared in constant time). Each poll renews every lease the runner holds (60 seconds), claims new work up to its capacity and returns the commands for its sessions. A lapsed lease puts the session back in `queued`; its unfinished commands are sent again and the old runner is told in `lost` to stop. A session that is Done or Failed is never claimed.

- `claimed`: leased, not yet started. `running` and `idle` follow OpenCode's busy and idle status. `released`: the runner exported the session and let the machine go; a person's message, or a judge command, queues it again with `resume`.
- A claim returns the session spec: repos, environment variables, skill versions with download paths, the generated OpenCode config, the model, and three tokens (`Model`, `Mcp`, `Runner`) minted for the claim. Claiming revokes the tokens of any earlier holder; releasing revokes all of them.
- Per-session calls (`/events`, `/results`, `/release`, `/export`, the skills download) carry that session's `Runner` token. None of the `/runner/*` routes is in the public OpenAPI.
- The routes are mounted only when `ANTILOG_RUNNER_KEY` is set. `ANTILOG_MODEL` (`provider/model`) is required with it; `ANTILOG_PROXY_URL`, `ANTILOG_PUBLIC_URL`, `ANTILOG_CLONE_BASE` and `ANTILOG_SKILLS` (`name@version,...`) tell the machine where things are.

## `antilog runner`

Polls over outbound HTTPS only. For a claim it clones the repos into `<workdir>/<session>/<repo>`, writes the config to `/etc/opencode/opencode.json` (OpenCode's managed path: a repo's own `opencode.json` cannot override it, and `OPENCODE_CONFIG` does not win, so never rely on it), unpacks the skill packs into `/opt/antilog/skills`, then starts `opencode serve` on 127.0.0.1 with a random password, `OPENCODE_DISABLE_MODELS_FETCH=1` and `autoupdate: false`. One session per machine: the managed config is one file.

- Messages go in with `prompt_async`, an abort with `abort`, a person's answer to a permission with `/permission/{id}/reply`.
- `GET /event` has no replay and no event id. After every connect the runner reads `GET /session/{id}/message` (paged by `before`, cursor in `x-next-cursor`), `GET /permission` and `GET /session/status`, and sends what is new, keyed by message id and part id. Only what an agent finished saying and the name of each tool it called are sent; the server stores nothing a tool returned.
- A `permission.asked` event becomes an Ask first card (`ask_first`, always asks unless policy allows). The answer comes back as an `approval` command.
- Release: `opencode export` goes to the server with the release. A later claim fetches it and runs `opencode import`, so the agent has its history. If the stream cannot be kept the session is released as failed.
- The judge's sealed command is a `run` command. The runner checks out each commit in a `git worktree` beside the clone, puts each protected file back by its hash (looked for in the repo's history, never taken from the trial; a hash that is nowhere is a failed run), and runs the command with `ANTILOG_DATA_KIND`, `ANTILOG_DATASET_ID` and `ANTILOG_DATASET_VERSION` set. The exit status and output go back through `/results`. Delivering the data itself is not built.
- The runner key is removed from the environment of everything it starts.

## What a repo can still do

Run against OpenCode 1.19.6 with the managed config in place:

| | repo `opencode.json` provider, permission, autoupdate | repo MCP servers, skills, agents | repo plugin code |
| --- | --- | --- | --- |
| default | cannot override | loaded | runs |
| `--pure` | cannot override | loaded | does not run |
| `OPENCODE_DISABLE_PROJECT_CONFIG=1` | cannot override | not loaded | not loaded |

`antilog runner --pure` and `--disable-project-config` set them; both are off by default because a repo may add its own tools inside the machine, as in Claude Code (`docs/V0-contracts.md`). A repo's plugin is code that runs next to the session's tokens: turn `--pure` on where that is not acceptable.

## Testing it

The `Harness` trait starts OpenCode, so tests put a fake HTTP server in its place (`crates/antilog/src/runner/tests.rs`). The ignored `real_opencode_runs_a_session_end_to_end` drives the pinned OpenCode against a scripted model server; its comment says how to run it.

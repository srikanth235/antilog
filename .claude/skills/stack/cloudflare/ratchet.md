# Cloudflare in Ratchet

(target) Ratchet cloud runs on Cloudflare: the cloud shell `ratchet-cloud` is a Worker with one Durable Object per organization, and every session runs on Ratchet's own agent platform accounts. Cloudflare runs no sandboxes: Cloudflare Containers and the managed runner are not part of the design. Nothing of it is on this branch. Self-hosted never touches Cloudflare. The design is in [../rust/ratchet.md](../rust/ratchet.md), [../postgres/ratchet.md](../postgres/ratchet.md) and [../infra/ratchet.md](../infra/ratchet.md).

`general/` is a copy of Cloudflare's own agent skills from [cloudflare/skills](https://github.com/cloudflare/skills) (Apache-2.0, [licence](licenses/cloudflare-skills-LICENSE)): Durable Objects, Workers practices, local development, profiling, the `cf` CLI, Wrangler and Sandboxes (kept for reference; the target uses neither Containers nor Sandboxes), plus the product references for what we use. Start at [general/INDEX.md](general/INDEX.md). The list, and why the rest is left out, is in [sync.py](sync.py).

**This page wins over `general/`.** The upstream skills are written for TypeScript projects that pick their own products. Ratchet's products are decided and its shell is Rust.

## What runs on Cloudflare

| Product | Used for |
| --- | --- |
| Workers | `ratchet-cloud`: the API, the live events and the static web app, in one Worker |
| Durable Objects (SQLite storage) | One per organization: its database, live event streams, job queue and timers (alarms) |
| D1 | The directory: sign-in email or domain to organization and region, and billing. No tenant data |
| Static assets | The built web app |
| Secrets Store | Ratchet's own agent platform credentials, bound to the Worker |
| R2 | Organization exports for moving between cloud and self-hosted |

Nothing else. KV, Queues, Workflows, Hyperdrive, Workers AI and the Agents SDK are not used. Adding a Cloudflare product is a product call: ask the owner.

## Rules

- **Rust, not TypeScript.** `ratchet-cloud` is built with workers-rs (`worker` crate, `http` feature, axum routes from `ratchet-core`). Read the upstream skills for the platform's behaviour, not for code to paste. The local trial passed, so the TypeScript fallback is not needed ([../rust/ratchet.md](../rust/ratchet.md), "Cloud shell trial").
- **The core never imports `worker`.** Cloudflare types stop at the shell. The Durable Object implements the `Store` trait and the timer interface; the core does not know it runs there.
- **Agents use `cf`; Wrangler runs underneath.** `cf` is the one command for the cloud shell and the account: `cf dev`, `cf build`, `cf deploy` (from CI only), `cf d1 migrations apply --local`, and account tasks such as tokens and spending limits. For a Rust Worker, `cf build` and `cf dev` hand the build to Wrangler (a local test in November 2026 showed it), so Wrangler stays a pinned dev dependency, but nobody calls it directly except `npx wrangler tail` for live logs, which `cf` does not have yet. The config is `cloudflare.config.ts`, with the `worker-build` command in the `wrangler.config.ts` that `cf` generates, next to `ratchet-cloud` (target). Never keep a `wrangler.toml` beside them: `cf` ignores it. `cf` is in beta: pin it exactly (`npm install --save-exact`) and read its release notes on every upgrade. `cf d1 migrations apply` targets the real account unless given `--local`.
- **One Durable Object per organization**, addressed by the organization's id from a namespace in its jurisdiction (EU, US or FedRAMP). SQLite storage only, never the key-value storage API. Writes that belong together run in one synchronous transaction. Timers are alarms, never `setTimeout`.
- **Only ledger rows in a Durable Object or D1:** ids, versions, numbers, states and audit rows. Never customer data, a diff, a log or a secret.
- **Platform credentials sit in the Secrets Store.** Ratchet's platform credentials are bound to the Worker through Cloudflare's Secrets Store, never in an organization's database, a response or a log. The server holds no connector admin secret and no model key. Ratchet cloud uses only Ratchet's platform accounts at launch.
- **The Worker talks out to platforms over HTTPS.** It reads each session's events and answers the `measure` tool over them; no platform calls the Worker. It stores ids, numbers, states, commit ids, the conversation's words, spend and audit rows only, never tool output, a diff or a log. The trust rules for sessions are in [../../session/rules.md](../../session/rules.md) and for judge sandboxes in [../../judge/rules.md](../../judge/rules.md).
- **Only CI deploys.** No one runs `cf deploy` from a laptop against production. Local work uses `cf dev` (target).
- **No feedback issues from here.** The upstream skills describe filing feedback on GitHub. Do not, unless the owner asks.

## Check

```bash
cargo build -p ratchet-core --target wasm32-unknown-unknown   # (target) the core stays portable
npx cf dev                                                   # (target) the cloud shell locally
python3 .claude/skills/stack/cloudflare/sync.py --check      # general/ matches the lists
```

## Updating general/

```bash
git clone --depth 1 https://github.com/cloudflare/skills /tmp/cf-skills
python3 .claude/skills/stack/cloudflare/sync.py /tmp/cf-skills
```

Never edit `general/` by hand. To take or drop a skill or a product reference, edit the lists in `sync.py` and sync. Read the diff of an update like any other change: a skill that now contradicts a rule above needs the rule restated here, not the skill edited.

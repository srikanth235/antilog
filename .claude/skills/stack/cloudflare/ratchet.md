# Cloudflare in Ratchet

(target) Ratchet cloud runs on Cloudflare: the cloud shell `ratchet-cloud` is a Worker with one Durable Object per organization, and Ratchet-managed sandboxes run on Cloudflare Containers. Nothing of it is on this branch. Self-hosted never touches Cloudflare. The design is in [../rust/ratchet.md](../rust/ratchet.md), [../postgres/ratchet.md](../postgres/ratchet.md) and [../infra/ratchet.md](../infra/ratchet.md).

`general/` is a copy of Cloudflare's own agent skills from [cloudflare/skills](https://github.com/cloudflare/skills) (Apache-2.0, [licence](licenses/cloudflare-skills-LICENSE)): Durable Objects, Workers practices, local development, profiling, Wrangler and Sandboxes, plus the product references for what we use. Start at [general/INDEX.md](general/INDEX.md). The list, and why the rest is left out, is in [sync.py](sync.py).

**This page wins over `general/`.** The upstream skills are written for TypeScript projects that pick their own products. Ratchet's products are decided and its shell is Rust.

## What runs on Cloudflare

| Product | Used for |
| --- | --- |
| Workers | `ratchet-cloud`: the API, the live events and the static web app, in one Worker |
| Durable Objects (SQLite storage) | One per organization: its database, live event streams, job queue and timers (alarms) |
| D1 | The directory: sign-in email or domain to organization and region, and billing. No tenant data |
| Static assets | The built web app |
| Secrets Store | Ratchet-managed organizations' connector secrets and own model keys, read only by the managed runner |
| Containers (Sandboxes) | Ratchet-managed workspace and evaluator sandboxes |
| R2 | Organization exports for moving between cloud and self-hosted |

Nothing else. KV, Queues, Workflows, Hyperdrive, Workers AI and the Agents SDK are not used. Adding a Cloudflare product is a product call: ask the owner.

## Rules

- **Rust, not TypeScript.** `ratchet-cloud` is built with workers-rs (`worker` crate, `http` feature, axum routes from `ratchet-core`). Read the upstream skills for the platform's behaviour, not for code to paste. A TypeScript shell is only the fallback if the Rust trial fails.
- **The core never imports `worker`.** Cloudflare types stop at the shell. The Durable Object implements the `Store` trait and the timer interface; the core does not know it runs there.
- **Wrangler, not `cf`.** The upstream skills prefer the `cf` CLI for new projects. Ratchet uses Wrangler, because workers-rs builds through it. The config is `wrangler.toml` next to `ratchet-cloud` (target).
- **One Durable Object per organization**, addressed by the organization's id from a namespace in its jurisdiction (EU, US or FedRAMP). SQLite storage only, never the key-value storage API. Writes that belong together run in one synchronous transaction. Timers are alarms, never `setTimeout`.
- **Only ledger rows in a Durable Object or D1:** ids, versions, numbers, states and audit rows. Never customer data, a diff, a log or a secret.
- **Secrets stay with the managed runner.** The Secrets Store binding is given to the managed runner Worker only, never to `ratchet-cloud`. The server must hold no connector secret or model key.
- **Sandboxes keep the trust rules.** A workspace and an evaluator are separate containers with no route between them. The holdout credential goes only to the evaluator. Model keys are added at egress, never passed in. Every sandbox is removed when its run ends. Use the Sandbox API on its `next` line, pinned to an exact version.
- **Only CI deploys.** No one runs `wrangler deploy` from a laptop against production. Local work uses `wrangler dev` (target).
- **No feedback issues from here.** The upstream skills describe filing feedback on GitHub. Do not, unless the owner asks.

## Check

```bash
cargo build -p ratchet-core --target wasm32-unknown-unknown   # (target) the core stays portable
npx wrangler dev                                             # (target) the cloud shell locally
python3 .claude/skills/stack/cloudflare/sync.py --check      # general/ matches the lists
```

## Updating general/

```bash
git clone --depth 1 https://github.com/cloudflare/skills /tmp/cf-skills
python3 .claude/skills/stack/cloudflare/sync.py /tmp/cf-skills
```

Never edit `general/` by hand. To take or drop a skill or a product reference, edit the lists in `sync.py` and sync. Read the diff of an update like any other change: a skill that now contradicts a rule above needs the rule restated here, not the skill edited.

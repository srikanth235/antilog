# antilog-proxy

The proxy outside the session machine: the model gateway (`/anthropic/v1/*`, `/openai/v1/*`) and the
CONNECT egress allowlist. The contract is in `docs/V0-contracts.md`.

## Prices

`data/prices.json` is a trimmed copy of LiteLLM's model price list (MIT). Its licence notice and the
source commit are in `data/prices.header.md`. `src/price.rs` loads it at build time with
`include_str!`. A model that is not in it is booked at the highest price in it, and that is logged.

To refresh it:

```bash
python3 crates/antilog-proxy/scripts/update-prices.py
```

That downloads LiteLLM's file, keeps the Anthropic and OpenAI chat models, and rewrites both files
in `data/` with the new commit. Offline, pass `--source FILE --commit SHA`. Review the diff and run
`cargo test -p antilog-proxy` before committing.

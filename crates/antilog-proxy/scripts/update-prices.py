#!/usr/bin/env python3
"""Regenerates data/prices.json from LiteLLM's price list (MIT, github.com/BerriAI/litellm).

    python3 crates/antilog-proxy/scripts/update-prices.py [--source FILE] [--commit SHA]

Without --source it downloads model_prices_and_context_window.json from LiteLLM's main branch
and records that branch's current commit. Keeps only Anthropic and OpenAI chat models and, for
each, four prices in thousandths of a dollar per million tokens: input, output, cache read and
cache write. A missing cache price is the input price. Review the diff before committing.
"""
import argparse
import json
import subprocess
import urllib.request
from pathlib import Path

RAW = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json"
REPO = "https://github.com/BerriAI/litellm.git"
HERE = Path(__file__).resolve().parent.parent / "data"
HEADER = """# Model prices

`prices.json` is a trimmed copy of `model_prices_and_context_window.json` from LiteLLM
(https://github.com/BerriAI/litellm), taken at commit `{commit}`.
Only Anthropic and OpenAI chat models are kept. Each price is in thousandths of a dollar per
million tokens (3000 is $3.00 per million). Regenerate it with `scripts/update-prices.py`.

LiteLLM is licensed under the MIT License:

MIT License

Copyright (c) 2023 Berri AI

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""


def thousandths(per_token):
    return round(float(per_token) * 1e9)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", help="a local copy of LiteLLM's file instead of downloading")
    parser.add_argument("--commit", help="the LiteLLM commit the file is from")
    args = parser.parse_args()

    if args.source:
        raw = json.loads(Path(args.source).read_text())
    else:
        with urllib.request.urlopen(RAW, timeout=60) as response:
            raw = json.load(response)
    commit = args.commit
    if commit is None:
        out = subprocess.run(["git", "ls-remote", REPO, "refs/heads/main"], capture_output=True, text=True, check=True)
        commit = out.stdout.split()[0]

    table = {}
    for model, info in sorted(raw.items()):
        if not isinstance(info, dict) or info.get("mode") != "chat":
            continue
        provider = info.get("litellm_provider")
        if provider not in ("anthropic", "openai") or ":" in model or "/" in model:
            continue
        if "input_cost_per_token" not in info or "output_cost_per_token" not in info:
            continue
        input_price = thousandths(info["input_cost_per_token"])
        table[model] = {
            "provider": provider,
            "input": input_price,
            "output": thousandths(info["output_cost_per_token"]),
            "cache_read": thousandths(info["cache_read_input_token_cost"]) if "cache_read_input_token_cost" in info else input_price,
            "cache_write": thousandths(info["cache_creation_input_token_cost"]) if "cache_creation_input_token_cost" in info else input_price,
        }
    HERE.mkdir(exist_ok=True)
    (HERE / "prices.json").write_text(json.dumps(table, indent=1, sort_keys=True) + "\n")
    (HERE / "prices.header.md").write_text(HEADER.format(commit=commit))
    print(f"{len(table)} models from {commit}")


if __name__ == "__main__":
    main()

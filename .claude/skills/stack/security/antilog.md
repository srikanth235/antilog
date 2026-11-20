# Security review in Antilog

`general/` is a copy of public security-review skills: four from [Trail of Bits](https://github.com/trailofbits/skills) (CC BY-SA 4.0, [licence](licenses/trailofbits-skills-LICENSE)) and one from [Sentry](https://github.com/getsentry/skills) (Apache-2.0, [licence](licenses/getsentry-skills-LICENSE)). Start at [general/INDEX.md](general/INDEX.md). The list, and why the rest is left out, is in [sync.py](sync.py). The Trail of Bits skills are copied verbatim, never digested, because of their share-alike licence.

**This page wins over `general/`.** The skills are written for audits of any project. Antilog's bar is its own trust rules, in [../../product/promises.md](../../product/promises.md), [../../session/rules.md](../../session/rules.md), [../../judge/rules.md](../../judge/rules.md) and [../../organization/rules.md](../../organization/rules.md):

- The server never connects to data.
- The holdout credential goes only to the judge.
- A model key never enters a session machine; the proxy swaps the session's token for the key. A service key may sit in an environment variable, and the screen says so.
- Every session's machine is released and its disk dropped when its run ends.

A review checks a change against those first, then against the skill's checklist. A finding that breaks one of them is blocking.

## When to use which

| Skill | Use it | Notes |
| --- | --- | --- |
| [rust-review](general/trailofbits/rust-review/SKILL.md) | Before landing a large Rust change, and on a schedule, for the server (`antilog-core`, `antilog-server`) and the runner | Most useful where input is untrusted: the API, the runner's parsing of session events, the `measure` tool intake. `unsafe` is rare here; see [../rust/antilog.md](../rust/antilog.md) |
| [supply-chain-risk-auditor](general/trailofbits/supply-chain-risk-auditor/SKILL.md) | When a dependency is added or raised, with [../libraries.md](../libraries.md) | Alongside `cargo deny check`, not instead of it. A new dependency is still a product call |
| [sharp-edges](general/trailofbits/sharp-edges/SKILL.md) | When designing an API or a config: access tokens and their scopes, roles, the holdout marking, connector settings, defaults | Ask whether the safe use is the easy use and whether a wrong value fails loudly |
| [property-based-testing](general/trailofbits/property-based-testing/SKILL.md) | When writing tests for the acceptance criteria, the noise floor, budget arithmetic, the judge's queue transitions, and the export and import file round trip | Use `proptest` (Rust). The web app has no property-test library in [../libraries.md](../libraries.md), so none for TypeScript. Test layout is in [../../land/tests.md](../../land/tests.md) |
| [gha-security-review](general/getsentry/gha-security-review/SKILL.md) | Whenever `.github/workflows/` changes | Only CI deploys Antilog cloud, so a workflow with deploy rights is the highest-value target. See [../../land/ci.md](../../land/ci.md) |

## Rules

- **Findings go in the change, or are reported.** Fix a finding in the change that caused it. Anything left is named in the report with its risk, as [../../land/report.md](../../land/report.md) says. Do not file issues or pull requests upstream.
- **Tool suggestions are not dependencies.** The skills suggest Semgrep, CodeQL, cargo-audit, Echidna and more. None is a dependency or a required check unless [../libraries.md](../libraries.md) lists it. Use what is installed, or say the step was not run.
- **The skills' output files are scratch.** Reports, SARIF and run plans go outside the repo, or are not committed.
- **Antilog's bar beats the skill's severity.** A holdout leak is critical even if the skill would rate it low.

## Updating general/

```bash
git clone --depth 1 https://github.com/trailofbits/skills /tmp/tob-skills
git clone --depth 1 https://github.com/getsentry/skills /tmp/sentry-skills
python3 .claude/skills/stack/security/sync.py trailofbits=/tmp/tob-skills getsentry=/tmp/sentry-skills
python3 .claude/skills/stack/security/sync.py --check
```

Never edit `general/` by hand. To take or drop a skill, edit the lists in `sync.py` and sync. Read the diff of an update like any other change.

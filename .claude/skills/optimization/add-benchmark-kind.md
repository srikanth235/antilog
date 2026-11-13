# Add a benchmark kind

A kind is a new way to measure, such as a replay of recorded traffic or a scored set of model answers. Today the kinds are in `EvaluatorKind` (`crates/ratchet-core/src/enums.rs`).

1. **A name and a one-line meaning.** The kind has a word in the glossary (`docs/GLOSSARY.md`) and in [model.md](model.md). Add the value to `EvaluatorKind` and to the schema in `crates/ratchet-core/src/config/schema.rs`.
2. **A pure output contract.** The benchmark prints one JSON object with the goal's metric as a dotted path, plus any limit metrics. Parsing is in `crates/ratchet-bench/src/pipeline.rs` (`ParsedBench`). The new kind adds no new parsing path unless the output differs.
3. **Data by reference only.** The benchmark names datasets by id and version (target, see [../context/SKILL.md](../context/SKILL.md)). It never holds a credential or a connection string. Today it reads files in the repo or the sealed holdout prefix.
4. **Machines by reference only.** The benchmark names an environment id and stays on one provider and one machine type (see [../infra/SKILL.md](../infra/SKILL.md)). Today the hardware is `evaluator.hardware` in the config.
5. **A holdout.** The kind says what its holdout is and how it is marked (see [../context/SKILL.md](../context/SKILL.md)). A kind with no way to hold data back cannot confirm a win and is not accepted.
6. **Noise.** The kind defines repeats and how the noise floor is derived from the baseline, using `crates/ratchet-core/src/bench/calibrate.rs`. The accept rule is not changed by a new kind.
7. **A template.** Add a folder under `templates/` with the evaluator, a starter config and a short doc in `docs/packs/`. Register it in `crates/ratchet-core/src/templates/registry.rs`.
8. **Tests.** A golden test of the template in `crates/ratchet-core/tests/templates.rs`, a parse test of the output, and one end-to-end run on a scripted agent.
9. **Receipt.** The win from this kind carries the fields in [owes.md](owes.md). Dataset versions and machines in the receipt are (target).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

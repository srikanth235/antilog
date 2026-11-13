# What optimization owes

Four promises. Each has one line on how it is checked and whether that check exists in the repo today.

| Promise | The rule |
| --- | --- |
| Decide | Only a change that beats the trusted benchmark, confirmed, on the holdout, moves forward. Nobody can mark a win by hand. |
| Receipt | Every win carries its baseline, candidate, confirmation, holdout, noise, dataset versions and machines. |
| Honest numbers | Value is derived from measured wins. Pending is never counted as delivered. A failure is never shown as zero. |
| Keep | A win is re-checked later and never slips back unnoticed. |

## How each is checked

**Decide.** Check: the accept rule is one pure function, `evaluate_ratchet` in `crates/ratchet-core/src/ratchet.rs`, and the engine tests drive it through candidates and the confirm run (`crates/ratchet-engine/tests/engine/core_loop.rs`). Exists for the rule and the confirm step. A test that no route or tool can write a verdict outside the engine is to add. The holdout is optional in `ratchet.yaml` today (`evaluator.holdout`), so "confirmed, on the holdout" is only as strong as the config; making it required is target.

**Receipt.** Check: a test that reads a win back and finds every listed field. Partly exists: the ledger keeps the baseline, the bench and confirm samples and the ratchet move (`crates/ratchet-ledger/src/store.rs`, tables in `crates/ratchet-ledger/migrations/0001_init.sql`). Noise floor is on the baseline row. Dataset versions and machines are not recorded as such yet; the check for them is to add.

**Honest numbers.** Check: tests that value comes only from verified moves and that pending (an open pull request) is kept apart from delivered (a merged one). Partly exists: `crates/ratchetd/src/views/overview.rs` splits delivered and pending by pull request state, and `crates/ratchetd/tests/api/value_story.rs` covers the story. A test that a failed run reads as failed and never as zero is to add.

**Keep.** Check: a scheduled re-run of the benchmark on the merged result, with a visible drift state. To add. Nothing re-checks a merged win today.

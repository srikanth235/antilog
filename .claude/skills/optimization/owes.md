# What optimization owes

Four promises. Each has one line on how it is checked and whether that check exists in the repo today.

| Promise | The rule |
| --- | --- |
| Decide | Only a trial that beats its agent's best on the trusted evaluation, is confirmed on the holdout and holds the guardrails is Kept. Only the evaluator's numbers count. Nobody can mark a win by hand. |
| Receipt | Every win carries a signed receipt: baseline, candidate, confirmation, holdout, noise, the signed numbers, dataset versions, the evaluator image, the machine type and the credential scopes used. |
| Honest numbers | Value is derived from measured wins. Pending is never counted as delivered. A failure is never shown as zero. |
| Keep | A win is re-checked later and never slips back unnoticed. |

## How each is checked

**Decide.** Check: the accept rule is one pure function, `evaluate_ratchet` in `crates/ratchet-core/src/ratchet.rs`, and the engine tests drive it through candidates and the confirm run (`crates/ratchet-engine/tests/engine/core_loop.rs`). Exists for the rule and the confirm step. A test that no route or tool can write a verdict outside the engine is to add. (Each trial measured and kept in a loop per agent is target.) The holdout is optional in `ratchet.yaml` today (`evaluator.holdout`), so "confirmed, on the holdout" is only as strong as the config; making it required is target.

**Receipt.** Check: a test that reads a win back and finds every listed field. Partly exists: the ledger keeps the baseline, the bench and confirm samples and the ratchet move (`crates/ratchet-ledger/src/store.rs`, tables in `crates/ratchet-ledger/migrations/0001_init.sql`). Noise floor is on the baseline row. Dataset versions, the evaluator image, the machine type and the credential scopes are not recorded as such yet, and nothing signs the numbers (target); the check for each is to add. The check for the signature is that a receipt whose numbers were changed after signing fails verification.

**Honest numbers.** Check: tests that value comes only from verified moves and that pending (an open pull request) is kept apart from delivered (a merged one). Partly exists: `crates/ratchetd/src/views/overview.rs` splits delivered and pending by pull request state, and `crates/ratchetd/tests/api/value_story.rs` covers the story. A test that a failed run reads as failed and never as zero is to add.

**Keep.** Check: a scheduled re-run of the benchmark on the merged result, with a visible drift state. To add. Nothing re-checks a merged win today.

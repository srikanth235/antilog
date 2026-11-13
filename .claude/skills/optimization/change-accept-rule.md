# Change the accept rule

The accept rule decides every win, so a change to it changes what the product promises. Put the change to the owner with options and a recommendation before writing code (see [../product/SKILL.md](../product/SKILL.md)).

1. **State the change as a promise.** Say which line of [owes.md](owes.md) it keeps or weakens. A change that weakens Decide is a product decision, not an engineering one.
2. **Keep it a pure function.** The rule lives in `crates/ratchet-core/src/ratchet.rs` (`evaluate_ratchet`, `RatchetInput`, `RatchetOutcome`). It reads inputs and returns a verdict with reasons. It does no I/O and no clock reads.
3. **Keep the required gain tied to noise.** Today the required gain is the larger of the noise floor and `min_gain` times the current best. A change must say what it does to that floor.
4. **Keep confirmation on the holdout.** A candidate that passes the bench but fails the confirm run is `failed_confirm`. A change never lets a win skip confirmation.
5. **Keep reasons.** Every verdict carries reasons a person can read.
6. **Never apply by hand.** No setting, tool or route may set a verdict. The engine writes it after calling the rule (`crates/ratchet-engine/src/tick.rs`).
7. **Version it.** Record which rule version decided each verdict. (target) Old verdicts are never rewritten when the rule changes.
8. **The stop rule** (budget, deadline, plateau) is in the same area of `crates/ratchet-engine/src/tick.rs`. A change to it follows the same steps, and a stop never deletes a verdict.
9. **Tests.** Extend the rule's unit and property tests in `crates/ratchet-core/tests/properties.rs` and the engine tests in `crates/ratchet-engine/tests/engine/core_loop.rs`. Include a case on each side of every boundary you moved.
10. **Words.** Update [model.md](model.md) and `docs/GLOSSARY.md` if a word changed.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

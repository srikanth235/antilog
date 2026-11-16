# Change the acceptance criteria

The acceptance criteria decide every win, so a change to them changes what the product promises. Put it to the owner with options and a recommendation before writing code (see [../product/SKILL.md](../product/SKILL.md)).

1. **State the change as a promise.** Say which line of [owes.md](owes.md) it keeps or weakens. A change that weakens Decide is a product decision, not an engineering one.
2. **Keep it one pure function.** It reads inputs (the trial's value, the value of the trial it started from, the noise, the number of trials tried, the guardrails, the confirmation, the holdout result) and returns an outcome with reasons. It does no I/O and no clock reads (target).
3. **Keep the bar tied to the noise.** A change must say what it does to the bar and to how the bar rises with the number of trials. That rule is open and settled with the first code.
4. **Keep confirmation.** A trial that passes the first measurement and fails the confirmation or the holdout is never Kept. A change never lets a win skip either.
5. **Keep reasons.** Every outcome carries reasons a person can read.
6. **Never apply by hand.** No setting, tool or route may set an outcome. The judge writes it after calling the function.
7. **Version it.** Record which version of the criteria decided each outcome. Old outcomes are never rewritten when the criteria change, and a contract version names the criteria it was sealed with.
8. **Tests.** Unit and property tests, with a case on each side of every boundary you moved, and a test that `deliver` still refuses a trial that is not Kept.
9. **Words.** Update [model.md](model.md) and [the glossary](../../../docs/GLOSSARY.md) if a word changed.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

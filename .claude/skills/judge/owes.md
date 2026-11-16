# What the judge owes

Five promises. Each has one line on how it is checked. Nothing is built, so every check is to add.

| Promise | The rule |
| --- | --- |
| Decide | The quick check never decides: only a trial that beats the trial it started from by more than the bar, holds every guardrail and is confirmed in a fresh judge sandbox and on the holdout is Kept. Only the judge's numbers count. Nobody can mark a win by hand. |
| Tamper-proof | An agent never reads the holdout and never changes the evaluation. A trial that changes a protected path is Rejected. The judge never reads a skill pack, and a plugin can never mark, unmark or grant a dataset. |
| Receipt | Every win carries a signed receipt with every field listed under receipt in [model.md](model.md): Experiment, Data and Compute. |
| Honest numbers | Value is derived from measured wins. A win counts as Delivered once its pull request is merged; Pending (open pull request) is never counted as delivered and is not in Return. A failure is never shown as zero. Return is Delivered divided by everything spent in its scope, misses included, and a regressed amount is At risk, not Delivered. |
| Keep | A delivered win is measured again later. A regression is seen and tells the session. |

## How each is checked

**Decide.** Check: a test that a trial stopped at the quick check is never Kept and that the quick check runs only in the judge. The acceptance criteria are one pure function, with tests on each side of every boundary. A test that no route or tool can write an outcome except the judge's own path. A test that `deliver` refuses a trial that is not Kept. To add.

**Tamper-proof.** Check: the judge's identity cannot read a skill pack and a plugin has no route to mark, unmark or grant a dataset. A test that the credential an agent gets is refused when it reads holdout data, run against each connector. A test that a trial changing a protected path in any repo gets Rejected, and that the judge takes protected files from the seal. To add.

**Receipt.** Check: a test that a win reads back with every listed field (dataset versions, skill pack versions, judge image and digest, credential scopes, agents and sessions included), and that a receipt whose numbers were changed after signing fails verification. Verify again checks that the signature matches, that the signer is the judge and that the image digest is the one recorded. The holdout appears only as passed or failed. To add.

**Honest numbers.** Check: tests that value comes only from Kept wins and a sealed value model (never one an agent set), that Return divides by all spend in scope including experiments with no win, that a regression moves its amount from Delivered to At risk until a fix is merged, that a win is Delivered only once its pull request is merged, that pending is kept apart from delivered and out of Return, and that a failed trial reads as failed and never as zero. To add.

**Keep.** Check: a watch measures the delivered experiment, a result worse than delivered by more than the noise is a regression, and a session is woken and a notification is sent. To add.

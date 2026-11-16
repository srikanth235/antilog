# What the judge owes

Five promises. Each has one line on how it is checked. Nothing is built, so every check is to add.

| Promise | The rule |
| --- | --- |
| Decide | Only a trial that beats the trial it started from by more than the bar, holds every guardrail and is confirmed in a fresh judge sandbox and on the holdout is Kept. Only the judge's numbers count. Nobody can mark a win by hand. |
| Tamper-proof | An agent never reads the holdout and never changes the evaluation. A trial that changes a protected path is Rejected. |
| Receipt | Every win carries a signed receipt: the experiment version and seal, the commits, the values, repeats, noise, the number of trials tried, the holdout result, the second opinion and what was spent. |
| Honest numbers | Value is derived from measured wins. Pending is never counted as delivered. A failure is never shown as zero. |
| Keep | A delivered win is measured again later. A regression is seen and tells the session. |

## How each is checked

**Decide.** Check: the acceptance criteria are one pure function, with tests on each side of every boundary. A test that no route or tool can write an outcome except the judge's own path. A test that `deliver` refuses a trial that is not Kept. To add.

**Tamper-proof.** Check: a test that the credential an agent gets is refused when it reads holdout data, run against each connector. A test that a trial changing a protected path in any repo gets Rejected, and that the judge takes protected files from the seal. To add.

**Receipt.** Check: a test that a win reads back with every listed field, and that a receipt whose numbers were changed after signing fails verification. The holdout appears only as passed or failed. To add.

**Honest numbers.** Check: tests that value comes only from Kept wins, that pending is kept apart from delivered, and that a failed trial reads as failed and never as zero. To add.

**Keep.** Check: a watch measures the delivered experiment, a result worse than delivered by more than the noise is a regression, and a session is woken and a notification is sent. To add.

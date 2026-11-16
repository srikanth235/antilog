# Review

- Read the diff, not the summary.
- Check each claim in the report against a command that was run. A claim with no command is unverified.
- Run the checks yourself when the report is thin or the change is risky. See [land/checks.md](../land/checks.md).
- Check the promises of the area touched. Read its `owes.md` and `rules.md` through [product](../product/SKILL.md) and the [promises index](../product/promises.md).
- Check the non-goals and the decision process in [product](../product/SKILL.md).
- Check that nothing target is described as built, and that docs the change made false are fixed.
- Check that no check, test or snapshot was loosened, skipped or deleted.
- Check wording against [design/wording.md](../design/wording.md).
- Send it back with the exact failing check or file. Do not fix a session's work silently.

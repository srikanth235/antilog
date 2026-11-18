# Briefs

A brief states:

- **The goal**, in one or two sentences, and which area owns it (the session, the judge or the organization floor).
- **The rulings already made.** Link the [product](../product/SKILL.md) skill and the area's `owes.md` and `rules.md`. Do not restate them, and do not let a session reopen them.
- **The files** to read and the files to change.
- **The checks to run.** Link [land/checks.md](../land/checks.md) and name the commands for this kind of change. Git rules: [land/git.md](../land/git.md).
- **How to ask the owner.** Owner questions go to the orchestrator with `send_message` (its session id is in the brief), not left at the end of a turn. Take the safest choice, record it as a decision to review and keep working; stop only for a new dependency, spend or a security question.
- **What to report.** The format in [land/report.md](../land/report.md): commands run, what passed, what was not run, what only CI or a real cloud can verify.

Rules:

- Never ask a session to do something that was denied to you or to another session.
- One area skill per session where possible. A change across areas is split, with the shared interface written first.
- Mockups first for anything a user sees.
- Say what is target and what is built, so a session does not build or document the wrong one.
- Real-cloud runs, spend and new dependencies are not delegated without the owner's go-ahead. See [land/real-cloud.md](../land/real-cloud.md).

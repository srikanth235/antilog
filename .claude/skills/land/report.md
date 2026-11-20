# Reporting

Say what the evidence is, not what you hope.

- List each command actually run and whether it passed. Paste the failing line, not a summary.
- List what was running or unfinished when you stopped.
- Name every check not run, with the exact command and what it needs (macOS, CI, a real cloud account).
- Say what only CI or a real account can verify: the macOS jobs, image builds, anything that touches a cloud, a cluster or a real agent. Never call an unrun layer verified.
- A claim you did not run is listed as unverified. Where a layer has no command, say "no check exists yet", not "passed".
- Say what is target and what is built.
- Say what a real run cost, if one ran, and what clean-up left.
- Plain words: what landed, what it changes for a user, what is owed.

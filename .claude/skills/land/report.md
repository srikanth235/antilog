# Reporting

Say what the evidence is, not what you hope.

- List each command actually run and whether it passed. Paste the failing line, not a summary.
- List what was running or unfinished when you stopped.
- Name every check not run, with the exact command and what it needs (macOS, CI, a real cloud account).
- Say what only CI or a real cloud account can verify: the macOS Rust job, Packer validation, anything that touches AWS or a real agent. Never call an unrun layer verified.
- A claim you did not run is listed as unverified.
- Say what is target and what is built.
- Plain words: what landed, what it changes for a user, what is owed.

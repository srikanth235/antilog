# Add a subject kind

A subject is a new kind of thing that may change: anything with a version and a diff. Today it is only a git repository.

1. **Version and diff.** The kind says what its version is and how two versions are compared. If it has neither, it is not a subject. Today a version is a commit sha (`Candidate.sha` in `crates/ratchet-core/src/rows.rs`).
2. **A way to apply a candidate.** One function takes a baseline version and a proposed change and returns a candidate version, without touching the benchmark. (target) Today this is a branch in a git worktree (`crates/ratchet-agents/src/workdir.rs`, `crates/ratchet-agents/src/git.rs`).
3. **Protected parts.** The kind lists what a proposer may never change: the benchmark, the scoring, the holdout. Today these are globs checked by `crates/ratchet-core/src/protected_set.rs` and restored by `crates/ratchet-bench/src/overlay.rs`. A new kind needs an equivalent that blocks a candidate before it is measured.
4. **Where it lands.** The kind says how a verified win reaches the world. Today it is a pull request (`crates/ratchet-github/src/github_host.rs`). Anything that writes outside Ratchet is a write-back action requested here and performed through context (see [../context/SKILL.md](../context/SKILL.md)), and is audited (see [../organization/SKILL.md](../organization/SKILL.md)).
5. **Where proposers work.** The kind names the kind of workspace it needs, by environment id (see [../infra/SKILL.md](../infra/SKILL.md)).
6. **The words.** Add the kind to [model.md](model.md) and to `docs/GLOSSARY.md` if a person will read it.
7. **Tests.** Apply a candidate, block a protected change, measure, reject a regression, and land a win, on a mock host. The git host has a mock (`crates/ratchet-github/src/mock.rs`).
8. **Receipt.** The win names the subject version before and after.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

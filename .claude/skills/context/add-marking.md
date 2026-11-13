# Add a marking

A marking limits who may be granted a dataset. Holdout is the only one, and it is only a name in the config today (`evaluator.holdout` in `crates/ratchet-core/src/config/schema.rs`).

1. **A name and a rule.** State in one sentence which audience a marking allows and which it denies. Holdout allows the evaluator and denies agents. (target)
2. **One enforcement point.** Add the rule to the single place that checks grants. No second check in a source, a benchmark or a route. (target)
3. **Denied by default.** A dataset with the marking and no grant is unreadable by everyone. A grant that conflicts with the marking is refused when it is created, not when it is read. (target)
4. **Marking changes are audited.** Adding or removing a marking is a named action with an actor (see [../organization/SKILL.md](../organization/SKILL.md)). Removing holdout from a dataset a run already read is refused. (target)
5. **Matching machine boundary.** The marking has a matching deny in infra, so a bug in the grant check does not expose the data. The model for this is the sealed bucket deny in `infra/aws/policies/sealed_bucket_policy.json.tftpl` and `infra/aws/policies/agent_boundary.json.tftpl`.
6. **Words.** Add the marking to [model.md](model.md).
7. **Tests.** An agent audience is denied, an evaluator audience is allowed, a missing grant is denied, and the machine-level deny holds when the grant check is bypassed. The infra side is tested in `infra/tests/src/agent-boundary.test.ts`.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

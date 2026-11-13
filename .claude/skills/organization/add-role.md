# Add a role

Roles on a project today are only target: owner, editor, viewer. Today every caller holds the owner bearer token (`crates/ratchetd/src/auth.rs`).

1. **Name and one sentence.** State what the role may do in the words of [model.md](model.md). A role that needs more than one sentence is two roles.
2. **A matrix row.** Add the role to the one permission table: for each action, allow or deny. The table is the only place permission is decided. (target)
3. **Ordered by power.** A role is a subset or superset of another where possible, so "can editor do this" has one answer. (target)
4. **Least by default.** A new member gets the lowest role. Only an owner raises it. Raising and lowering are audited actions (see [add-audited-action.md](add-audited-action.md)).
5. **Group mapping.** If the identity provider sends groups, add the mapping from a group claim to the role. Mapping is read at sign-in. (target)
6. **Never loses the last owner.** A project must keep at least one owner. Removing or demoting the last one is refused. (target)
7. **No role in a handler.** Handlers ask the action layer, never test a role name. (target)
8. **Route test.** Extend the route-walk test pattern in `crates/ratchetd/tests/api/auth.rs` so every route is called as each role and the answer matches the matrix.
9. **Words.** Add the role to [model.md](model.md).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

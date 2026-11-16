# Add a role

Roles are owner, editor and viewer, and are organization-wide. Per-repo visibility is an open owner call ([../product/rulings.md](../product/rulings.md)). All of it is target. The earlier code had one owner bearer token; see [today.md](today.md).

1. **Name and one sentence.** State what the role may do in the words of [model.md](model.md). A role that needs more than one sentence is two roles.
2. **A matrix row.** Add the role to the one permission table: for each action and each approval card kind (see [approvals.md](approvals.md)), allow or deny. The table is the only place permission is decided.
3. **Ordered by power.** A role is a subset or superset of another where possible, so "can editor do this" has one answer.
4. **Least by default.** A new member gets the lowest role. Only an owner raises it. Raising and lowering are audited actions (see [add-audited-action.md](add-audited-action.md)).
5. **Group mapping.** If the identity provider sends groups, add the mapping from a group claim to the role. Mapping is read at sign-in.
6. **Never loses the last owner.** An organization must keep at least one owner. Removing or demoting the last one is refused.
7. **No role in a handler.** Handlers ask the action layer, never test a role name. Access tokens and service accounts carry a role too, and are checked the same way.
8. **Route test.** A route-walk test calls every route in the OpenAPI document as each role and compares the answer with the matrix. A new route with no matrix entry fails.
9. **Clients.** The web app and the Mac app (target) read what a role may do from the server and never decide it themselves; a viewer sees controls disabled, not hidden (see [../design/behaviour.md](../design/behaviour.md)).
10. **Words.** Add the role to [model.md](model.md) and the glossary.

In the earlier code the route-walk pattern was `every_route_needs_the_token` in `crates/ratchetd/tests/api/auth.rs`, walking `route_table()` in `crates/ratchetd/src/openapi.rs`; the role version would extend it.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

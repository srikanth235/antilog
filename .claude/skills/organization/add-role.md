# Add a role

Roles are owner, editor and viewer. A member has an organization role, which is their default in every project, and a project can set a different role for a member in that project. Per-repo visibility is not offered ([../product/rulings.md](../product/rulings.md)). Organization owners manage organization settings. Project owners manage the project's members and budget. All of it is target.

1. **Name and one sentence.** State what the role may do in the words of [model.md](model.md). A role that needs more than one sentence is two roles.
2. **A matrix row.** Add the role to the one permission table: for each action and each approval card kind (see [approvals.md](approvals.md)), allow or deny, at organization scope and at project scope. The table is the only place permission is decided.
3. **Ordered by power.** A role is a subset or superset of another where possible, so "can editor do this" has one answer.
4. **Least by default.** A new member gets the lowest role, and a new project member gets their organization role. Only an owner raises it: an organization owner for the organization role, a project owner for a project role. Raising and lowering are audited actions (see [add-audited-action.md](add-audited-action.md)).
5. **Group mapping.** If the identity provider sends groups, add the mapping from a group claim to the role. Mapping is read at sign-in.
6. **Never loses the last owner.** An organization must keep at least one owner, and so must every project. Removing or demoting the last one is refused.
7. **No role in a handler.** Handlers ask the action layer, never test a role name. The layer resolves the role in one place: the project's role for the member if set, else the organization role. Access tokens and service accounts carry a role too, and may be given a project role, and are checked the same way.
8. **Route test.** A route-walk test calls every route in the OpenAPI document as each role, in the organization and in a project that overrides it, and compares the answer with the matrix. A new route with no matrix entry fails.
9. **Clients.** The web app and the Mac app (target) read what a role may do from the server and never decide it themselves; a viewer sees controls disabled, not hidden (see [../design/behaviour.md](../design/behaviour.md)).
10. **Words.** Add the role to [model.md](model.md) and the glossary. Skill packs, plugins and agents have no role of their own: a plugin never grants a role or access to a dataset.


## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

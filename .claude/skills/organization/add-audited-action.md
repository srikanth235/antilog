# Add an audited action

Every write is a named action, and every action goes through one action layer, the functions of `crates/antilog-core/src/organization.rs`, `approval.rs` and `budget.rs`. The audited actions are the tool calls that change something and the approvals ([approvals.md](approvals.md)). The layer and its audit entries are built for the organization floor's writes; live events and the other areas' writes are target.

1. **A name.** Give the action one verb-object name, such as `session.stop` or `card.approve`. The name is what the audit log shows and what the permission matrix lists.
2. **Declare it once.** Add the action to `Action` in `organization.rs` (its name and least role, the one matrix) and register it in the action layer with its input, the roles that may run it, the audit fields it records, and the event it emits.
3. **Permission first.** The layer checks the actor's role in the organization before anything runs. A denied call writes an audit entry with the outcome and changes nothing.
4. **Then the write.** The write and its audit entry happen in one Postgres transaction, or not at all. A write without an entry is a defect. See [../change/write-action.md](../change/write-action.md).
5. **Then the event.** The layer publishes the live event after the commit ([../change/live-event.md](../change/live-event.md)). Events and audit entries carry no secret.
6. **The route.** Add the handler and the operation in the OpenAPI document by [../change/api-endpoint.md](../change/api-endpoint.md). The handler calls the layer and tests no role. Every route needs a signed-in member or an access token.
7. **The organization and the project.** Every row the action writes carries its organization, and an action on an experiment or a project carries the project id the permission check used.
8. **Work on an agent platform.** An action that starts or stops a session or a judge sandbox queues a job in the same transaction; see [../change/SKILL.md](../change/SKILL.md).
9. **Outside effects and cards.** An action that writes to an outside system, such as opening a pull request, is the `deliver` tool behind an Open pull request card. A tool that needs a person raises one of the eight cards and is audited twice, once for the call and once for the approval ([approvals.md](approvals.md)). Policy never skips the audit row.
10. **Owner-only settings.** Changing an organization's agent platform accounts, policy or the monthly cap is an organization-owner-only, audited action. Project actions are audited with the project id: creating or deleting a project, adding or removing a project member, changing a member's role in a project, and changing a project's cap or default budget (project owners). Setting or changing a value model (project owners, `value_model.set`, built; a changed one is approved on the Seal experiment card) and reading the organization's value and Report (`report.read`, organization owners and editors, built; any member reads a project's), handing a session off, a failover, choosing the monthly report's recipients and exporting a report are audited actions too, the first with the project id. Adding, changing and removing a Customize item (an evaluation, a skill pack, a connector, an agent, a plugin) are named actions too. (target)
11. **Tests.** Allowed role writes and audits once; denied role writes nothing and audits the denial; the event is emitted after the commit; the audit row and the change roll back together; no secret in the entry or the event.


## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

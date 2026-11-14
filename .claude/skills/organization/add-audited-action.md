# Add an audited action

Every write is a named action, and every action goes through one action layer in `ratchet-server`. All of it is target. Today: there is no action layer; see [today.md](today.md).

1. **A name.** Give the action one verb-object name, such as `experiment.stop`. The name is what the audit log shows and what the permission matrix lists.
2. **Declare it once.** Register the action in the action layer with its input, the roles that may run it, the audit fields it records, and the event it emits.
3. **Permission first.** The layer checks the actor's role on the project before anything runs. A denied call writes an audit entry with the outcome and changes nothing.
4. **Then the write.** The write and its audit entry happen in one Postgres transaction, or not at all. A write without an entry is a defect. See [../change/write-action.md](../change/write-action.md).
5. **Then the event.** The layer publishes the live event after the commit ([../change/live-event.md](../change/live-event.md)). Events and audit entries carry no secret.
6. **The route.** Add the handler and the operation in the OpenAPI document by [../change/api-endpoint.md](../change/api-endpoint.md). The handler calls the layer and tests no role. Every route needs a signed-in member or an access token.
7. **The organization.** Every row the action writes carries its organization.
8. **Work in the customer's infrastructure.** An action that needs a sandbox queues a job in the same transaction; see [../change/runner-job.md](../change/runner-job.md).
9. **Outside effects.** An action that writes to an outside system, such as opening a pull request, is a write-back action requested by optimization and performed through context (see [../context/SKILL.md](../context/SKILL.md)). It is audited like any other.
10. **Tests.** Allowed role writes and audits once; denied role writes nothing and audits the denial; the event is emitted after the commit; the audit row and the change roll back together; no secret in the entry or the event.

Today: a handler in `crates/ratchetd/src/api/` called the service directly with no actor and no audit entry. Events needed a new variant in `ApiEvent` (`crates/ratchetd/src/api/events.rs`).

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

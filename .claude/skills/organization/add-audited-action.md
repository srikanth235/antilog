# Add an audited action

Every write is a named action. Today there is no action layer: a handler in `crates/ratchetd/src/api/` calls the service directly, with no actor and no audit entry.

1. **A name.** Give the action one verb-object name, such as `optimization.stop`. The name is what the audit log shows and what the permission matrix lists. (target)
2. **Declare it once.** Register the action in the action layer with its input, the roles that may run it, the audit fields it records, and the event it emits. (target)
3. **Permission first.** The layer checks the actor's role on the project before anything runs. A denied call writes an audit entry with the outcome and changes nothing. (target)
4. **Then the write.** The write and its audit entry happen together or not at all. A write without an entry is a defect. (target)
5. **Then the event.** The layer emits the live event after the write. Events and audit entries carry no secret. Today events are in `crates/ratchetd/src/api/events.rs` and need a new variant in `ApiEvent`.
6. **The route.** Add the route in `crates/ratchetd/src/api/` and the operation in the OpenAPI document. After a type change, regenerate it with `cargo run -p ratchetd -- openapi` and keep `crates/ratchetd/tests/api/openapi.rs` green. Every route still needs the token (`crates/ratchetd/tests/api/auth.rs`).
7. **The organization.** Every row the action writes carries its organization. (target)
8. **Outside effects.** An action that writes to an outside system, such as opening a pull request, is a write-back action requested by optimization and performed through context (see [../context/SKILL.md](../context/SKILL.md)). It is audited like any other.
9. **Tests.** Allowed role writes and audits once; denied role writes nothing and audits the denial; the event is emitted; no secret in the entry or the event.

## Proving it

Follow [../land/SKILL.md](../land/SKILL.md).

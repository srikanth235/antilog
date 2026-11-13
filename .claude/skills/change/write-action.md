# A write

## The rule (target)

(target) Every write goes through one action layer, and nowhere else. The layer does three things, in this order: it checks the caller's role, it writes one audit entry, and it publishes one live event. A write that skips any of the three is a bug. A new write is an entry in the action layer, not a handler with its own checks. The area's recipe `add-audited-action.md` in [organization](../organization/add-audited-action.md) says how to register one.

Nothing in the repository implements this layer yet. There is no role check, no audit entry and no organization.

## What a write does today

- The route handler (`crates/ratchetd/src/routes/`) decodes the request and calls one `AppService` method (`crates/ratchetd/src/service.rs`). Examples: `create_optimization`, `stop_optimization`.
- The method changes the ledger and the engine, in `crates/ratchetd/src/real/service.rs` for real runs and `crates/ratchetd/src/sim/` for the simulation.
- The service broadcasts an `ApiEvent` (`AppService::subscribe`); `EventHub::publish` in `crates/ratchetd/src/hub.rs` numbers it and sends it to every client. See [live-event.md](live-event.md).
- Authorization today is one per-boot bearer token on a 0600 unix socket (`crates/ratchetd/src/auth.rs`). There are no users or roles.
- The CLI preview rule applies: a command that spends or changes things needs `--yes` and previews without it (`crates/ratchet-cli/src/cli.rs`). See [api-endpoint.md](api-endpoint.md).

When you add a write today, follow the steps in [api-endpoint.md](api-endpoint.md) and keep it in one `AppService` method, so moving it behind the action layer later is one change.

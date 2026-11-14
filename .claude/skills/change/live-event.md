# A live event

Events tell a client that something changed so it can refetch. An event is not the data and never carries a secret or customer data. (target: nothing here is built yet.)

## How events work (target)

- One stream from `ratchet-server`, server-sent events, scoped to the signed-in member's organization and projects. A client only receives events for what its role may read.
- Each event has a kebab-case type, a sequence id and the JSON of the whole event.
- The first message on a connection is `hello`. A client that reconnects sends `Last-Event-ID` and receives what it missed. If the server no longer has it, the first message is `resync` and the client refetches what it shows.
- The action layer publishes the event after the write commits (see [write-action.md](write-action.md)). The runner's reports of state and spend come in as writes, so they publish events the same way.
- Events are rows. The action layer writes the event to an append-only Postgres table in the same transaction as the change, and `NOTIFY` wakes every server process. A stream replays from that table by `Last-Event-ID`, so any process can serve any client. Rows older than the replay window are pruned; a client further behind gets `resync`. No Redis.

## Adding an event (target)

1. Add a variant to the event type in `crates/ratchet-server/src/api/events.rs` and publish it from the action layer, never from a handler.
2. Regenerate the document and the TypeScript client by [api-endpoint.md](api-endpoint.md). The event union is part of the document and is snapshotted.
3. Tell the web app and the Mac app which screens refetch on it; the boards say which state changes (see [screen.md](screen.md)).
4. Tests: the stream replays after a gap, sends `resync` when it cannot, publishes after commit only, and a viewer receives no event for a project it cannot read.

The earlier daemon's event kinds and hub are in [today.md](today.md).

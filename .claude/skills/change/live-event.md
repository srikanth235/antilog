# A live event

## How events work today

- One stream: `GET /v1/events`, server-sent events.
- The event type is `ApiEvent` in `crates/ratchetd/src/api/events.rs`. The SSE `event:` field is its `type` (kebab-case), `id:` is a per-boot sequence number, and `data:` is the JSON of the whole event.
- Kinds today: `hello` and `resync` (the stream's own control events), then `optimization`, `agent-event`, `candidate`, `ratchet`, `spend`, `notice`, `run-active`, `sandbox`, `environment-draft`, `connector` and `github`. The file is the source; read it before adding one.
- The first message on every connection is `hello`. A client that reconnects sends `Last-Event-ID` and receives what it missed. If the daemon no longer has it, or restarted, the first message is `resync` and the client refetches everything it shows.
- `EventHub` (`crates/ratchetd/src/hub.rs`) numbers events, keeps a window and replays; its tests cover replay, gaps and the bridge from a service.
- A client refetches what an event names. Events announce a change; they are not the data.
- A chat has its own stream (`ChatStreamEvent`, `/v1/chats`).

## Adding an event

1. Add a variant to `ApiEvent` and publish it from the service. Add it to the documented union in `api/openapi.yaml` through the steps in [api-endpoint.md](api-endpoint.md); the union is snapshotted (`crates/ratchetd/tests/api/snapshots/api__openapi__api_event.snap`).
2. Test the stream in `crates/ratchetd/tests/api/sse.rs`.
3. An event never carries a secret.

(target) An event is published by the action layer as part of every write; see [write-action.md](write-action.md).

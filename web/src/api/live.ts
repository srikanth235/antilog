import type { QueryClient, QueryKey } from "@tanstack/react-query";
import { useMemo, useSyncExternalStore } from "react";
import { API_BASE, type Schemas } from "./client";
import { setLive } from "./liveStatus";
import { keys } from "./queries";

export type LiveEvent = Schemas["LiveEvent"];

const NAMES = ["hello", "resync", "message", "session", "cards"] as const;
/** After the stream is refused or cut for good, try again this often; the 4 s poll runs meanwhile. */
const RETRY_MS = 30_000;

/** The queries an event makes stale. Events are hints: a client refetches what it shows. */
export function staleBy(event: LiveEvent): QueryKey[] {
  switch (event.type) {
    case "hello":
    case "resync":
      return [["sessions"], ["approvals"], ["projects"]];
    case "message":
      return [keys.transcript(event.sessionId)];
    case "session":
      return [keys.session(event.sessionId), keys.sessions(), keys.pending(), ["projects"]];
    case "cards":
      return [keys.pending(), keys.sessions()];
  }
}

export function parse(data: string): LiveEvent | null {
  try {
    const value: unknown = JSON.parse(data);
    if (typeof value === "object" && value !== null && "type" in value) return value as LiveEvent;
  } catch {
    // A half message is ignored; the next hello refetches everything.
  }
  return null;
}

/**
 * Opens the event stream and refetches what it says changed. The stream is closed in a hidden
 * tab and opened again, with a refetch, when the tab shows. When it fails the pages poll as
 * before (`useStreamLive` is false), and it is tried again after a while. The browser
 * reconnects by itself after the server ends a stream, sending `Last-Event-ID`.
 */
export function startLive(client: QueryClient): () => void {
  let source: EventSource | null = null;
  let retry: ReturnType<typeof setTimeout> | undefined;

  const close = () => {
    source?.close();
    source = null;
    clearTimeout(retry);
    setLive(false);
  };

  const open = () => {
    if (source !== null || document.hidden || typeof EventSource === "undefined") return;
    const next = new EventSource(`${API_BASE}/events/stream`, { withCredentials: true });
    source = next;
    for (const name of NAMES) {
      next.addEventListener(name, (message) => {
        const event = parse((message as MessageEvent<string>).data);
        if (event === null) return;
        if (event.type === "hello" || event.type === "resync") setLive(true);
        for (const queryKey of staleBy(event)) void client.invalidateQueries({ queryKey });
      });
    }
    next.onerror = () => {
      setLive(false);
      // CLOSED: refused (signed out, a proxy that cannot stream). CONNECTING: the browser retries.
      if (next.readyState === EventSource.CLOSED) {
        close();
        retry = setTimeout(open, RETRY_MS);
      }
    };
  };

  const onVisibility = () => (document.hidden ? close() : open());
  document.addEventListener("visibilitychange", onVisibility);
  open();
  return () => {
    document.removeEventListener("visibilitychange", onVisibility);
    close();
  };
}

/** Mount once, in the signed-in shell: the stream lives as long as the subscription does. */
export function useLiveUpdates(client: QueryClient): void {
  const subscribe = useMemo(() => () => startLive(client), [client]);
  useSyncExternalStore(
    subscribe,
    () => null,
    () => null,
  );
}

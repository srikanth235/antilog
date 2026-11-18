import { useSyncExternalStore } from "react";

/** Whether the server's event stream is delivering. While it is, the pages stop polling. */
let live = false;
const listeners = new Set<() => void>();

export function setLive(next: boolean): void {
  if (live === next) return;
  live = next;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useStreamLive(): boolean {
  return useSyncExternalStore(
    subscribe,
    () => live,
    () => false,
  );
}

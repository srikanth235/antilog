// Per-viewer conveniences only: appearance, collapsed groups, pins, the last organization. No tokens, no customer data.
import { useSyncExternalStore } from "react";

const listeners = new Set<() => void>();

export function readJson(key: string): unknown {
  try {
    const text = localStorage.getItem(key);
    return text === null ? null : (JSON.parse(text) as unknown);
  } catch {
    return null;
  }
}

export function writeJson(key: string, value: unknown): void {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // storage is blocked: the preference lasts until reload
  }
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** A stored value that is narrowed by `parse` and re-read when it changes. */
export function useStored<T>(key: string, parse: (raw: unknown) => T): T {
  const raw = useSyncExternalStore(subscribe, () => localStorage_get(key));
  return parse(raw === null ? null : safeParse(raw));
}

function localStorage_get(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function safeParse(text: string): unknown {
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return null;
  }
}

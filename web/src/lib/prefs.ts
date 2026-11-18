import { readJson, useStored, writeJson } from "./storage";

function strings(raw: unknown): string[] {
  return Array.isArray(raw) ? raw.filter((item): item is string => typeof item === "string") : [];
}

const PINS = "antilog.pins.v1";
const FOLDED = "antilog.folded.v1";

export function usePins(): string[] {
  return useStored(PINS, strings);
}
export function togglePin(id: string): void {
  const pins = strings(readJson(PINS));
  writeJson(PINS, pins.includes(id) ? pins.filter((pin) => pin !== id) : [...pins, id]);
}
export function useFolded(): string[] {
  return useStored(FOLDED, strings);
}
export function setFolded(id: string, folded: boolean): void {
  const list = strings(readJson(FOLDED)).filter((item) => item !== id);
  writeJson(FOLDED, folded ? [...list, id] : list);
}

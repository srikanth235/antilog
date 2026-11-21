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

const LAST_PROJECT = "antilog.lastProject.v1";
const LAST_MODEL = "antilog.lastModel.v1";

function text(raw: unknown): string | null {
  return typeof raw === "string" && raw !== "" ? raw : null;
}

/** The project the last session was started in, for the home page's picker. */
export function useLastProject(): string | null {
  return useStored(LAST_PROJECT, text);
}
export function setLastProject(id: string): void {
  writeJson(LAST_PROJECT, id);
}
/** The last `provider/model` a person typed. */
export function useLastModel(): string | null {
  return useStored(LAST_MODEL, text);
}
export function setLastModel(model: string): void {
  writeJson(LAST_MODEL, model);
}

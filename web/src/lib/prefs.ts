import { pushRecent } from "./models";
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
/** The last `provider/model` a person picked. */
export function useLastModel(): string | null {
  return useStored(LAST_MODEL, text);
}
export function setLastModel(model: string): void {
  writeJson(LAST_MODEL, model);
}

const LAST_EFFORT = "antilog.lastEffort.v1";
const RECENT_MODELS = "antilog.recentModels.v1";

/** The effort last picked with the last model; `null` when that model had none. */
export function useLastEffort(): string | null {
  return useStored(LAST_EFFORT, text);
}
/** The models last used, newest first, at most four: the picker's favourites. */
export function useRecentModels(): string[] {
  return useStored(RECENT_MODELS, strings);
}
/** Remembers a pick: the model, its effort and its place among the favourites. */
export function rememberModel(model: string, effort: string | null): void {
  writeJson(LAST_MODEL, model);
  writeJson(LAST_EFFORT, effort ?? "");
  writeJson(RECENT_MODELS, pushRecent(strings(readJson(RECENT_MODELS)), model));
}

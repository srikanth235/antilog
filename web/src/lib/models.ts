import type { Schemas } from "../api/client";

export type ModelInfo = Schemas["ModelInfo"];

/** The most models the picker keeps in its first group. */
export const FAVOURITES = 4;

/** The model's levels, lowest first: what the Effort submenu offers. Empty: the row is hidden. */
export function levelsOf(model: ModelInfo | undefined): string[] {
  return model?.reasoningLevels ?? [];
}

const LEVEL_NAMES: Record<string, string> = { xhigh: "Extra high" };

/** "xhigh" reads "Extra high"; any other level has its first letter in capitals. */
export function levelLabel(level: string): string {
  return LEVEL_NAMES[level] ?? level.charAt(0).toUpperCase() + level.slice(1);
}

/** The effort a model starts with: Medium, else High, else its first level; none if it has none. */
export function defaultEffort(model: ModelInfo | undefined): string | null {
  const levels = levelsOf(model);
  if (levels.includes("medium")) return "medium";
  if (levels.includes("high")) return "high";
  return levels[0] ?? null;
}

/** The effort to keep when the model changes: the current one if the new model offers it, else its default. */
export function effortAfterSwitch(model: ModelInfo, current: string | null): string | null {
  return current !== null && levelsOf(model).includes(current) ? current : defaultEffort(model);
}

/**
 * The first group of the menu: the models last used (newest first) that the organization's keys
 * still reach, with the current one among them, at most four.
 */
export function favourites(
  recent: readonly string[],
  models: readonly ModelInfo[],
  current: string | null,
): ModelInfo[] {
  const byId = new Map(models.map((model) => [model.id, model]));
  // The order is the recent list's, so a number key keeps its model; the current one joins at the front if it is not there.
  const ids = (
    current === null || recent.includes(current) ? [...recent] : [current, ...recent]
  ).filter((id, at, all) => all.indexOf(id) === at);
  const known = ids.flatMap((id) => {
    const model = byId.get(id);
    return model === undefined ? [] : [model];
  });
  return known.slice(0, FAVOURITES);
}

/** The model the number key picks while the menu is open: "1" to "4", in the order shown. */
export function byShortcut(key: string, shown: readonly ModelInfo[]): ModelInfo | undefined {
  return /^[1-9]$/.test(key) ? shown[Number(key) - 1] : undefined;
}

/** Move a model to the front of the recent list, keeping four. */
export function pushRecent(recent: readonly string[], id: string): string[] {
  return [id, ...recent.filter((item) => item !== id)].slice(0, FAVOURITES);
}

export type Group = { provider: ModelInfo["provider"]; models: ModelInfo[] };

/** Models whose name, id or provider contains every word typed, grouped by provider in the order given. */
export function search(models: readonly ModelInfo[], query: string): Group[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  const groups: Group[] = [];
  for (const model of models) {
    const hay = `${model.name} ${model.id} ${model.provider}`.toLowerCase();
    if (!words.every((word) => hay.includes(word))) continue;
    const last = groups[groups.length - 1];
    if (last?.provider === model.provider) last.models.push(model);
    else groups.push({ provider: model.provider, models: [model] });
  }
  return groups;
}

function trim(value: number): string {
  return value.toFixed(1).replace(/\.0$/, "");
}

/** "200K", "1M", "1.1M". */
export function contextLabel(tokens: number | null | undefined): string {
  if (tokens === null || tokens === undefined || tokens <= 0) return "";
  if (tokens >= 1_000_000) return `${trim(tokens / 1_000_000)}M`;
  return `${Math.round(tokens / 1000)}K`;
}

function dollars(value: number): string {
  return Number.isInteger(value) ? `$${value}` : `$${value.toFixed(2)}`;
}

/** "$3 / $15" per million input and output tokens, "Free" at no price, empty when unknown. */
export function priceLabel(model: ModelInfo): string {
  const { inputPrice, outputPrice } = model;
  if (inputPrice === null || inputPrice === undefined) return "";
  if (inputPrice === 0 && (outputPrice ?? 0) === 0) return "Free";
  return `${dollars(inputPrice)} / ${dollars(outputPrice ?? 0)}`;
}

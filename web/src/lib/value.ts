// Pure helpers for value models, Return and the Report. Dollar math lives here, never in a component.
import type { Schemas } from "../api/client";
import { dollars, parseDollars, plural } from "./format";

/** Cents a year that one unit is worth, from "one unit is N" and "worth D dollars a year". */
export function centsPerUnit(unitSize: number, dollarsPerYear: number): number | null {
  if (!(unitSize > 0) || !(dollarsPerYear > 0)) return null;
  const cents = (dollarsPerYear * 100) / unitSize;
  return Number.isFinite(cents) && cents > 0 ? cents : null;
}

/** Whole cents of a yearly value, for display. */
function whole(cents: number): number {
  return Math.round(cents);
}

function unitWord(size: number, unit: string): string {
  return size === 1 ? unit : `${size.toLocaleString("en-US")} ${unit}`;
}

/** "Each 10 KB of production bundle size is worth $420 a year. A win of 40 KB would be worth $1,680 a year." */
export function valueSentence(
  metric: string,
  unit: string,
  unitSize: number,
  dollarsPerYear: number,
): string | null {
  const per = centsPerUnit(unitSize, dollarsPerYear);
  if (per === null) return null;
  const of = metric.trim() === "" ? "" : ` of ${metric.trim()}`;
  const win = unitSize * 4;
  return `Each ${unitWord(unitSize, unit)}${of} is worth ${dollars(whole(dollarsPerYear * 100))} a year. A win of ${unitWord(win, unit)} would be worth ${dollars(whole(per * win))} a year.`;
}

/** The smallest power of ten whose worth reads as at least $100 a year: "Each 100 ms is worth $3,600 a year". */
export function readableUnitSize(perUnitCents: number): number {
  let size = 1;
  while (size < 1e9 && perUnitCents * size < 10_000) size *= 10;
  return size;
}

/** "Each 100 ms is worth $3,600 a year", from a stored model. */
export function modelLine(model: Pick<Schemas["ValueModel"], "unit" | "centsPerUnit">): string {
  const size = readableUnitSize(model.centsPerUnit);
  return `Each ${unitWord(size, model.unit)} is worth ${dollars(whole(model.centsPerUnit * size))} a year`;
}

/** Return as the page words it: "59×" from 59.4, "4.2×" under ten; "-" when nothing was spent. */
export function returnLabel(returnX: number | null | undefined): string {
  if (returnX === null || returnX === undefined || !Number.isFinite(returnX)) return "-";
  if (returnX >= 10) return `${Math.round(returnX).toLocaleString("en-US")}×`;
  return `${Number(returnX.toFixed(1))}×`;
}

/** The fields of the Add value model form, as typed. */
export interface ValueForm {
  metric: string;
  unit: string;
  unitSize: string;
  dollarsPerYear: string;
  source: string;
  link: string;
}

export type ValueFormField = keyof ValueForm;

export function isHttpUrl(text: string): boolean {
  try {
    const url = new URL(text);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

/** The number a person typed for the size of one unit: more than zero. */
export function parseUnitSize(text: string): number | null {
  const value = Number(text.replace(/[,\s]/g, ""));
  return text.trim() !== "" && Number.isFinite(value) && value > 0 ? value : null;
}

export type ValueFormResult =
  | { ok: true; body: Schemas["NewValueModel"] }
  | { ok: false; errors: Partial<Record<ValueFormField, string>> };

/** Checks the form and builds what the server takes. Messages are words.ts keys' text, passed in. */
export function checkValueForm(
  form: ValueForm,
  say: Record<ValueFormField, string>,
): ValueFormResult {
  const errors: Partial<Record<ValueFormField, string>> = {};
  if (form.metric.trim() === "") errors.metric = say.metric;
  if (form.unit.trim() === "") errors.unit = say.unit;
  const size = parseUnitSize(form.unitSize);
  if (size === null) errors.unitSize = say.unitSize;
  const dollarCents = parseDollars(form.dollarsPerYear);
  if (dollarCents === null || dollarCents === 0) errors.dollarsPerYear = say.dollarsPerYear;
  if (form.source.trim() === "") errors.source = say.source;
  if (form.link.trim() !== "" && !isHttpUrl(form.link.trim())) errors.link = say.link;
  if (Object.keys(errors).length > 0 || size === null || dollarCents === null)
    return { ok: false, errors };
  const per = centsPerUnit(size, dollarCents / 100);
  if (per === null) return { ok: false, errors: { dollarsPerYear: say.dollarsPerYear } };
  const link = form.link.trim();
  return {
    ok: true,
    body: {
      metric: form.metric.trim(),
      unit: form.unit.trim(),
      centsPerUnit: per,
      source: { text: form.source.trim(), link: link === "" ? null : link },
    },
  };
}

/** The last `count` months as `YYYY-MM` values with names, newest first, in UTC. */
export function monthOptions(now: Date, count = 12): { value: string; label: string }[] {
  const format = new Intl.DateTimeFormat("en-US", {
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  });
  const out: { value: string; label: string }[] = [];
  for (let back = 0; back < count; back += 1) {
    const at = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() - back, 1));
    const month = String(at.getUTCMonth() + 1).padStart(2, "0");
    out.push({ value: `${at.getUTCFullYear()}-${month}`, label: format.format(at) });
  }
  return out;
}

/** Whole-number share of a total, 0 when the total is 0. */
export function percent(part: number, total: number): number {
  return total <= 0 ? 0 : Math.round((part / total) * 100);
}

/** Cost per delivered win, or null when no win was delivered. */
export function costPerWin(spentCents: number, winsDelivered: number): number | null {
  return winsDelivered > 0 ? Math.round(spentCents / winsDelivered) : null;
}

export interface AgentTotals {
  trials: number;
  kept: number;
  winsDelivered: number;
  winsPending: number;
  pendingCents: number;
  spentCents: number;
  deliveredCents: number;
}

/** The "All agents" row: the sum of every agent's row. */
export function agentTotals(rows: readonly Schemas["AgentRow"][]): AgentTotals {
  const total: AgentTotals = {
    trials: 0,
    kept: 0,
    winsDelivered: 0,
    winsPending: 0,
    pendingCents: 0,
    spentCents: 0,
    deliveredCents: 0,
  };
  for (const row of rows) {
    total.trials += row.trials;
    total.kept += row.kept;
    total.winsDelivered += row.winsDelivered;
    total.winsPending += row.winsPending;
    total.pendingCents += row.pendingCents;
    total.spentCents += row.spentCents;
    total.deliveredCents += row.deliveredCents;
  }
  return total;
}

/** "N merged, M open" for a project's wins. */
export function winsLine(figures: Pick<Schemas["ValueFigures"], "mergedWins" | "pendingWins">): string {
  return `${figures.mergedWins} merged, ${figures.pendingWins} open`;
}

/** "2 merged wins" / "1 pull request open". */
export function mergedWinsText(count: number): string {
  return plural(count, "merged win");
}
export function openPullRequestsText(count: number): string {
  return `${plural(count, "pull request")} open`;
}

/** The baseline-to-best change of a win, as a share of the baseline, signed: "-15.9%". */
export function winChange(baseline: number, best: number): string | null {
  if (baseline === 0) return null;
  const change = ((best - baseline) / Math.abs(baseline)) * 100;
  return `${change > 0 ? "+" : ""}${change.toFixed(1)}%`;
}

/** The metric moved this much, written with its unit: "1,310 ms to 1,102 ms". */
export function fromTo(win: Pick<Schemas["WinView"], "baseline" | "best" | "unit">): string {
  const n = (value: number) => Number(value.toFixed(2)).toLocaleString("en-US");
  const unit = win.unit === "" ? "" : ` ${win.unit}`;
  return `${n(win.baseline)}${unit} to ${n(win.best)}${unit}`;
}

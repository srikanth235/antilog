// The `history` tool answers with free-form JSON. It is narrowed here, once, before anything reads it.

export type OutcomeKey =
  | "measuring"
  | "kept"
  | "withinNoise"
  | "worse"
  | "guardrailFailed"
  | "failedHoldout"
  | "rejected"
  | "failed";

const OUTCOMES: readonly OutcomeKey[] = [
  "measuring",
  "kept",
  "withinNoise",
  "worse",
  "guardrailFailed",
  "failedHoldout",
  "rejected",
  "failed",
];

export const OUTCOME_WORDS: Record<OutcomeKey, string> = {
  measuring: "Measuring",
  kept: "Kept",
  withinNoise: "Within noise",
  worse: "Worse",
  guardrailFailed: "Guardrail failed",
  failedHoldout: "Failed holdout",
  rejected: "Rejected",
  failed: "failed",
};

export const OUTCOME_COLOR: Record<OutcomeKey, string> = {
  measuring: "var(--ink3)",
  kept: "var(--good)",
  withinNoise: "var(--ink3)",
  worse: "var(--bad)",
  guardrailFailed: "var(--bad)",
  failedHoldout: "var(--bad)",
  rejected: "var(--bad)",
  failed: "var(--bad)",
};

export interface TrialRow {
  /** Per experiment, across its sessions, counted from the first. */
  number: number;
  trial: string;
  session: string;
  parent: string | null;
  hypothesis: string;
  outcome: OutcomeKey;
  value: number | null;
}

export interface History {
  trials: TrialRow[];
  best: string | null;
  unit: string;
  version: number | null;
  note: string | null;
}

function record(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function text(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

export function parseHistory(answer: unknown): History {
  const top = record(answer);
  const rows = Array.isArray(top?.trials) ? (top.trials as unknown[]) : [];
  const trials: TrialRow[] = [];
  for (const raw of rows) {
    const row = record(raw);
    const trial = text(row?.trial);
    const session = text(row?.session);
    const outcome = text(row?.outcome);
    if (row === null || trial === null || session === null || outcome === null) continue;
    if (!OUTCOMES.includes(outcome as OutcomeKey)) continue;
    const value = row.value;
    trials.push({
      number: trials.length + 1,
      trial,
      session,
      parent: text(row.parent),
      hypothesis: text(row.hypothesis) ?? "",
      outcome: outcome as OutcomeKey,
      value: typeof value === "number" ? value : null,
    });
  }
  const version = top?.version;
  return {
    trials,
    best: text(top?.best),
    unit: text(top?.unit) ?? "",
    version: typeof version === "number" ? version : null,
    note: text(top?.note),
  };
}

export function kept(history: History): number {
  return history.trials.filter((row) => row.outcome === "kept").length;
}

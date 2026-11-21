import type { Schemas } from "../api/client";

/** The share of the window, in percent, from which the ring and the panel warn. */
export const CONTEXT_WARN_PERCENT = 80;

/** Tokens as Claude Code writes them: 950, 12.3k, 129.6k, 1M. */
export function tokens(n: number): string {
  if (n < 1000) return `${Math.round(n)}`;
  const trim = (value: number) => value.toFixed(1).replace(/\.0$/, "");
  if (n < 999_950) return `${trim(n / 1000)}k`;
  return `${trim(n / 1_000_000)}M`;
}

export type ContextRow = {
  /** The part's kind, or "free" for the window that is left. */
  kind: Schemas["ContextPartKind"] | "free";
  tokens: number;
  /** Percent of the window, one decimal. */
  percent: number;
};

export type ContextView = {
  used: number;
  window: number;
  /** Whole percent of the window used. */
  percent: number;
  warn: boolean;
  /** Only the parts the gateway measured, then the free space. */
  rows: ContextRow[];
};

const ORDER: Schemas["ContextPartKind"][] = ["system", "tools", "antilog_tools", "messages"];

/**
 * What the panel and the ring show, or null when the session has made no call yet or its
 * model's window is unknown: a figure we cannot back is left out.
 */
export function contextView(context: Schemas["SessionContext"] | undefined): ContextView | null {
  if (context === undefined) return null;
  const { used, window } = context;
  if (used === null || used === undefined || window === null || window === undefined) return null;
  if (window <= 0) return null;
  const share = (n: number) => Math.round((n / window) * 1000) / 10;
  const percent = Math.min(100, Math.round((used / window) * 100));
  const measured = ORDER.flatMap((kind) => {
    const part = context.parts.find((row) => row.kind === kind);
    return part === undefined ? [] : [{ kind, tokens: part.tokens, percent: share(part.tokens) }];
  });
  const free = Math.max(0, window - used);
  return {
    used,
    window,
    percent,
    warn: used * 100 >= window * CONTEXT_WARN_PERCENT,
    rows: [...measured, { kind: "free", tokens: free, percent: share(free) }],
  };
}

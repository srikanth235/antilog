import type { Schemas } from "../../api/client";
import { dollars } from "../../lib/format";
import { type History, OUTCOME_WORDS } from "../../lib/history";
import { w } from "../../words";

export interface CardRow {
  key: string;
  label: string;
  lines: string[];
  mono?: boolean;
}

/** What the card is asked, as the API words it: the summary before its first ": ", and the reason after it. */
export function splitSummary(summary: string): { lead: string; reason: string | null } {
  const at = summary.indexOf(": ");
  return at < 0
    ? { lead: summary, reason: null }
    : { lead: summary.slice(0, at), reason: summary.slice(at + 2) };
}

interface Context {
  budgetCapCents: number;
  spentCents: number | undefined;
  history: History;
}

/**
 * The detail rows the boards show above the buttons, filled from what the API serves: the experiment's budget
 * and spend, the card's own cost and summary, and the judge's history. A row whose data is not served is left out.
 */
export function cardRows(approval: Schemas["Approval"], ctx: Context): CardRow[] {
  const { lead, reason } = splitSummary(approval.summary);
  switch (approval.card) {
    case "raise_budget": {
      const rows: CardRow[] = [];
      const cap = ctx.budgetCapCents;
      if (ctx.spentCents !== undefined && approval.status === "pending")
        rows.push({
          key: "now",
          label: w.card.rows.now,
          lines: [
            ctx.spentCents >= cap
              ? w.card.rows.allSpent(dollars(cap))
              : w.card.rows.ofCap(dollars(ctx.spentCents), dollars(cap)),
          ],
        });
      if (approval.costCents != null && approval.status === "pending")
        rows.push({
          key: "requested",
          label: w.card.rows.requested,
          lines: [dollars(cap + approval.costCents)],
        });
      return [...rows, ...(reason === null ? [] : [why(reason)])];
    }
    case "attach_repo": {
      const repo = lead.replace(/^Attach repo\s+/, "");
      return [
        { key: "repo", label: w.card.rows.repo, lines: [repo], mono: true },
        { key: "access", label: w.card.rows.access, lines: [w.card.rows.pushesOwnBranch] },
      ];
    }
    case "add_credential": {
      const match = /^Add a read-only credential for (.+?) \((.+)\)$/.exec(lead);
      return match === null
        ? reason === null
          ? []
          : [why(reason)]
        : [
            { key: "connector", label: w.card.rows.connector, lines: [match[1] ?? ""] },
            ...(reason === null ? [] : [why(reason)]),
          ];
    }
    case "change_environment": {
      const changes = lead.replace(/^Change environment:?\s*/, "");
      return changes === "" ? [] : [{ key: "change", label: w.card.rows.change, lines: [changes] }];
    }
    case "keep_going": {
      const tried = ctx.history.trials.slice(-6).reverse();
      return tried.length === 0
        ? []
        : [
            {
              key: "tried",
              label: w.card.rows.tried,
              lines: tried.map((row) => `${row.hypothesis} · ${OUTCOME_WORDS[row.outcome]}`),
            },
          ];
    }
    default:
      return [];
  }
}

function why(reason: string): CardRow {
  return { key: "why", label: w.card.rows.why, lines: [reason] };
}

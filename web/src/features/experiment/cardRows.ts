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

interface Context {
  budgetCapCents: number;
  spentCents: number | undefined;
  history: History;
}

const r = w.card.rows;

function why(reason: string): CardRow {
  return { key: "why", label: r.why, lines: [reason] };
}

/**
 * What the card is asked, built from its typed payload. A card raised before payloads existed has none and
 * shows its summary as the API worded it; nothing is read out of that text.
 */
export function cardLead(approval: Schemas["Approval"]): string {
  const p = approval.payload;
  switch (p?.kind) {
    case "raise_budget":
      return w.card.lead.raiseBudget(dollars(p.requestedCents));
    case "attach_repo":
      return w.card.lead.attachRepo(p.repo);
    case "add_credential":
      return w.card.lead.addCredential(p.connector, p.scope);
    case "change_environment":
      return w.card.lead.changeEnvironment;
    case "open_pull_request":
      return w.card.lead.openPullRequest(p.hypothesis);
    case "use_connector_tool":
      return w.card.lead.askConnectorTool(p.tool, p.connector);
    default:
      return approval.summary;
  }
}

/**
 * The detail rows the boards show above the buttons, filled from the card's payload, the experiment's budget
 * and spend, and the judge's history. A row whose data is not served is left out.
 */
export function cardRows(approval: Schemas["Approval"], ctx: Context): CardRow[] {
  const p = approval.payload;
  switch (p?.kind) {
    case "raise_budget": {
      const rows: CardRow[] = [];
      if (approval.status === "pending") {
        const cap = p.nowCents;
        if (ctx.spentCents !== undefined)
          rows.push({
            key: "now",
            label: r.now,
            lines: [
              ctx.spentCents >= ctx.budgetCapCents
                ? r.allSpent(dollars(ctx.budgetCapCents))
                : r.ofCap(dollars(ctx.spentCents), dollars(ctx.budgetCapCents)),
            ],
          });
        rows.push({
          key: "requested",
          label: r.requested,
          lines: [dollars(cap + p.requestedCents)],
        });
      }
      return [...rows, ...(p.why === "" ? [] : [why(p.why)])];
    }
    case "attach_repo":
      return [
        { key: "repo", label: r.repo, lines: [p.repo], mono: true },
        {
          key: "access",
          label: r.access,
          lines: [p.access === "read" ? r.readsOnly : r.pushesOwnBranch],
        },
        ...(p.branch == null
          ? []
          : [{ key: "branch", label: r.branch, lines: [p.branch], mono: true }]),
      ];
    case "add_credential":
      return [
        { key: "connector", label: r.connector, lines: [p.connector] },
        { key: "scope", label: r.scope, lines: [p.scope] },
        ...(p.why === "" ? [] : [why(p.why)]),
      ];
    case "change_environment": {
      const rows: CardRow[] = [];
      if (p.image != null)
        rows.push({ key: "image", label: r.image, lines: [p.image], mono: true });
      if (p.setupScriptLines != null)
        rows.push({
          key: "setup",
          label: r.setupScript,
          lines: [r.setupScriptLines(p.setupScriptLines)],
        });
      if (p.size != null) rows.push({ key: "size", label: r.size, lines: [p.size] });
      if (p.network != null) rows.push({ key: "network", label: r.network, lines: [p.network] });
      return rows;
    }
    case "use_connector_tool":
      return [
        { key: "connector", label: r.connector, lines: [p.connector] },
        { key: "tool", label: r.tool, lines: [p.tool], mono: true },
        { key: "input", label: r.input, lines: [p.inputDigest], mono: true },
      ];
    case "open_pull_request":
      return [{ key: "repos", label: r.repos, lines: p.repos, mono: true }];
    case "ask_first":
      return [
        { key: "tool", label: r.tool, lines: [p.tool] },
        { key: "asked", label: r.asked, lines: [p.detail], mono: true },
      ];
    case "keep_going": {
      const tried = ctx.history.trials.slice(-6).reverse();
      return tried.length === 0
        ? []
        : [
            {
              key: "tried",
              label: r.tried,
              lines: tried.map((row) => `${row.hypothesis} · ${OUTCOME_WORDS[row.outcome]}`),
            },
          ];
    }
    default:
      return [];
  }
}

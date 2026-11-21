import type { Schemas } from "../../api/client";
import { dollars } from "../../lib/format";
import { w } from "../../words";

/** One changed line of a diff: `+` is added, `-` removed. */
export interface DiffLine {
  sign: "+" | "-";
  text: string;
}

/** A titled group of changed lines, such as the variables of an environment. */
export interface DiffGroup {
  title: string;
  lines: DiffLine[];
}

export interface CardRow {
  key: string;
  label: string;
  lines: string[];
  mono?: boolean;
  /** Shown as a diff under the lines. */
  diff?: DiffGroup[];
}

interface Context {
  budgetCapCents: number | null;
  spentCents: number | undefined;
}

const r = w.card.rows;

function why(reason: string): CardRow {
  return { key: "why", label: r.why, lines: [reason] };
}

const SCRIPT_LINES = 12;

function setDifference(from: string[], to: string[]) {
  return {
    added: to.filter((x) => !from.includes(x)),
    removed: from.filter((x) => !to.includes(x)),
  };
}

/**
 * What a Change environment card would change, from the environment before and after. The groups say
 * when each applies: the network at once, the variables for new commands, the script and repos now.
 */
export function environmentDiff(
  before: Schemas["EnvironmentView"],
  after: Schemas["EnvironmentView"],
): DiffGroup[] {
  const d = w.card.rows.diff;
  const groups: DiffGroup[] = [];

  const network: DiffLine[] = [];
  if (before.network !== after.network) {
    network.push({ sign: "-", text: d.level[before.network] });
    network.push({ sign: "+", text: d.level[after.network] });
  }
  if (before.allowPackageManagers !== after.allowPackageManagers)
    network.push({
      sign: after.allowPackageManagers ? "+" : "-",
      text: d.registries,
    });
  const hosts = setDifference(before.allowedHosts, after.allowedHosts);
  for (const host of hosts.removed) network.push({ sign: "-", text: host });
  for (const host of hosts.added) network.push({ sign: "+", text: host });
  if (network.length > 0) groups.push({ title: d.network, lines: network });

  const variables: DiffLine[] = [];
  const names = [
    ...new Set([...Object.keys(before.variables), ...Object.keys(after.variables)]),
  ].sort();
  for (const name of names) {
    const was = before.variables[name];
    const now = after.variables[name];
    if (was === now) continue;
    if (was !== undefined) variables.push({ sign: "-", text: `${name}=${was}` });
    if (now !== undefined) variables.push({ sign: "+", text: `${name}=${now}` });
  }
  if (variables.length > 0) groups.push({ title: d.variables, lines: variables });

  if (before.setupScript !== after.setupScript) {
    const lines: DiffLine[] = [];
    const push = (sign: "+" | "-", script: string) => {
      const rows = script.split("\n").filter((line) => line.trim() !== "");
      for (const text of rows.slice(0, SCRIPT_LINES)) lines.push({ sign, text });
      if (rows.length > SCRIPT_LINES)
        lines.push({ sign, text: d.moreLines(rows.length - SCRIPT_LINES) });
    };
    push("-", before.setupScript);
    push("+", after.setupScript);
    groups.push({ title: d.script, lines });
  }

  const repos = setDifference(before.repos, after.repos);
  const repoLines: DiffLine[] = [
    ...repos.removed.map((text): DiffLine => ({ sign: "-", text })),
    ...repos.added.map((text): DiffLine => ({ sign: "+", text })),
  ];
  if (repoLines.length > 0) groups.push({ title: d.repos, lines: repoLines });
  return groups;
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
    case "use_connector_tool":
      return w.card.lead.askConnectorTool(p.tool, p.connector);
    default:
      return approval.summary;
  }
}

/**
 * The detail rows the boards show above the buttons, filled from the card's payload, the session's budget
 * and spend. A row whose data is not served is left out.
 */
export function cardRows(approval: Schemas["Approval"], ctx: Context): CardRow[] {
  const p = approval.payload;
  switch (p?.kind) {
    case "raise_budget": {
      const rows: CardRow[] = [];
      if (approval.status === "pending") {
        const cap = p.nowCents;
        if (ctx.spentCents !== undefined && ctx.budgetCapCents !== null)
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
      const rows: CardRow[] = [
        {
          key: "environment",
          label: r.environment,
          lines: [
            p.environmentName,
            p.createsEnvironment
              ? r.environmentOwn
              : p.sessionVersion === p.baseVersion
                ? r.environmentShared(p.sessionVersion, p.baseVersion + 1)
                : r.environmentBehind(p.sessionVersion, p.baseVersion, p.baseVersion + 1),
          ],
        },
      ];
      const diff = environmentDiff(p.before, p.after);
      if (diff.length > 0) rows.push({ key: "change", label: r.change, lines: [], diff });
      return rows;
    }
    case "use_connector_tool":
      return [
        { key: "connector", label: r.connector, lines: [p.connector] },
        { key: "tool", label: r.tool, lines: [p.tool], mono: true },
        { key: "input", label: r.input, lines: [p.inputDigest], mono: true },
      ];
    case "ask_first":
      return [
        { key: "tool", label: r.tool, lines: [p.tool] },
        { key: "asked", label: r.asked, lines: [p.detail], mono: true },
      ];
    default:
      return [];
  }
}

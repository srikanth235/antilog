import type { Schemas } from "../api/client";
import type { Activity } from "../components/icon/Icon";
import { w } from "../words";

export type Role = Schemas["Role"];

export function canEdit(role: Role | undefined): boolean {
  return role !== "viewer";
}
export function isOwner(role: Role | undefined): boolean {
  return role === "owner";
}

/** What needs a person comes first: Waiting for you, then Working, then Failed, then Done. */
export function activityOf(
  sessions: readonly Schemas["Session"][],
  pendingCards: number,
): Activity | null {
  if (pendingCards > 0) return "waiting_for_you";
  if (sessions.length === 0) return null;
  const status = new Set(sessions.map((session) => session.status));
  if (status.has("waiting_for_you")) return "waiting_for_you";
  if (status.has("working")) return "working";
  if (status.has("failed") && !status.has("done")) return "failed";
  return "done";
}

/** The lead session is the first one started and has no parent. */
export function leadSession(
  sessions: readonly Schemas["Session"][],
): Schemas["Session"] | undefined {
  return [...sessions]
    .filter((session) => session.parent === null || session.parent === undefined)
    .sort((a, b) => a.createdAt - b.createdAt)[0];
}

export function pendingCards(approvals: readonly Schemas["Approval"][]): Schemas["Approval"][] {
  return approvals.filter((approval) => approval.status === "pending");
}

/** `antilog mcp` opens its sessions as agent "mcp". */
export const MCP_AGENT = "mcp";

/** The agent a session ran: its display name from the organization's list. An id never becomes a name. */
export function agentLabel(
  agents: readonly Pick<Schemas["Agent"], "id" | "name">[] | undefined,
  id: string,
): string {
  const found = agents?.find((agent) => agent.id === id);
  if (found !== undefined) return found.name;
  return id === MCP_AGENT ? w.session.yourAgent : w.session.unknownAgent;
}

/** The default budget the server applies to a new experiment: the project's, else the organization's, else none. */
export function defaultBudgetCents(
  project: Pick<Schemas["Project"], "defaultBudgetCents"> | undefined,
  organization: Pick<Schemas["Organization"], "defaultBudgetCents"> | undefined,
): number | null {
  return project?.defaultBudgetCents ?? organization?.defaultBudgetCents ?? null;
}

/** Repos typed as "owner/name, owner/name". Anything that is not owner/name is dropped. */
export function parseRepos(text: string): string[] {
  return text
    .split(/[,\s]+/)
    .map((repo) => repo.trim())
    .filter((repo) => /^[\w.-]+\/[\w.-]+$/.test(repo));
}

import type { Schemas } from "../api/client";
import type { Activity } from "../components/icon/Icon";

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

/** "claude-code" reads as "Claude Code". */
export function agentName(agent: string): string {
  return agent
    .split(/[-_ ]/)
    .filter((part) => part !== "")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

const PLATFORMS: Record<string, string> = {
  fake: "the fake platform",
  agentcore: "AWS AgentCore",
  managed: "Claude Managed Agents",
  openai: "OpenAI Agents API",
  foundry: "Microsoft Foundry Agent Service",
  google: "Google Agent Runtime",
};
export function platformName(platform: string): string {
  return PLATFORMS[platform] ?? agentName(platform);
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

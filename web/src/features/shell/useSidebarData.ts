import type { Schemas } from "../../api/client";
import { usePendingApprovals, useProjects, useSessions } from "../../api/queries";

/** Projects, each with its sessions (newest first), and the cards waiting on this person. */
export function useSidebarData() {
  const projects = useProjects();
  const sessions = useSessions();
  const pending = usePendingApprovals();
  const all = sessions.data ?? [];
  const groups = (projects.data ?? []).map((project) => ({
    project,
    sessions: all.filter((session) => session.projectId === project.id),
  }));
  const approvalsOf = new Map<string, Schemas["Approval"][]>(
    all.map((session) => [session.id, []]),
  );
  for (const approval of pending.data ?? []) approvalsOf.get(approval.sessionId)?.push(approval);
  return {
    loading: projects.isPending || sessions.isPending,
    groups,
    all,
    approvalsOf,
  };
}

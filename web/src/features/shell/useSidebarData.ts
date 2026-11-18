import { useQueries } from "@tanstack/react-query";
import type { Schemas } from "../../api/client";
import { api, unwrap } from "../../api/client";
import { keys, usePendingApprovals, useProjects } from "../../api/queries";

const SLOW_MS = 15000;

/** Projects, their experiments, each experiment's sessions, and the cards waiting on this person (one call). */
export function useSidebarData() {
  const projects = useProjects();
  const list = projects.data ?? [];
  const experiments = useQueries({
    queries: list.map((project) => ({
      queryKey: keys.projectExperiments(project.id),
      queryFn: async () =>
        unwrap(
          await api.GET("/projects/{id}/experiments", { params: { path: { id: project.id } } }),
        ),
      refetchInterval: SLOW_MS,
    })),
  });
  const all = experiments.flatMap((query) => query.data ?? []);
  const sessions = useQueries({
    queries: all.map((experiment) => ({
      queryKey: keys.sessions(experiment.id),
      queryFn: async () =>
        unwrap(
          await api.GET("/experiments/{id}/sessions", { params: { path: { id: experiment.id } } }),
        ),
      refetchInterval: SLOW_MS,
    })),
  });
  const pending = usePendingApprovals();
  const groups = list.map((project, index) => ({
    project,
    experiments: experiments[index]?.data ?? [],
  }));
  const sessionsOf = new Map(
    all.map((experiment, index) => [experiment.id, sessions[index]?.data ?? []]),
  );
  const approvalsOf = new Map<string, Schemas["Approval"][]>(
    all.map((experiment) => [experiment.id, []]),
  );
  for (const approval of pending.data ?? []) approvalsOf.get(approval.experimentId)?.push(approval);
  return {
    loading: projects.isPending || experiments.some((query) => query.isPending),
    groups,
    all,
    sessionsOf,
    approvalsOf,
  };
}

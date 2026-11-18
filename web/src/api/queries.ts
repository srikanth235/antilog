import {
  keepPreviousData,
  type QueryClient,
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { api, type Schemas, unwrap, withQuery } from "./client";

export const keys = {
  providers: () => ["providers"] as const,
  organization: () => ["organization"] as const,
  members: () => ["members"] as const,
  projects: () => ["projects"] as const,
  projectExperiments: (id: string) => ["projects", id, "experiments"] as const,
  projectRoles: (id: string) => ["projects", id, "roles"] as const,
  experiment: (id: string) => ["experiments", id] as const,
  sessions: (id: string) => ["experiments", id, "sessions"] as const,
  approvals: (id: string) => ["experiments", id, "approvals"] as const,
  session: (id: string) => ["sessions", id] as const,
  receipt: (trial: string) => ["trials", trial, "receipt"] as const,
  history: (id: string) => ["experiments", id, "history"] as const,
  budgets: () => ["budgets"] as const,
  policies: () => ["policies"] as const,
  tokens: () => ["tokens"] as const,
  audit: () => ["audit"] as const,
  tools: () => ["tools"] as const,
};

/** Sessions and approvals change while an agent works; the page asks again every few seconds. */
const LIVE_MS = 4000;

export function useProviders() {
  return useQuery({
    queryKey: keys.providers(),
    queryFn: async () => unwrap(await api.GET("/auth/providers")).providers,
    retry: false,
  });
}

/** Also the "am I signed in" probe: a 401 means no. */
export function useOrganization() {
  return useQuery({
    queryKey: keys.organization(),
    queryFn: async () => unwrap(await api.GET("/organization")),
    retry: false,
    staleTime: 60_000,
  });
}

export function useMembers() {
  return useQuery({
    queryKey: keys.members(),
    queryFn: async () => unwrap(await api.GET("/members")),
  });
}

export function useProjects() {
  return useQuery({
    queryKey: keys.projects(),
    queryFn: async () => unwrap(await api.GET("/projects")),
  });
}

export function useProjectExperiments(id: string) {
  return useQuery({
    queryKey: keys.projectExperiments(id),
    queryFn: async () =>
      unwrap(await api.GET("/projects/{id}/experiments", { params: { path: { id } } })),
  });
}

export function useProjectRoles(id: string) {
  return useQuery({
    queryKey: keys.projectRoles(id),
    queryFn: async () =>
      unwrap(await api.GET("/projects/{id}/roles", { params: { path: { id } } })),
  });
}

export function useExperiment(id: string) {
  return useQuery({
    queryKey: keys.experiment(id),
    queryFn: async () => unwrap(await api.GET("/experiments/{id}", { params: { path: { id } } })),
  });
}

export function useExperimentSessions(id: string) {
  return useQuery({
    queryKey: keys.sessions(id),
    queryFn: async () =>
      unwrap(await api.GET("/experiments/{id}/sessions", { params: { path: { id } } })),
    refetchInterval: LIVE_MS,
  });
}

export function useExperimentApprovals(id: string) {
  return useQuery({
    queryKey: keys.approvals(id),
    queryFn: async () =>
      unwrap(await api.GET("/experiments/{id}/approvals", { params: { path: { id } } })),
    refetchInterval: LIVE_MS,
  });
}

export function useReceipt(trial: string) {
  return useQuery({
    queryKey: keys.receipt(trial),
    queryFn: async () =>
      unwrap(await api.GET("/trials/{id}/receipt", { params: { path: { id: trial } } })),
    retry: false,
  });
}

export function useBudgets() {
  return useQuery({
    queryKey: keys.budgets(),
    queryFn: async () => unwrap(await api.GET("/budgets")),
  });
}

export function usePolicies() {
  return useQuery({
    queryKey: keys.policies(),
    queryFn: async () => unwrap(await api.GET("/policies")),
  });
}

export function useTokens() {
  return useQuery({
    queryKey: keys.tokens(),
    queryFn: async () =>
      unwrap(await api.GET("/tokens", { params: withQuery({ memberId: null }) })),
  });
}

export function useAudit() {
  return useInfiniteQuery({
    queryKey: keys.audit(),
    initialPageParam: null as Schemas["AuditCursor"] | null,
    queryFn: async ({ pageParam }) =>
      unwrap(
        await api.GET("/audit", {
          params: withQuery({
            limit: 50,
            beforeAt: pageParam?.at ?? null,
            beforeId: pageParam?.id ?? null,
          }),
        }),
      ),
    getNextPageParam: (page) => page.next ?? null,
    placeholderData: keepPreviousData,
  });
}

export function useTools() {
  return useQuery({
    queryKey: keys.tools(),
    queryFn: async () => unwrap(await api.GET("/tools")),
    staleTime: Number.POSITIVE_INFINITY,
  });
}

/** `history`, the judge's list of every trial. Its answer is free-form; the page narrows it. */
export function useHistory(experiment: string) {
  return useQuery({
    queryKey: keys.history(experiment),
    queryFn: async () =>
      unwrap(
        await api.POST("/experiments/{id}/tools/{tool}", {
          params: { path: { id: experiment, tool: "history" } },
          body: { input: { limit: 100 } },
        }),
      ).answer as unknown,
    retry: false,
    refetchInterval: LIVE_MS * 2,
  });
}

function useWrite<V, R>(
  write: (variables: V) => Promise<R>,
  invalidate: (client: QueryClient, variables: V, result: R) => Promise<unknown> | undefined,
) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: write,
    onSuccess: (result, variables) => invalidate(client, variables, result),
  });
}

export function useCreateProject() {
  return useWrite(
    async (body: Schemas["NewProject"]) => unwrap(await api.POST("/projects", { body })),
    (client) => client.invalidateQueries({ queryKey: keys.projects() }),
  );
}

export function useCreateExperiment(project: string) {
  return useWrite(
    async (body: Schemas["NewExperiment"]) =>
      unwrap(
        await api.POST("/projects/{id}/experiments", { params: { path: { id: project } }, body }),
      ),
    (client) => client.invalidateQueries({ queryKey: keys.projectExperiments(project) }),
  );
}

export function useStartSession(experiment: string) {
  return useWrite(
    async (body: Schemas["StartSession"]) =>
      unwrap(
        await api.POST("/experiments/{id}/sessions", {
          params: { path: { id: experiment } },
          body,
        }),
      ),
    (client) => client.invalidateQueries({ queryKey: keys.sessions(experiment) }),
  );
}

export function useSendMessage(experiment: string, session: string) {
  return useWrite(
    async (text: string) =>
      unwrap(
        await api.POST("/sessions/{id}/messages", {
          params: { path: { id: session } },
          body: { text },
        }),
      ),
    (client) => client.invalidateQueries({ queryKey: keys.sessions(experiment) }),
  );
}

export function useStopSession(experiment: string) {
  return useWrite(
    async (session: string) =>
      unwrap(await api.POST("/sessions/{id}/stop", { params: { path: { id: session } } })),
    (client) => client.invalidateQueries({ queryKey: keys.sessions(experiment) }),
  );
}

export function useDecide(experiment: string) {
  return useWrite(
    async (v: { id: string; decision: Schemas["Decision"] }) =>
      unwrap(
        await api.POST("/approvals/{id}/decision", {
          params: { path: { id: v.id } },
          body: { decision: v.decision },
        }),
      ),
    async (client) => {
      await client.invalidateQueries({ queryKey: keys.approvals(experiment) });
      await client.invalidateQueries({ queryKey: keys.experiment(experiment) });
      await client.invalidateQueries({ queryKey: keys.budgets() });
    },
  );
}

export function useSetExperimentBudget(experiment: string) {
  return useWrite(
    async (budgetCapCents: number) =>
      unwrap(
        await api.PUT("/experiments/{id}/budget", {
          params: { path: { id: experiment } },
          body: { budgetCapCents },
        }),
      ),
    (client) => client.invalidateQueries({ queryKey: keys.experiment(experiment) }),
  );
}

export function useAddMember() {
  return useWrite(
    async (body: Schemas["NewMember"]) => unwrap(await api.POST("/members", { body })),
    (client) => client.invalidateQueries({ queryKey: keys.members() }),
  );
}

export function useSetMemberRole() {
  return useWrite(
    async (v: { id: string; role: Schemas["Role"] }) =>
      unwrap(
        await api.PUT("/members/{id}/role", {
          params: { path: { id: v.id } },
          body: { role: v.role },
        }),
      ),
    (client) => client.invalidateQueries({ queryKey: keys.members() }),
  );
}

export function useCreateToken() {
  return useWrite(
    async (body: Schemas["NewToken"]) => unwrap(await api.POST("/tokens", { body })),
    (client) => client.invalidateQueries({ queryKey: keys.tokens() }),
  );
}

export function useRevokeToken() {
  return useWrite(
    async (id: string) => unwrap(await api.DELETE("/tokens/{id}", { params: { path: { id } } })),
    (client) => client.invalidateQueries({ queryKey: keys.tokens() }),
  );
}

export function useSetPolicy() {
  return useWrite(
    async (body: Schemas["Policy"]) => unwrap(await api.PUT("/policies", { body })),
    (client) => client.invalidateQueries({ queryKey: keys.policies() }),
  );
}

export function useSetOrganizationLimits() {
  return useWrite(
    async (body: Schemas["Limits"]) => unwrap(await api.PUT("/organization/limits", { body })),
    async (client) => {
      await client.invalidateQueries({ queryKey: keys.organization() });
      await client.invalidateQueries({ queryKey: keys.budgets() });
    },
  );
}

export function useSetProjectLimits(project: string) {
  return useWrite(
    async (body: Schemas["Limits"]) =>
      unwrap(await api.PUT("/projects/{id}/limits", { params: { path: { id: project } }, body })),
    async (client) => {
      await client.invalidateQueries({ queryKey: keys.projects() });
      await client.invalidateQueries({ queryKey: keys.budgets() });
    },
  );
}

export function useSetProjectRole(project: string) {
  return useWrite(
    async (v: { member: string; role: Schemas["Role"] | null }) =>
      unwrap(
        await api.PUT("/projects/{id}/roles/{member}", {
          params: { path: { id: project, member: v.member } },
          body: { role: v.role },
        }),
      ),
    (client) => client.invalidateQueries({ queryKey: keys.projectRoles(project) }),
  );
}

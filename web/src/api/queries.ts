import {
  keepPreviousData,
  type QueryClient,
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { agentLabel } from "../lib/domain";
import { api, type Schemas, unwrap } from "./client";
import { useStreamLive } from "./liveStatus";

export const keys = {
  providers: () => ["providers"] as const,
  me: () => ["me"] as const,
  pending: () => ["approvals", "pending"] as const,
  organization: () => ["organization"] as const,
  members: () => ["members"] as const,
  projects: () => ["projects"] as const,
  projectSessions: (id: string) => ["projects", id, "sessions"] as const,
  projectRoles: (id: string) => ["projects", id, "roles"] as const,
  sessions: () => ["sessions"] as const,
  session: (id: string) => ["sessions", id] as const,
  sessionApprovals: (id: string) => ["sessions", id, "approvals"] as const,
  sessionSpend: (id: string) => ["sessions", id, "spend"] as const,
  agents: () => ["agents"] as const,
  autoAgent: () => ["agents", "auto"] as const,
  environments: () => ["environments"] as const,
  connectors: () => ["connectors"] as const,
  skills: () => ["skills"] as const,
  plugins: () => ["plugins"] as const,
  budgets: () => ["budgets"] as const,
  spend: () => ["spend"] as const,
  transcript: (session: string) => ["sessions", session, "events"] as const,
  policies: () => ["policies"] as const,
  tokens: () => ["tokens"] as const,
  modelKeys: () => ["model-keys"] as const,
  audit: () => ["audit"] as const,
};

/** Sessions and approvals change while an agent works. The server's event stream says when (`live.ts`);
 * until it delivers, and whenever it fails, the page asks again every few seconds. */
const LIVE_MS = 4000;

/** The poll for what the event stream covers: off while the stream delivers. */
function usePoll(): number | false {
  return useStreamLive() ? false : LIVE_MS;
}

export function useProviders() {
  return useQuery({
    queryKey: keys.providers(),
    queryFn: async () => unwrap(await api.GET("/auth/providers")).providers,
    retry: false,
  });
}

/** Who is signed in, and the "am I signed in" probe: a 401 means no. */
export function useMe() {
  return useQuery({
    queryKey: keys.me(),
    queryFn: async () => unwrap(await api.GET("/me")),
    retry: false,
    staleTime: 60_000,
  });
}

/** The signed-in member, once `/me` has answered. */
export function useMember() {
  return useMe().data?.member;
}

/** Every card across experiments the person may decide; Home and the sidebar read this one list. */
export function usePendingApprovals() {
  const poll = usePoll();
  return useQuery({
    queryKey: keys.pending(),
    queryFn: async () => unwrap(await api.GET("/approvals/pending")),
    refetchInterval: poll,
  });
}

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

export function useProjectSessions(id: string) {
  const poll = usePoll();
  return useQuery({
    queryKey: keys.projectSessions(id),
    queryFn: async () =>
      unwrap(await api.GET("/projects/{id}/sessions", { params: { path: { id } } })),
    refetchInterval: poll,
  });
}

export function useProjectRoles(id: string) {
  return useQuery({
    queryKey: keys.projectRoles(id),
    queryFn: async () =>
      unwrap(await api.GET("/projects/{id}/roles", { params: { path: { id } } })),
  });
}

/** Every session of the organization, newest first, across projects: the sidebar and Home read this. */
export function useSessions() {
  const poll = usePoll();
  return useQuery({
    queryKey: keys.sessions(),
    queryFn: async () => unwrap(await api.GET("/sessions")),
    refetchInterval: poll,
  });
}

export function useSession(id: string) {
  const poll = usePoll();
  return useQuery({
    queryKey: keys.session(id),
    queryFn: async () => unwrap(await api.GET("/sessions/{id}", { params: { path: { id } } })),
    refetchInterval: poll,
  });
}

export function useSessionApprovals(id: string) {
  const poll = usePoll();
  return useQuery({
    queryKey: keys.sessionApprovals(id),
    queryFn: async () =>
      unwrap(await api.GET("/sessions/{id}/approvals", { params: { path: { id } } })),
    refetchInterval: poll,
  });
}

/** One session's spend against its own cap, by kind. */
export function useSessionSpend(id: string) {
  return useQuery({
    queryKey: keys.sessionSpend(id),
    queryFn: async () =>
      unwrap(await api.GET("/sessions/{id}/spend", { params: { path: { id } } })),
    refetchInterval: LIVE_MS,
  });
}

/** The conversation of a session, oldest first: every page is read, and the list is asked for again while it works. */
export function useTranscript(session: string, status: Schemas["SessionStatus"] | undefined) {
  const poll = usePoll();
  return useQuery({
    queryKey: keys.transcript(session),
    enabled: session !== "",
    queryFn: async () => {
      const events: Schemas["SessionEvent"][] = [];
      let after: number | undefined;
      for (let page = 0; page < 40; page += 1) {
        const got = unwrap(
          await api.GET("/sessions/{id}/events", {
            params: {
              path: { id: session },
              query: { limit: 500, ...(after === undefined ? {} : { after }) },
            },
          }),
        );
        events.push(...got.events);
        if (got.next === null) break;
        after = got.next;
      }
      return events;
    },
    // The poll pauses in a hidden tab (the default), is off while the event stream delivers, and
    // ends once the session is Done or Failed.
    refetchInterval: status === "done" || status === "failed" ? false : poll,
  });
}

/** The agents an organization turned on, for the composer's Agent chip. */
export function useAgents() {
  return useQuery({
    queryKey: keys.agents(),
    queryFn: async () => unwrap(await api.GET("/agents")),
  });
}

/** Turns an agent id into its display name; "An agent" until the list has loaded. */
export function useAgentLabel() {
  const agents = useAgents();
  return (id: string) => agentLabel(agents.data, id);
}

/** What Auto would start and why; a refusal says to pick one. */
export function useAutoAgent() {
  return useQuery({
    queryKey: keys.autoAgent(),
    queryFn: async () => unwrap(await api.GET("/agents/auto")),
    retry: false,
  });
}

export function useEnvironments() {
  return useQuery({
    queryKey: keys.environments(),
    queryFn: async () => unwrap(await api.GET("/environments")),
  });
}

function refreshEnvironments(client: QueryClient) {
  return client.invalidateQueries({ queryKey: keys.environments() });
}

export function useCreateEnvironment() {
  return useWrite(
    async (body: Schemas["NewEnvironment"]) => unwrap(await api.POST("/environments", { body })),
    refreshEnvironments,
  );
}

/** An edit is the environment's next version. */
export function useEditEnvironment() {
  return useWrite(
    async (vars: { id: string; body: Schemas["NewEnvironment"] }) =>
      unwrap(
        await api.PUT("/environments/{id}", {
          params: { path: { id: vars.id } },
          body: vars.body,
        }),
      ),
    refreshEnvironments,
  );
}

export function useArchiveEnvironment() {
  return useWrite(
    async (id: string) =>
      unwrap(await api.POST("/environments/{id}/archive", { params: { path: { id } } })),
    refreshEnvironments,
  );
}

export function useConnectors() {
  return useQuery({
    queryKey: keys.connectors(),
    queryFn: async () => unwrap(await api.GET("/connectors")),
  });
}

export function useConnector(id: string) {
  return useQuery({
    queryKey: [...keys.connectors(), id] as const,
    queryFn: async () => unwrap(await api.GET("/connectors/{id}", { params: { path: { id } } })),
    retry: false,
  });
}

const refreshConnectors = (client: QueryClient) =>
  client.invalidateQueries({ queryKey: keys.connectors() });

export function useAddConnector() {
  return useWrite(
    async (body: Schemas["NewConnector"]) => unwrap(await api.POST("/connectors", { body })),
    refreshConnectors,
  );
}

export function useCheckConnector(id: string) {
  return useWrite(
    async () => unwrap(await api.POST("/connectors/{id}/check", { params: { path: { id } } })),
    refreshConnectors,
  );
}

export function useSetConnectorCredential(id: string) {
  return useWrite(
    async (credential: string) =>
      unwrap(
        await api.PUT("/connectors/{id}/credential", {
          params: { path: { id } },
          body: { credential },
        }),
      ),
    refreshConnectors,
  );
}

export function useSetToolMode(id: string) {
  return useWrite(
    async (change: { tool: string; mode: Schemas["ToolMode"] }) =>
      unwrap(
        await api.PUT("/connectors/{id}/tools/{tool}", {
          params: { path: { id, tool: change.tool } },
          body: { mode: change.mode },
        }),
      ),
    refreshConnectors,
  );
}

export function useRemoveConnector(id: string) {
  return useWrite(
    async () => unwrap(await api.DELETE("/connectors/{id}", { params: { path: { id } } })),
    refreshConnectors,
  );
}

/** The organization's skill packs. */
export function useSkills() {
  return useQuery({
    queryKey: keys.skills(),
    queryFn: async () => unwrap(await api.GET("/skills")),
  });
}

export function useSkillVersion(name: string, version: number | undefined) {
  return useQuery({
    queryKey: [...keys.skills(), name, "versions", version] as const,
    enabled: version !== undefined,
    queryFn: async () =>
      unwrap(
        await api.GET("/skills/{name}/versions/{version}", {
          params: { path: { name, version: version ?? 0 } },
        }),
      ),
  });
}

const refreshSkills = (client: QueryClient) =>
  client.invalidateQueries({ queryKey: keys.skills() });

export function useAddSkill() {
  return useWrite(
    async (body: Schemas["NewSkill"]) => unwrap(await api.POST("/skills", { body })),
    refreshSkills,
  );
}

export function useRemoveSkill(name: string) {
  return useWrite(
    async () => unwrap(await api.DELETE("/skills/{name}", { params: { path: { name } } })),
    refreshSkills,
  );
}

export function usePlugins() {
  return useQuery({
    queryKey: keys.plugins(),
    queryFn: async () => unwrap(await api.GET("/plugins")),
  });
}

/** A plugin brings or takes away skill packs and connectors. */
function refreshPlugin(client: QueryClient) {
  return Promise.all(
    [keys.plugins(), keys.skills(), keys.connectors()].map((queryKey) =>
      client.invalidateQueries({ queryKey }),
    ),
  );
}

export function useAddPlugin() {
  return useWrite(
    async (body: Schemas["NewPlugin"]) => unwrap(await api.POST("/plugins", { body })),
    refreshPlugin,
  );
}

export function useRemovePlugin(id: string) {
  return useWrite(
    async (keep: Schemas["PluginPart"][]) =>
      unwrap(await api.POST("/plugins/{id}/remove", { params: { path: { id } }, body: { keep } })),
    refreshPlugin,
  );
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
    queryFn: async () => unwrap(await api.GET("/tokens", { params: { query: {} } })),
  });
}

export function useModelKeys() {
  return useQuery({
    queryKey: keys.modelKeys(),
    queryFn: async () => unwrap(await api.GET("/model-keys")),
  });
}

export function useAudit() {
  return useInfiniteQuery({
    queryKey: keys.audit(),
    initialPageParam: null as Schemas["AuditCursor"] | null,
    queryFn: async ({ pageParam }) =>
      unwrap(
        await api.GET("/audit", {
          params: {
            query: {
              limit: 50,
              ...(pageParam === null ? {} : { beforeAt: pageParam.at, beforeId: pageParam.id }),
            },
          },
        }),
      ),
    getNextPageParam: (page) => page.next ?? null,
    placeholderData: keepPreviousData,
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

/** Every list of sessions: the sidebar, a project's page, one session. */
const refreshSessions = (client: QueryClient) =>
  client.invalidateQueries({ queryKey: keys.sessions() });

export function useStartSession(project: string) {
  return useWrite(
    async (body: Schemas["StartSession"]) =>
      unwrap(
        await api.POST("/projects/{id}/sessions", { params: { path: { id: project } }, body }),
      ),
    async (client) => {
      await refreshSessions(client);
      await client.invalidateQueries({ queryKey: keys.projectSessions(project) });
    },
  );
}

export function useSendMessage(session: string) {
  return useWrite(
    async (text: string) =>
      unwrap(
        await api.POST("/sessions/{id}/messages", {
          params: { path: { id: session } },
          body: { text },
        }),
      ),
    (client) => client.invalidateQueries({ queryKey: keys.transcript(session) }),
  );
}

export function useStopSession() {
  return useWrite(
    async (session: string) =>
      unwrap(await api.POST("/sessions/{id}/stop", { params: { path: { id: session } } })),
    refreshSessions,
  );
}

export function useDecide(session: string) {
  return useWrite(
    async (v: { id: string; decision: Schemas["Decision"] }) =>
      unwrap(
        await api.POST("/approvals/{id}/decision", {
          params: { path: { id: v.id } },
          body: { decision: v.decision },
        }),
      ),
    async (client) => {
      await client.invalidateQueries({ queryKey: keys.sessionApprovals(session) });
      await client.invalidateQueries({ queryKey: keys.pending() });
      await client.invalidateQueries({ queryKey: keys.session(session) });
      await client.invalidateQueries({ queryKey: keys.budgets() });
    },
  );
}

export function useSetSessionBudget(session: string) {
  return useWrite(
    async (budgetCapCents: number) =>
      unwrap(
        await api.PUT("/sessions/{id}/budget", {
          params: { path: { id: session } },
          body: { budgetCapCents },
        }),
      ),
    async (client) => {
      await client.invalidateQueries({ queryKey: keys.session(session) });
      await client.invalidateQueries({ queryKey: keys.sessionSpend(session) });
    },
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

export function useSetModelKey() {
  return useWrite(
    async (vars: { provider: Schemas["ModelProvider"]; key: string }) =>
      unwrap(
        await api.PUT("/model-keys/{provider}", {
          params: { path: { provider: vars.provider } },
          body: { key: vars.key },
        }),
      ),
    (client) => client.invalidateQueries({ queryKey: keys.modelKeys() }),
  );
}

export function useRemoveModelKey() {
  return useWrite(
    async (provider: Schemas["ModelProvider"]) =>
      unwrap(await api.DELETE("/model-keys/{provider}", { params: { path: { provider } } })),
    (client) => client.invalidateQueries({ queryKey: keys.modelKeys() }),
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

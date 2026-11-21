import { type QueryClient, useQueryClient } from "@tanstack/react-query";
import {
  createRootRouteWithContext,
  createRoute,
  createRouter,
  Navigate,
  Outlet,
  redirect,
} from "@tanstack/react-router";
import { ApiFailure, api, unwrap } from "./api/client";
import { useLiveUpdates } from "./api/live";
import { keys, useMe } from "./api/queries";
import { TipProvider } from "./components/tooltip/Tooltip";
import { AddConnector } from "./features/customize/AddConnector";
import { AddPlugin } from "./features/customize/AddPlugin";
import { AddSkill } from "./features/customize/AddSkill";
import { ConnectorDetail } from "./features/customize/ConnectorDetail";
import { Connectors } from "./features/customize/Connectors";
import { PluginDetail } from "./features/customize/PluginDetail";
import { Plugins } from "./features/customize/Plugins";
import { SkillDetail } from "./features/customize/SkillDetail";
import { Skills } from "./features/customize/Skills";
import type { Panel } from "./features/experiment/Header";
import { Session } from "./features/experiment/Session";
import { TerminalWindow } from "./features/experiment/Terminal";
import { Home } from "./features/home/Home";
import { Project } from "./features/projects/Project";
import { Projects } from "./features/projects/Projects";
import { SECTIONS, type Section, SettingsDialog } from "./features/settings/SettingsDialog";
import { Sidebar } from "./features/shell/Sidebar";
import { Callback } from "./features/signin/Callback";
import { SignIn } from "./features/signin/SignIn";

interface Context {
  queryClient: QueryClient;
}

const root = createRootRouteWithContext<Context>()({
  component: () => (
    <TipProvider>
      <Outlet />
    </TipProvider>
  ),
});

function str(value: unknown): string | undefined {
  return typeof value === "string" && value !== "" ? value : undefined;
}

const signin = createRoute({
  getParentRoute: () => root,
  path: "/signin",
  validateSearch: (search: Record<string, unknown>): { error?: "refused" | "noaccess" } =>
    search.error === "refused" || search.error === "noaccess" ? { error: search.error } : {},
  component: function SignInPage() {
    const { error } = signin.useSearch();
    return (
      <div className="board app shell">
        <SignIn failure={error} />
      </div>
    );
  },
});

const callback = createRoute({
  getParentRoute: () => root,
  path: "/auth/$provider/callback",
  validateSearch: (search: Record<string, unknown>) => ({
    code: str(search.code) ?? "",
    state: str(search.state) ?? "",
  }),
  component: function CallbackPage() {
    const { provider } = callback.useParams();
    const { code, state } = callback.useSearch();
    return (
      <div className="board app shell">
        <Callback provider={provider} code={code} state={state} />
      </div>
    );
  },
});

/** Everything behind sign-in: a 401 on the organization sends the browser to /signin. */
const app = createRoute({
  getParentRoute: () => root,
  id: "app",
  beforeLoad: async ({ context }) => {
    try {
      await context.queryClient.fetchQuery({
        queryKey: keys.me(),
        queryFn: async () => unwrap(await api.GET("/me")),
        staleTime: 60_000,
        retry: false,
      });
    } catch (caught) {
      if (caught instanceof ApiFailure && (caught.status === 401 || caught.status === 403))
        throw redirect({ to: "/signin" });
      throw caught;
    }
  },
  component: function Shell() {
    useMe();
    useLiveUpdates(useQueryClient());
    return (
      <div className="board app shell">
        <Sidebar />
        <main className="main">
          <Outlet />
        </main>
      </div>
    );
  },
});

function Scroll({ children, column }: { children: React.ReactNode; column?: boolean }) {
  return (
    <div
      className="scroll"
      {...(column === true ? { style: { display: "flex", flexDirection: "column" } } : {})}
    >
      {children}
    </div>
  );
}

const home = createRoute({
  getParentRoute: () => app,
  path: "/",
  validateSearch: (search: Record<string, unknown>): { project?: string } => {
    const project = str(search.project);
    return project === undefined ? {} : { project };
  },
  component: function HomePage() {
    const { project } = home.useSearch();
    return (
      <Scroll column>
        <Home project={project} />
      </Scroll>
    );
  },
});

const projects = createRoute({
  getParentRoute: () => app,
  path: "/projects",
  component: () => (
    <Scroll>
      <Projects />
    </Scroll>
  ),
});

const project = createRoute({
  getParentRoute: () => app,
  path: "/projects/$projectId",
  component: function ProjectPage() {
    const { projectId } = project.useParams();
    return (
      <Scroll>
        <Project projectId={projectId} />
      </Scroll>
    );
  },
});

const PANELS: readonly Panel[] = ["details", "changes", "terminal", "preview", "files"];

const session = createRoute({
  getParentRoute: () => app,
  path: "/sessions/$sessionId",
  validateSearch: (search: Record<string, unknown>): { panel?: Panel } => {
    const panel = PANELS.find((candidate) => candidate === search.panel);
    return panel === undefined ? {} : { panel };
  },
  component: function SessionPage() {
    const { sessionId } = session.useParams();
    const { panel } = session.useSearch();
    const navigate = session.useNavigate();
    return (
      <Session
        key={sessionId}
        sessionId={sessionId}
        panel={panel}
        onPanel={(next) =>
          void navigate({
            search: (prev) => {
              const { panel: _drop, ...rest } = prev;
              return next === undefined ? rest : { ...rest, panel: next };
            },
            replace: true,
          })
        }
      />
    );
  },
});

/** A terminal in a window of its own, opened from the panel's pop-out. */
const terminalWindow = createRoute({
  getParentRoute: () => root,
  path: "/sessions/$sessionId/terminal",
  beforeLoad: async ({ context }) => {
    try {
      await context.queryClient.fetchQuery({
        queryKey: keys.me(),
        queryFn: async () => unwrap(await api.GET("/me")),
        staleTime: 60_000,
        retry: false,
      });
    } catch (caught) {
      if (caught instanceof ApiFailure && (caught.status === 401 || caught.status === 403))
        throw redirect({ to: "/signin" });
      throw caught;
    }
  },
  component: function TerminalPage() {
    const { sessionId } = terminalWindow.useParams();
    return <TerminalWindow sessionId={sessionId} />;
  },
});

const customize = createRoute({
  getParentRoute: () => app,
  path: "/customize",
  component: () => <Navigate to="/customize/skills" replace />,
});
const skills = createRoute({
  getParentRoute: () => app,
  path: "/customize/skills",
  component: () => (
    <Scroll>
      <Skills />
    </Scroll>
  ),
});
const addSkill = createRoute({
  getParentRoute: () => app,
  path: "/customize/skills/new",
  component: () => (
    <Scroll>
      <AddSkill />
    </Scroll>
  ),
});
const skillDetail = createRoute({
  getParentRoute: () => app,
  path: "/customize/skills/$name",
  component: function SkillPage() {
    const { name } = skillDetail.useParams();
    return (
      <Scroll>
        <SkillDetail name={name} />
      </Scroll>
    );
  },
});
const connectors = createRoute({
  getParentRoute: () => app,
  path: "/customize/connectors",
  component: () => (
    <Scroll>
      <Connectors />
    </Scroll>
  ),
});
const addConnector = createRoute({
  getParentRoute: () => app,
  path: "/customize/connectors/new",
  component: () => (
    <Scroll>
      <AddConnector />
    </Scroll>
  ),
});
const connectorDetail = createRoute({
  getParentRoute: () => app,
  path: "/customize/connectors/$id",
  component: function ConnectorPage() {
    const { id } = connectorDetail.useParams();
    return (
      <Scroll>
        <ConnectorDetail id={id} />
      </Scroll>
    );
  },
});
const plugins = createRoute({
  getParentRoute: () => app,
  path: "/customize/plugins",
  component: () => (
    <Scroll>
      <Plugins />
    </Scroll>
  ),
});
const addPlugin = createRoute({
  getParentRoute: () => app,
  path: "/customize/plugins/new",
  component: () => (
    <Scroll>
      <AddPlugin />
    </Scroll>
  ),
});
const pluginDetail = createRoute({
  getParentRoute: () => app,
  path: "/customize/plugins/$id",
  component: function PluginPage() {
    const { id } = pluginDetail.useParams();
    return (
      <Scroll>
        <PluginDetail id={id} />
      </Scroll>
    );
  },
});

const settings = createRoute({
  getParentRoute: () => app,
  path: "/settings/$section",
  component: function SettingsPage() {
    const { section } = settings.useParams();
    const known = SECTIONS.find((item): item is Section => item === section);
    if (known === undefined)
      return <Navigate to="/settings/$section" params={{ section: "profile" }} replace />;
    return (
      <>
        <Scroll column>
          <Home project={undefined} />
        </Scroll>
        <SettingsDialog section={known} />
      </>
    );
  },
});

const tree = root.addChildren([
  signin,
  callback,
  terminalWindow,
  app.addChildren([
    home,
    projects,
    project,
    session,
    customize,
    skills,
    addSkill,
    skillDetail,
    connectors,
    addConnector,
    connectorDetail,
    plugins,
    addPlugin,
    pluginDetail,
    settings,
  ]),
]);

export function makeRouter(queryClient: QueryClient) {
  return createRouter({ routeTree: tree, context: { queryClient }, defaultPreload: "intent" });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof makeRouter>;
  }
}

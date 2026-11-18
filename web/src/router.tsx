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
import { AddEvaluation } from "./features/customize/AddEvaluation";
import { EvaluationDetail } from "./features/customize/EvaluationDetail";
import { Evaluations } from "./features/customize/Evaluations";
import { Experiment } from "./features/experiment/Experiment";
import { ExperimentDetail } from "./features/experiment/ExperimentDetail";
import type { Panel } from "./features/experiment/Header";
import { Receipt } from "./features/experiment/Receipt";
import { Home } from "./features/home/Home";
import { Project } from "./features/projects/Project";
import { Projects } from "./features/projects/Projects";
import { Report } from "./features/report/Report";
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

function Scroll({ children }: { children: React.ReactNode }) {
  return <div className="scroll">{children}</div>;
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
      <Scroll>
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

const report = createRoute({
  getParentRoute: () => app,
  path: "/report",
  validateSearch: (search: Record<string, unknown>): { month?: string; project?: string } => {
    const month = typeof search.month === "string" && /^\d{4}-(0[1-9]|1[0-2])$/.test(search.month);
    const project = str(search.project);
    return {
      ...(month ? { month: search.month as string } : {}),
      ...(project === undefined ? {} : { project }),
    };
  },
  component: function ReportPage() {
    const { month, project: chosen } = report.useSearch();
    return (
      <Scroll>
        <Report month={month} project={chosen} />
      </Scroll>
    );
  },
});

const PANELS: readonly Panel[] = ["details", "trials", "changes", "terminal", "preview", "files"];

const experiment = createRoute({
  getParentRoute: () => app,
  path: "/experiments/$experimentId",
  validateSearch: (search: Record<string, unknown>): { session?: string; panel?: Panel } => {
    const session = str(search.session);
    const panel = PANELS.find((candidate) => candidate === search.panel);
    return {
      ...(session === undefined ? {} : { session }),
      ...(panel === undefined ? {} : { panel }),
    };
  },
  component: function ExperimentPage() {
    const { experimentId } = experiment.useParams();
    const { session, panel } = experiment.useSearch();
    const navigate = experiment.useNavigate();
    return (
      <Experiment
        key={experimentId}
        experimentId={experimentId}
        sessionId={session}
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

const experimentDetail = createRoute({
  getParentRoute: () => app,
  path: "/experiments/$experimentId/details",
  component: function DetailPage() {
    const { experimentId } = experimentDetail.useParams();
    return (
      <Scroll>
        <ExperimentDetail experimentId={experimentId} />
      </Scroll>
    );
  },
});

const receipt = createRoute({
  getParentRoute: () => app,
  path: "/receipts/$trialId",
  component: function ReceiptPage() {
    const { trialId } = receipt.useParams();
    return (
      <Scroll>
        <Receipt trialId={trialId} />
      </Scroll>
    );
  },
});

const customize = createRoute({
  getParentRoute: () => app,
  path: "/customize",
  component: () => <Navigate to="/customize/evaluations" replace />,
});
const evaluations = createRoute({
  getParentRoute: () => app,
  path: "/customize/evaluations",
  component: () => (
    <Scroll>
      <Evaluations />
    </Scroll>
  ),
});
const addEvaluation = createRoute({
  getParentRoute: () => app,
  path: "/customize/evaluations/new",
  component: () => (
    <Scroll>
      <AddEvaluation />
    </Scroll>
  ),
});
const evaluationDetail = createRoute({
  getParentRoute: () => app,
  path: "/customize/evaluations/$id",
  component: function EvaluationPage() {
    const { id } = evaluationDetail.useParams();
    return (
      <Scroll>
        <EvaluationDetail id={id} />
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
        <Scroll>
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
  app.addChildren([
    home,
    projects,
    project,
    report,
    experiment,
    experimentDetail,
    receipt,
    customize,
    evaluations,
    addEvaluation,
    evaluationDetail,
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

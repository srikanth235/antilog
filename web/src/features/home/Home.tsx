import { useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import {
  useAgents,
  useEnvironments,
  useMember,
  useModels,
  useProjects,
  useStartSession,
} from "../../api/queries";
import { Glyph, Icon } from "../../components/icon/Icon";
import { Select } from "../../components/select/Select";
import { reason } from "../../components/state/State";
import { canEdit, isOwner, parseRepos } from "../../lib/domain";
import { defaultEffort } from "../../lib/models";
import {
  rememberModel,
  setLastProject,
  useLastEffort,
  useLastModel,
  useLastProject,
  useRecentModels,
} from "../../lib/prefs";
import { w } from "../../words";
import { ModelPicker } from "../experiment/ModelPicker";
import { NewProject } from "../projects/NewProject";
import { useSidebarData } from "../shell/useSidebarData";
import { Waiting } from "./Waiting";

/** Home starts a session: the goal, the project it belongs to, and the model; the session page follows. */
export function Home({ project: preferred }: { project: string | undefined }) {
  const projects = useProjects();
  const me = useMember();
  const navigate = useNavigate();
  const data = useSidebarData();
  const lastProject = useLastProject();
  const lastModel = useLastModel();
  const lastEffort = useLastEffort();
  const recent = useRecentModels();
  const models = useModels();
  const [text, setText] = useState("");
  const [repos, setRepos] = useState("");
  const [chosen, setChosen] = useState<string | null>(null);
  const [picked, setPicked] = useState<{ model: string; effort: string | null } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const agents = useAgents();
  const environments = useEnvironments();
  const [agent, setAgent] = useState("auto");
  const [environment, setEnvironment] = useState<string | null>(null);
  // An archived environment is not offered to a new session.
  const offered = (environments.data ?? []).filter((item) => !item.archived);

  const list = projects.data ?? [];
  const known = (id: string | null | undefined) => list.some((p) => p.id === id);
  const selectedId =
    chosen ??
    (known(preferred)
      ? (preferred ?? null)
      : known(lastProject)
        ? lastProject
        : (list[0]?.id ?? null));
  const project = list.find((p) => p.id === selectedId);
  // The pick, else the last one if the keys still reach it, else the first model listed.
  const reachable = models.data ?? [];
  const remembered = reachable.find((item) => item.id === lastModel);
  const fallback = remembered ?? reachable[0];
  const model = picked?.model ?? fallback?.id ?? "";
  const effort =
    picked !== null
      ? picked.effort
      : remembered !== undefined
        ? lastEffort !== null && remembered.reasoningLevels.includes(lastEffort)
          ? lastEffort
          : defaultEffort(remembered)
        : defaultEffort(fallback);
  const allowed = canEdit(me?.role);
  const start = useStartSession(project?.id ?? "");

  const send = async (event: FormEvent) => {
    event.preventDefault();
    if (project === undefined || text.trim() === "" || model === "") return;
    setError(null);
    try {
      const session = await start.mutateAsync({
        agent,
        model,
        ...(effort === null ? {} : { effort }),
        prompt: text.trim(),
        repos: parseRepos(repos),
        ...(environment === null ? {} : { environment }),
      });
      setLastProject(project.id);
      rememberModel(model, effort);
      await navigate({ to: "/sessions/$sessionId", params: { sessionId: session.id } });
    } catch (caught) {
      setError(reason(caught));
    }
  };

  if (projects.isSuccess && list.length === 0)
    return (
      <div className="hw hw-c">
        <h1 className="t-title hw-t">{w.home.title}</h1>
        <p className="ink2 sm">{w.home.needProject}</p>
        {isOwner(me?.role) ? (
          <button type="button" className="btn primary" onClick={() => setCreating(true)}>
            <Icon name="folder" />
            {w.home.createProject}
          </button>
        ) : me === undefined ? null : (
          <p className="target-note">{w.home.needProjectViewer}</p>
        )}
        <NewProject open={creating} onClose={() => setCreating(false)} onCreated={setChosen} />
      </div>
    );

  return (
    <div className="hw hw-c">
      <h1 className="t-title hw-t">{w.home.title}</h1>
      <form className="cmpw" onSubmit={(event) => void send(event)}>
        <div className="cmp big">
          <textarea
            className="cmp-in"
            rows={3}
            aria-label={w.home.composerLabel}
            placeholder={w.home.placeholder}
            value={text}
            disabled={!allowed || project === undefined}
            onChange={(event) => setText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.metaKey || event.ctrlKey))
                event.currentTarget.form?.requestSubmit();
            }}
          />
          <div className="cmp-bar">
            <button
              type="button"
              className="ib"
              aria-label={w.home.attach}
              disabled
              title={w.experiment.notYet}
            >
              <Icon name="plus" />
            </button>
            <span className="go">
              <button
                type="submit"
                className="ib send"
                aria-label={w.home.send}
                disabled={
                  start.isPending || text.trim() === "" || project === undefined || model === ""
                }
              >
                <Icon name="return" />
              </button>
            </span>
          </div>
        </div>
        <div className="cmp-opt">
          <Select
            variant="chip"
            label={w.home.project}
            value={selectedId}
            options={list.map((p) => ({ value: p.id, label: p.name }))}
            onChange={setChosen}
          />
          <label className="opt" title={w.home.reposHint}>
            <Glyph name="git" />
            <input
              className="sr-input"
              aria-label={w.home.repos}
              placeholder={w.home.reposPlaceholder}
              value={repos}
              onChange={(event) => setRepos(event.target.value)}
            />
          </label>
          <Select
            variant="chip"
            label={w.home.agent}
            value={agent}
            options={[
              { value: "auto", label: w.home.auto },
              ...(agents.data ?? [])
                .filter((item) => item.enabled)
                .map((item) => ({ value: item.id, label: item.name })),
            ]}
            onChange={setAgent}
          />
          <Select
            variant="chip"
            label={w.home.environment}
            value={environment ?? offered[0]?.id ?? null}
            options={offered.map((item) => ({
              value: item.id,
              label: item.name,
            }))}
            onChange={setEnvironment}
          />
          <ModelPicker
            models={reachable}
            model={model === "" ? null : model}
            effort={effort}
            recent={recent}
            disabled={!allowed}
            onChange={(next, level) => setPicked({ model: next, effort: level })}
          />
        </div>
        {error === null ? null : (
          <p className="error-line" role="alert">
            {error}
          </p>
        )}
        {!allowed && me !== undefined ? (
          <p className="target-note">{w.readOnly.viewersRead}</p>
        ) : null}
      </form>
      {data.all.length === 0 && !data.loading ? (
        <p className="ink2 sm">{w.home.noSessions}</p>
      ) : null}
      <Waiting data={data} />
    </div>
  );
}

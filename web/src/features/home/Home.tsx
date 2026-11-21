import { useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import {
  useEnvironments,
  useMember,
  useModels,
  useProjects,
  useSessions,
  useStartSession,
} from "../../api/queries";
import { Mark } from "../../components/brand/Brand";
import { Glyph, Icon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { canEdit, isOwner } from "../../lib/domain";
import { defaultEffort } from "../../lib/models";
import {
  rememberModel,
  setLastProject,
  useLastEffort,
  useLastModel,
  useLastProject,
  useRecentModels,
} from "../../lib/prefs";
import { recentRepos, reposLabel, validBranch } from "../../lib/repos";
import { w } from "../../words";
import { ModelPicker } from "../experiment/ModelPicker";
import { NewProject } from "../projects/NewProject";
import { BranchPicker, ListPicker, RepoPicker } from "./Pickers";

/** The first word of a name, for the greeting. */
function firstName(name: string | undefined): string | null {
  const first = name?.trim().split(/\s+/)[0];
  return first === undefined || first === "" ? null : first;
}

/**
 * The new session page: a greeting, three starting points and the composer with its context
 * pickers. Sending starts a session in the chosen project and opens it.
 */
export function Home({ project: preferred }: { project: string | undefined }) {
  const projects = useProjects();
  const me = useMember();
  const navigate = useNavigate();
  const lastProject = useLastProject();
  const lastModel = useLastModel();
  const lastEffort = useLastEffort();
  const recent = useRecentModels();
  const models = useModels();
  const sessions = useSessions();
  const environments = useEnvironments();
  const [text, setText] = useState("");
  const [repos, setRepos] = useState<string[]>([]);
  const [branch, setBranch] = useState("");
  const [chosen, setChosen] = useState<string | null>(null);
  const [environment, setEnvironment] = useState<string | null>(null);
  const [followPolicy, setFollowPolicy] = useState(false);
  const [picked, setPicked] = useState<{ model: string; effort: string | null } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
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
  const known_repos = recentRepos(sessions.data ?? [], environments.data ?? []);
  const branchOk = branch === "" || validBranch(branch);
  const ready =
    !start.isPending && text.trim() !== "" && project !== undefined && model !== "" && branchOk;

  const send = async (event: FormEvent) => {
    event.preventDefault();
    if (project === undefined || !ready) return;
    setError(null);
    try {
      const session = await start.mutateAsync({
        model,
        ...(effort === null ? {} : { effort }),
        prompt: text.trim(),
        repos,
        ...(branch === "" || repos.length === 0 ? {} : { branch }),
        ...(environment === null ? {} : { environment }),
        followPolicy,
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
        <h1 className="t-title hw-t">{w.home.greeting(firstName(me?.name))}</h1>
        <p className="ink2 sm">{w.home.needProject}</p>
        {isOwner(me?.role) ? (
          <button type="button" className="btn primary" onClick={() => setCreating(true)}>
            <Icon name="folder" />
            {w.home.createProject}
          </button>
        ) : me === undefined ? null : (
          <p className="target-note">{w.readOnly.viewersRead}</p>
        )}
        <NewProject open={creating} onClose={() => setCreating(false)} onCreated={setChosen} />
      </div>
    );

  const mode = [
    { value: "ask", label: w.home.modeAsk },
    { value: "follow", label: w.home.modeFollow },
  ] as const;

  return (
    <div className="nh">
      <div className="nh-c">
        <span className="nh-mark">
          <Mark width={112} />
        </span>
        <h1 className="nh-t">{w.home.greeting(firstName(me?.name))}</h1>
      </div>
      <form className="nh-b" onSubmit={(event) => void send(event)}>
        <fieldset className="nh-s" aria-label={w.home.suggestions}>
          <button type="button" onClick={() => setText(w.home.suggestFix)}>
            <Glyph name="term" />
            {w.home.suggestFix}
          </button>
          <button type="button" onClick={() => setText(w.home.suggestFeature)}>
            <Glyph name="code" />
            {w.home.suggestFeature}
          </button>
          <button type="button" onClick={() => setText(w.home.suggestSetup)}>
            <Icon name="dl" />
            {w.home.suggestSetup}
          </button>
        </fieldset>
        <div className="cmp nh-cmp">
          <fieldset className="pkrow" aria-label={w.home.context}>
            <RepoPicker
              repos={known_repos}
              chosen={repos}
              disabled={!allowed}
              label={reposLabel(repos, { none: w.home.noRepository, more: w.home.reposMore })}
              onChange={setRepos}
            />
            <BranchPicker branch={branch} disabled={!allowed} onChange={setBranch} />
            <ListPicker
              label={w.home.project}
              lead={{ glyph: "blocks" }}
              value={selectedId}
              options={list.map((p) => ({ value: p.id, label: p.name }))}
              disabled={!allowed}
              onChange={setChosen}
            />
            <ListPicker
              label={w.home.environment}
              lead={{ glyph: "box" }}
              value={environment ?? offered[0]?.id ?? null}
              options={offered.map((item) => ({ value: item.id, label: item.name }))}
              disabled={!allowed}
              onChange={setEnvironment}
            />
          </fieldset>
          <textarea
            className="cmp-in"
            rows={2}
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
              aria-label={w.home.attachFiles}
              disabled
              title={w.experiment.notYet}
            >
              <Icon name="plus" />
            </button>
            <ModelPicker
              models={reachable}
              model={model === "" ? null : model}
              effort={effort}
              recent={recent}
              disabled={!allowed}
              onChange={(next, level) => setPicked({ model: next, effort: level })}
            />
            <span className="go">
              <ListPicker
                variant="opt"
                label={w.home.mode}
                lead={{ icon: "verify" }}
                value={followPolicy ? "follow" : "ask"}
                options={mode}
                disabled={!allowed}
                onChange={(value) => setFollowPolicy(value === "follow")}
              />
              <button
                type="submit"
                className={`ib send${ready ? " on" : ""}`}
                aria-label={w.home.send}
                disabled={!ready}
              >
                <Icon name="return" />
              </button>
            </span>
          </div>
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
    </div>
  );
}

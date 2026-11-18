import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { api, unwrap } from "../../api/client";
import {
  keys,
  useAgents,
  useAutoAgent,
  useEnvironments,
  useMember,
  useOrganization,
  useProjects,
} from "../../api/queries";
import { Glyph, Icon } from "../../components/icon/Icon";
import { Select } from "../../components/select/Select";
import { reason } from "../../components/state/State";
import { canEdit, defaultBudgetCents, parseRepos } from "../../lib/domain";
import { parseDollars } from "../../lib/format";
import { w } from "../../words";
import { NewProject } from "../projects/NewProject";
import { useSidebarData } from "../shell/useSidebarData";
import { Waiting } from "./Waiting";

/** Home is the Session page type with no conversation yet: the composer is the page. */
export function Home({ project: preferred }: { project: string | undefined }) {
  const projects = useProjects();
  const organization = useOrganization();
  const me = useMember();
  const navigate = useNavigate();
  const client = useQueryClient();
  const data = useSidebarData();
  const [text, setText] = useState("");
  const [repos, setRepos] = useState("");
  const [chosen, setChosen] = useState<string | null>(null);
  const [follow, setFollow] = useState<"ask" | "follow">("ask");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [amount, setAmount] = useState("");
  const [creating, setCreating] = useState(false);
  const agents = useAgents();
  const auto = useAutoAgent();
  const environments = useEnvironments();
  const [agent, setAgent] = useState("auto");
  const [environment, setEnvironment] = useState<string | null>(null);

  const list = projects.data ?? [];
  const selectedId =
    chosen ?? (list.some((p) => p.id === preferred) ? (preferred ?? null) : (list[0]?.id ?? null));
  const project = list.find((p) => p.id === selectedId);
  const fallback = defaultBudgetCents(project, organization.data);
  // What the person sees is what is sent: their own amount, else the default the server will apply.
  const typed = amount.trim() === "" ? null : parseDollars(amount);
  const budget = typed ?? fallback;
  const needsAmount = budget === null || (amount.trim() !== "" && typed === null);
  const allowed = canEdit(me?.role);

  const send = async (event: FormEvent) => {
    event.preventDefault();
    if (project === undefined || text.trim() === "" || needsAmount) return;
    setBusy(true);
    setError(null);
    try {
      const experiment = unwrap(
        await api.POST("/projects/{id}/experiments", {
          params: { path: { id: project.id } },
          body: {
            goal: text.trim(),
            ...(typed === null ? {} : { budgetCapCents: typed }),
            followPolicy: follow === "follow",
          },
        }),
      );
      // "auto" names the agent choice; the server resolves it and refuses when it cannot choose.
      unwrap(
        await api.POST("/experiments/{id}/sessions", {
          params: { path: { id: experiment.id } },
          body: {
            agent,
            prompt: text.trim(),
            repos: parseRepos(repos),
            ...(environment === null ? {} : { environment }),
          },
        }),
      );
      await client.invalidateQueries({ queryKey: keys.projectExperiments(project.id) });
      await client.invalidateQueries({ queryKey: keys.sessions(experiment.id) });
      await navigate({ to: "/experiments/$experimentId", params: { experimentId: experiment.id } });
    } catch (caught) {
      setError(reason(caught));
    } finally {
      setBusy(false);
    }
  };

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
                disabled={busy || text.trim() === "" || project === undefined || needsAmount}
              >
                <Icon name="return" />
              </button>
            </span>
          </div>
        </div>
        <div className="cmp-opt">
          {list.length > 0 ? (
            <Select
              variant="chip"
              label={w.home.project}
              value={selectedId}
              options={list.map((p) => ({ value: p.id, label: p.name }))}
              onChange={setChosen}
            />
          ) : (
            <button type="button" className="opt" onClick={() => setCreating(true)}>
              <Icon name="folder" />
              <span>{w.home.createProject}</span>
            </button>
          )}
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
            title={
              agent !== "auto"
                ? undefined
                : auto.data !== undefined
                  ? w.home.autoWould(auto.data.agent.name, auto.data.reason)
                  : auto.isError
                    ? reason(auto.error)
                    : w.home.autoHint
            }
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
            value={environment ?? environments.data?.[0]?.id ?? null}
            options={(environments.data ?? []).map((item) => ({
              value: item.id,
              label: item.name,
            }))}
            onChange={setEnvironment}
          />
          <Select
            variant="chip"
            label={w.home.mode}
            value={follow}
            options={[
              { value: "ask", label: w.home.modeAsk },
              { value: "follow", label: w.home.modeFollow },
            ]}
            onChange={setFollow}
          />
          <label className="opt" title={fallback === null ? w.home.noBudgetHint : undefined}>
            <span className="ring" style={{ ["--p" as string]: 0 }} />
            <span className="sr-only">{w.home.budgetAmount}</span>
            <input
              className="sr-input"
              inputMode="decimal"
              aria-label={w.home.budgetAmount}
              placeholder={fallback === null ? w.home.noBudget : w.home.budgetOf(0, fallback)}
              value={amount}
              onChange={(event) => setAmount(event.target.value)}
            />
          </label>
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
      {list.length === 0 && projects.isSuccess ? (
        <p className="ink2 sm">{w.home.needProject}</p>
      ) : null}
      {data.all.length === 0 && !data.loading && list.length > 0 ? (
        <p className="ink2 sm">{w.home.noExperiments}</p>
      ) : null}
      <Waiting data={data} />
      <NewProject open={creating} onClose={() => setCreating(false)} />
    </div>
  );
}

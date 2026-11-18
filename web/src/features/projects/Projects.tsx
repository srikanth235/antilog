import { useQueries } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { api, unwrap } from "../../api/client";
import { keys, useBudgets, useMember, useProjects } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading } from "../../components/state/State";
import { canEdit } from "../../lib/domain";
import { dollars, plural } from "../../lib/format";
import { w } from "../../words";
import { NewProject } from "./NewProject";

export function Projects() {
  const projects = useProjects();
  const budgets = useBudgets();
  const me = useMember();
  const [query, setQuery] = useState("");
  const [creating, setCreating] = useState(false);
  const list = projects.data ?? [];
  const counts = useQueries({
    queries: list.map((project) => ({
      queryKey: keys.projectExperiments(project.id),
      queryFn: async () =>
        unwrap(
          await api.GET("/projects/{id}/experiments", { params: { path: { id: project.id } } }),
        ),
    })),
  });
  const roles = useQueries({
    queries: list.map((project) => ({
      queryKey: keys.projectRoles(project.id),
      queryFn: async () =>
        unwrap(await api.GET("/projects/{id}/roles", { params: { path: { id: project.id } } })),
    })),
  });
  const values = useQueries({
    queries: list.map((project) => ({
      queryKey: keys.projectValue(project.id),
      queryFn: async () =>
        unwrap(await api.GET("/projects/{id}/value", { params: { path: { id: project.id } } })),
    })),
  });
  const visible = list.filter((project) =>
    project.name.toLowerCase().includes(query.toLowerCase()),
  );

  return (
    <section className="page ct ls">
      <div className="ct-head">
        <div className="ct-ttl">
          <div className="rw">
            <h1 className="t-title">{w.projects.title}</h1>
            <span className="tag">{list.length}</span>
          </div>
          <span className="ink2">{w.projects.lead}</span>
        </div>
        <div className="ct-ctl">
          <label className="sfield">
            <Icon name="search" />
            <input
              placeholder={w.projects.search}
              aria-label={w.projects.search}
              autoComplete="off"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
          <Link className="btn" to="/report">
            {w.report.open}
          </Link>
          <button
            type="button"
            className="btn primary"
            onClick={() => setCreating(true)}
            disabled={!canEdit(me?.role)}
          >
            <Icon name="plus" size="s" />
            {w.projects.newProject}
          </button>
        </div>
      </div>
      {budgets.data === undefined ? null : (
        <div className="vs c4">
          <div>
            <span className="k">{w.settings.budgets.spent}</span>
            <span className="v">{dollars(budgets.data.organization.spentCents)}</span>
            <span className="s">
              {budgets.data.organization.capCents == null
                ? w.settings.budgets.noCap
                : `of ${dollars(budgets.data.organization.capCents)}`}
            </span>
          </div>
        </div>
      )}
      <div style={{ marginTop: 24 }} />
      {projects.isPending ? (
        <Loading />
      ) : projects.isError ? (
        <Failed error={projects.error} retry={() => void projects.refetch()} />
      ) : list.length === 0 ? (
        <Empty title={w.projects.empty} hint={w.projects.emptyHint}>
          <button type="button" className="btn primary" onClick={() => setCreating(true)}>
            <Icon name="plus" size="s" />
            {w.projects.newProject}
          </button>
        </Empty>
      ) : (
        <div className="tb tb-pj">
          <div className="tb-r th">
            <span>{w.projects.columns.project}</span>
            <span>{w.projects.columns.role}</span>
            <span>{w.projects.columns.experiments}</span>
            <span>{w.projects.columns.wins}</span>
            <span>{w.projects.columns.spend}</span>
            <span />
          </div>
          {visible.map((project) => {
            const index = list.indexOf(project);
            const own = roles[index]?.data?.find((role) => role.memberId === me?.id)?.role;
            const role = own ?? me?.role;
            const spend = budgets.data?.projects.find(
              (entry) => entry.projectId === project.id,
            )?.spentCents;
            return (
              <Link
                key={project.id}
                className="tb-r"
                to="/projects/$projectId"
                params={{ projectId: project.id }}
              >
                <span className="n">
                  <span>{project.name}</span>
                  <span className="m">{project.description}</span>
                </span>
                <span>
                  {role === undefined ? null : <span className="tag">{w.roles[role]}</span>}
                </span>
                <span className="mono">{counts[index]?.data?.length ?? ""}</span>
                <span className="mono">
                  {values[index]?.data === undefined
                    ? ""
                    : w.projects.winsOf(
                        values[index].data.mergedWins,
                        values[index].data.pendingWins,
                      )}
                </span>
                <span className="mono">{spend === undefined ? "" : dollars(spend)}</span>
                <Icon name="chev" size="s" />
              </Link>
            );
          })}
        </div>
      )}
      <NewProject open={creating} onClose={() => setCreating(false)} />
    </section>
  );
}

export function experimentCount(n: number): string {
  return plural(n, "experiment");
}

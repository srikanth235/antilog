import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useBudgets, useMember, useProjects, useSessions } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading } from "../../components/state/State";
import { isOwner } from "../../lib/domain";
import { dollars, plural } from "../../lib/format";
import { w } from "../../words";
import { Row, Table } from "../settings/Table";
import { NewProject } from "./NewProject";

export function Projects() {
  const projects = useProjects();
  const sessions = useSessions();
  const budgets = useBudgets();
  const me = useMember();
  const [creating, setCreating] = useState(false);
  const navigate = useNavigate();
  const p = w.projects;
  const cols = "minmax(0, 1.6fr) 140px 160px";
  const spent = new Map((budgets.data?.projects ?? []).map((x) => [x.projectId, x.spentCents]));
  return (
    <section className="page">
      <div className="hd">
        <h1 className="t-display">{p.title}</h1>
        {isOwner(me?.role) ? (
          <button type="button" className="btn primary sm" onClick={() => setCreating(true)}>
            <Icon name="plus" size="s" />
            {p.newProject}
          </button>
        ) : null}
      </div>
      <p className="sd-lead">{p.lead}</p>
      {projects.isPending ? (
        <Loading rows={2} />
      ) : projects.isError ? (
        <Failed error={projects.error} retry={() => void projects.refetch()} />
      ) : projects.data.length === 0 ? (
        <Empty title={p.empty} hint={p.emptyHint} />
      ) : (
        <Table
          columns={cols}
          head={[p.columns.project, p.sessionsTitle, p.columns.spend]}
          label={p.title}
        >
          {projects.data.map((project) => (
            <Row key={project.id} columns={cols}>
              <Link to="/projects/$projectId" params={{ projectId: project.id }}>
                {project.name}
              </Link>
              <span>
                {plural(
                  (sessions.data ?? []).filter((s) => s.projectId === project.id).length,
                  "session",
                )}
              </span>
              <span className="mono">{dollars(spent.get(project.id) ?? 0)}</span>
            </Row>
          ))}
        </Table>
      )}
      <NewProject
        open={creating}
        onClose={() => setCreating(false)}
        onCreated={(projectId) =>
          void navigate({ to: "/projects/$projectId", params: { projectId } })
        }
      />
    </section>
  );
}

import { Link } from "@tanstack/react-router";
import { useState } from "react";
import type { Schemas } from "../../api/client";
import {
  useBudgets,
  useMember,
  useMembers,
  useProjectRoles,
  useProjectSessions,
  useProjects,
  useSetProjectLimits,
  useSetProjectRole,
} from "../../api/queries";
import { Icon, StatusIcon } from "../../components/icon/Icon";
import { ROLE_OPTIONS } from "../../components/select/roles";
import { Select } from "../../components/select/Select";
import { Empty, Failed, Loading, reason } from "../../components/state/State";
import { isOwner } from "../../lib/domain";
import { ago, dollars, parseDollars } from "../../lib/format";
import { w } from "../../words";
import { ReadOnly, Row, Table } from "../settings/Table";

const p = w.projects;

export function Project({ projectId }: { projectId: string }) {
  const projects = useProjects();
  if (projects.isPending) return <Loading />;
  if (projects.isError)
    return <Failed error={projects.error} retry={() => void projects.refetch()} />;
  const project = projects.data.find((item) => item.id === projectId);
  if (project === undefined) return <Empty title={w.noData} />;
  return (
    <section className="page">
      <Link className="textbtn" to="/projects" style={{ alignSelf: "flex-start" }}>
        <Icon name="back" size="s" />
        {p.back}
      </Link>
      <div className="hd">
        <h1 className="t-display">{project.name}</h1>
      </div>
      {project.description === "" ? null : <p className="sd-lead">{project.description}</p>}
      <Sessions projectId={project.id} />
      <Limits project={project} />
      <Roles projectId={project.id} />
    </section>
  );
}

function Sessions({ projectId }: { projectId: string }) {
  const sessions = useProjectSessions(projectId);
  const cols = "minmax(0, 1.8fr) 140px 140px";
  const now = Date.now();
  return (
    <div className="sd-sec">
      <h3 className="t-section">{p.sessionsTitle}</h3>
      {sessions.isPending ? (
        <Loading rows={2} />
      ) : sessions.isError ? (
        <Failed error={sessions.error} retry={() => void sessions.refetch()} />
      ) : sessions.data.length === 0 ? (
        <Empty title={p.noSessionsHere} hint={p.sessionsHint} />
      ) : (
        <Table
          columns={cols}
          head={[p.sessionColumns.session, p.sessionColumns.status, p.sessionColumns.updated]}
          label={p.sessionsTitle}
        >
          {sessions.data.map((session) => (
            <Row key={session.id} columns={cols}>
              <Link className="ell" to="/sessions/$sessionId" params={{ sessionId: session.id }}>
                {session.title}
              </Link>
              <span className="rw g8">
                <StatusIcon activity={session.status} />
                {w.activity[session.status]}
              </span>
              <span className="xs mut">{ago(session.updatedAt, now)}</span>
            </Row>
          ))}
        </Table>
      )}
    </div>
  );
}

function Limits({ project }: { project: Schemas["Project"] }) {
  const me = useMember();
  const owner = isOwner(me?.role);
  const budgets = useBudgets();
  const set = useSetProjectLimits(project.id);
  const spent = budgets.data?.projects.find((x) => x.projectId === project.id)?.spentCents ?? 0;
  const [cap, setCap] = useState(
    project.monthlyCapCents == null ? "" : String(project.monthlyCapCents / 100),
  );
  const [def, setDef] = useState(
    project.defaultBudgetCents == null ? "" : String(project.defaultBudgetCents / 100),
  );
  const [saved, setSaved] = useState(false);
  const bad = (text: string) => text.trim() !== "" && parseDollars(text) === null;
  return (
    <div className="sd-sec">
      <h3 className="t-section">{p.limitsTitle}</h3>
      <p className="sd-lead">{p.limitsLead}</p>
      {owner ? null : <ReadOnly text={p.onlyOwners} />}
      <dl className="kvl">
        <div className="stg">
          <div className="tx">
            <span className="t-item">{p.spentThisMonth}</span>
          </div>
          <div className="ct">
            <span className="mono">{dollars(spent)}</span>
          </div>
        </div>
      </dl>
      <form
        className="form-col"
        onSubmit={(event) => {
          event.preventDefault();
          setSaved(false);
          set.mutate(
            {
              monthlyCapCents: cap.trim() === "" ? null : parseDollars(cap),
              defaultBudgetCents: def.trim() === "" ? null : parseDollars(def),
            },
            { onSuccess: () => setSaved(true) },
          );
        }}
      >
        <label className="field">
          {p.capLabel}
          <input
            className="input"
            inputMode="decimal"
            disabled={!owner}
            value={cap}
            onChange={(event) => setCap(event.target.value)}
          />
          <span className="xs mut">{p.clearCap}</span>
        </label>
        <label className="field">
          {p.defaultSession}
          <input
            className="input"
            inputMode="decimal"
            disabled={!owner}
            value={def}
            onChange={(event) => setDef(event.target.value)}
          />
        </label>
        {set.isError ? (
          <p className="error-line" role="alert">
            {reason(set.error)}
          </p>
        ) : null}
        {saved ? <span className="xs mut">{p.saved}</span> : null}
        <button
          type="submit"
          className="btn primary sm"
          style={{ alignSelf: "flex-start" }}
          disabled={!owner || set.isPending || bad(cap) || bad(def)}
        >
          {w.save}
        </button>
      </form>
    </div>
  );
}

function Roles({ projectId }: { projectId: string }) {
  const members = useMembers();
  const roles = useProjectRoles(projectId);
  const set = useSetProjectRole(projectId);
  const me = useMember();
  const owner = isOwner(me?.role);
  const cols = "minmax(0, 1.6fr) 160px";
  const own = new Map((roles.data ?? []).map((entry) => [entry.memberId, entry.role]));
  return (
    <div className="sd-sec">
      <h3 className="t-section">{p.members}</h3>
      <p className="sd-lead">{p.membersLead}</p>
      {members.isPending || roles.isPending ? (
        <Loading rows={2} />
      ) : members.isError ? (
        <Failed error={members.error} retry={() => void members.refetch()} />
      ) : (
        <Table
          columns={cols}
          head={[w.settings.members.name, w.settings.members.role]}
          label={p.members}
        >
          {members.data.map((member) => (
            <Row key={member.id} columns={cols}>
              <span className="ell">{member.name}</span>
              <Select
                label={`${w.settings.members.role}: ${member.name}`}
                value={own.get(member.id) ?? member.role}
                options={ROLE_OPTIONS}
                disabled={!owner}
                onChange={(next) => set.mutate({ member: member.id, role: next })}
              />
            </Row>
          ))}
        </Table>
      )}
      {set.isError ? (
        <p className="error-line" role="alert">
          {reason(set.error)}
        </p>
      ) : null}
    </div>
  );
}

import { useQueries } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { api, unwrap } from "../../api/client";
import {
  keys,
  useBudgets,
  useMembers,
  useOrganization,
  useProjectExperiments,
  useProjectRoles,
  useProjects,
  useSetProjectRole,
} from "../../api/queries";
import { Icon, StatusIcon } from "../../components/icon/Icon";
import { ROLE_OPTIONS } from "../../components/select/roles";
import { Select } from "../../components/select/Select";
import { Empty, Failed, Loading } from "../../components/state/State";
import { activityOf, defaultBudgetCents, isOwner, pendingCards } from "../../lib/domain";
import { dollars, initial, plural, share } from "../../lib/format";
import { useIdentity } from "../../lib/identity";
import { w } from "../../words";
import { ProjectBudget } from "./ProjectBudget";

export function Project({ projectId }: { projectId: string }) {
  const projects = useProjects();
  const experiments = useProjectExperiments(projectId);
  const roles = useProjectRoles(projectId);
  const members = useMembers();
  const organization = useOrganization();
  const budgets = useBudgets();
  const identity = useIdentity();
  const setRole = useSetProjectRole(projectId);
  const [budgetOpen, setBudgetOpen] = useState(false);
  const project = projects.data?.find((item) => item.id === projectId);
  const list = experiments.data ?? [];
  const sessions = useQueries({
    queries: list.map((experiment) => ({
      queryKey: keys.sessions(experiment.id),
      queryFn: async () =>
        unwrap(
          await api.GET("/experiments/{id}/sessions", { params: { path: { id: experiment.id } } }),
        ),
    })),
  });
  const approvals = useQueries({
    queries: list.map((experiment) => ({
      queryKey: keys.approvals(experiment.id),
      queryFn: async () =>
        unwrap(
          await api.GET("/experiments/{id}/approvals", { params: { path: { id: experiment.id } } }),
        ),
    })),
  });

  if (projects.isPending)
    return (
      <section className="page ct ls">
        <Loading />
      </section>
    );
  if (project === undefined)
    return (
      <section className="page ct ls">
        <Failed error={projects.error} />
      </section>
    );

  const me = members.data?.find((member) => member.id === identity?.memberId);
  const myProjectRole = roles.data?.find((role) => role.memberId === identity?.memberId)?.role;
  const role = myProjectRole ?? me?.role;
  const owner = isOwner(role);
  const spent =
    budgets.data?.projects.find((entry) => entry.projectId === projectId)?.spentCents ?? 0;
  const cap = project.monthlyCapCents ?? null;
  const defaultBudget = defaultBudgetCents(project, organization.data);
  const working = sessions.filter((query) =>
    query.data?.some((session) => session.status === "working"),
  ).length;

  return (
    <article className="page ct ls">
      <Link className="textbtn ct-back" to="/projects">
        <Icon name="back" size="s" />
        {w.projects.back}
      </Link>
      <div className="ct-head">
        <div className="ct-ttl">
          <div className="rw">
            <h1 className="t-title">{project.name}</h1>
            {role === undefined ? null : <span className="tag">{w.roles[role]}</span>}
            <span className="tag">{w.projects.counts(list.length, 0)}</span>
          </div>
          <span className="ink2">{project.description}</span>
        </div>
        <div className="ct-ctl">
          <Link className="btn primary" to="/" search={{ project: projectId }}>
            <Icon name="plus" size="s" />
            {w.sidebar.newExperiment}
          </Link>
        </div>
      </div>
      <div className="vs c4">
        <div>
          <span className="k">{w.projects.experiments}</span>
          <span className="v">{list.length}</span>
          <span className="s">{working === 0 ? "" : w.projectStrip.working(working)}</span>
        </div>
        <div>
          <span className="k">{w.projects.spentThisMonth}</span>
          <span className="v">
            {dollars(spent)}
            {cap === null ? null : <small> of {dollars(cap)}</small>}
          </span>
          <span className="s">
            {cap === null
              ? w.settings.budgets.noCap
              : w.projectStrip.ofCap(Math.round(share(spent, cap) * 100))}
          </span>
        </div>
        <div>
          <span className="k">{w.projects.defaultPer}</span>
          <span className="v">
            {defaultBudget === null ? w.projects.noDefault : dollars(defaultBudget)}
          </span>
          <span className="s">
            {project.defaultBudgetCents == null
              ? w.projectStrip.orgDefault
              : w.projectStrip.projectDefault}
          </span>
        </div>
      </div>

      <section className="ds" aria-labelledby="h-experiments">
        <div className="ds-h">
          <h2 className="t-section" id="h-experiments">
            {w.projects.experiments}
          </h2>
        </div>
        {experiments.isPending ? (
          <Loading />
        ) : list.length === 0 ? (
          <Empty title={w.projects.noExperiments} hint={w.projects.noExperimentsHint}>
            <Link className="btn primary" to="/" search={{ project: projectId }}>
              <Icon name="plus" size="s" />
              {w.sidebar.newExperiment}
            </Link>
          </Empty>
        ) : (
          <div className="tb tb-px" style={{ marginTop: 12 }}>
            <div className="tb-r th">
              <span>{w.projects.columnsExp.experiment}</span>
              <span>{w.projects.columnsExp.best}</span>
              <span>{w.projects.columnsExp.state}</span>
              <span>{w.projects.columnsExp.activity}</span>
              <span>{w.projects.columnsExp.spent}</span>
              <span />
            </div>
            {list.map((experiment, index) => {
              const activity = activityOf(
                sessions[index]?.data ?? [],
                pendingCards(approvals[index]?.data ?? []).length,
              );
              return (
                <Link
                  key={experiment.id}
                  className="tb-r"
                  to="/experiments/$experimentId"
                  params={{ experimentId: experiment.id }}
                >
                  <span className="n">
                    <span>{experiment.goal}</span>
                  </span>
                  <span className="vv">
                    <span className="mono">
                      {/* (target) the best value needs the judge's history */ w.noData}
                    </span>
                  </span>
                  <span className="spill" style={{ ["--c" as string]: "var(--ink3)" }}>
                    {w.state[experiment.status]}
                  </span>
                  <span className="spill" style={{ ["--c" as string]: "var(--accent)" }}>
                    {activity === null ? null : (
                      <>
                        <StatusIcon activity={activity} />
                        {w.activity[activity]}
                      </>
                    )}
                  </span>
                  <span className="mono">{dollars(experiment.budgetCapCents)}</span>
                  <Icon name="chev" size="s" />
                </Link>
              );
            })}
          </div>
        )}
      </section>

      <section className="ds" aria-labelledby="h-ideas">
        <div className="ds-h">
          <h2 className="t-section" id="h-ideas">
            {w.projects.ideas}
          </h2>
        </div>
        <p className="ds-n">{w.projects.ideasLead}</p>
        <p className="target-note">{w.projects.ideasNone}</p>
      </section>

      <section className="ds" aria-labelledby="h-value">
        <div className="ds-h">
          <h2 className="t-section" id="h-value">
            {w.projects.value}
          </h2>
        </div>
        <p className="ds-n">{w.projects.valueLead}</p>
        <p className="target-note">{w.projects.valueNone}</p>
      </section>

      <section className="ds" aria-labelledby="h-connectors">
        <div className="ds-h">
          <h2 className="t-section" id="h-connectors">
            {w.projects.connectors}
          </h2>
        </div>
        <Empty title={w.projects.connectorsNone} />
      </section>

      <section className="ds" aria-labelledby="h-budget">
        <div className="ds-h">
          <h2 className="t-section" id="h-budget">
            {w.projects.budget}
          </h2>
          <button
            type="button"
            className="btn sm"
            disabled={!owner}
            title={owner ? undefined : w.readOnly.onlyOwners}
            onClick={() => setBudgetOpen(true)}
          >
            {w.projects.changeBudget}
          </button>
        </div>
        <p className="ds-n">{w.projects.budgetLead}</p>
        <div className="ds-b">
          <div className="dr">
            <span className="k">{w.projects.spentThisMonth}</span>
            <span className="v">
              <span>
                <span className="mono">{dollars(spent)}</span>
                {cap === null ? null : (
                  <>
                    {" "}
                    of <span className="mono">{dollars(cap)}</span> {w.projects.projectCap}
                  </>
                )}
              </span>
              {cap === null ? null : (
                <span className="bar" style={{ maxWidth: 360 }}>
                  <span style={{ width: `${Math.round(share(spent, cap) * 100)}%` }} />
                </span>
              )}
            </span>
          </div>
          <div className="dr">
            <span className="k">{w.projects.defaultPer}</span>
            <span className="v">
              <span className="mono">
                {defaultBudget === null ? w.projects.noDefault : dollars(defaultBudget)}
              </span>
            </span>
          </div>
        </div>
        {owner ? null : <p className="target-note">{w.readOnly.onlyOwners}</p>}
      </section>

      <section className="ds" aria-labelledby="h-members">
        <div className="ds-h">
          <h2 className="t-section" id="h-members">
            {w.projects.members}
          </h2>
        </div>
        <p className="ds-n">{w.projects.membersLead}</p>
        <div className="tb tb-pm" style={{ marginTop: 12 }}>
          {(members.data ?? []).map((member) => {
            const set = roles.data?.find((entry) => entry.memberId === member.id)?.role ?? null;
            const effective = set ?? member.role;
            return (
              <div key={member.id} className="tb-r">
                <span className="rw g12" style={{ minWidth: 0 }}>
                  <span className="avatar round" aria-hidden="true">
                    {initial(member.name)}
                  </span>
                  <span className="n">
                    <span style={{ fontSize: 13.5 }}>{member.name}</span>
                    <span className="m">
                      {set === null ? w.projects.inherited : w.projects.setForProject}
                    </span>
                  </span>
                </span>
                <Select
                  label={`${member.name}: ${w.settings.members.role}`}
                  value={effective}
                  options={ROLE_OPTIONS}
                  disabled={!owner}
                  onChange={(next) => setRole.mutate({ member: member.id, role: next })}
                />
              </div>
            );
          })}
        </div>
        <p className="target-note">{w.projects.keepsOwner}</p>
        {setRole.isError ? (
          <p className="error-line" role="alert">
            {setRole.error.message}
          </p>
        ) : null}
      </section>
      <ProjectBudget open={budgetOpen} onClose={() => setBudgetOpen(false)} project={project} />
      <span className="sr-only">{plural(list.length, "experiment")}</span>
    </article>
  );
}

import { Link } from "@tanstack/react-router";
import { useState } from "react";
import {
  useExperiment,
  useExperimentSessions,
  useHistory,
  useMembers,
  useProjects,
} from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading } from "../../components/state/State";
import { agentName, canEdit, platformName } from "../../lib/domain";
import { dateTime, dollars } from "../../lib/format";
import { kept, OUTCOME_WORDS, parseHistory } from "../../lib/history";
import { useIdentity } from "../../lib/identity";
import { w } from "../../words";
import { BudgetDialog } from "./BudgetDialog";

const t = w.experimentDetail;

function Section(props: { title: string; note?: string; children: React.ReactNode }) {
  return (
    <section className="ds" aria-label={props.title}>
      <div className="ds-h">
        <h2 className="t-section">{props.title}</h2>
      </div>
      {props.note === undefined ? null : <p className="ds-n">{props.note}</p>}
      {props.children}
    </section>
  );
}

function Target({ text }: { text: string }) {
  return <p className="target-note">{text}</p>;
}

/** The detail page: what the API serves today, and a marked stub for the rest. */
export function ExperimentDetail({ experimentId }: { experimentId: string }) {
  const experiment = useExperiment(experimentId);
  const sessions = useExperimentSessions(experimentId);
  const history = useHistory(experimentId);
  const projects = useProjects();
  const members = useMembers();
  const identity = useIdentity();
  const [budgetOpen, setBudgetOpen] = useState(false);
  if (experiment.isPending)
    return (
      <section className="page ct">
        <Loading />
      </section>
    );
  if (experiment.isError)
    return (
      <section className="page ct">
        <Failed error={experiment.error} retry={() => void experiment.refetch()} />
      </section>
    );
  const e = experiment.data;
  const parsed = parseHistory(history.data);
  const project = projects.data?.find((p) => p.id === e.projectId);
  const me = members.data?.find((m) => m.id === identity?.memberId);
  const list = sessions.data ?? [];
  const cols = "minmax(0, 1.4fr) minmax(0, 1.4fr) 150px 110px";
  return (
    <article className="page ct">
      <Link
        className="textbtn ct-back"
        to="/projects/$projectId"
        params={{ projectId: e.projectId }}
      >
        <Icon name="back" size="s" />
        {project?.name ?? w.projects.back}
      </Link>
      <div className="ct-head">
        <div className="ct-ttl">
          <div className="rw">
            <h1 className="t-title">{e.goal}</h1>
            <span className="tag">{w.state[e.status]}</span>
            {parsed.version === null ? null : <span className="tag">v{parsed.version}</span>}
          </div>
        </div>
        <div className="ct-ctl">
          <Link
            className="btn primary"
            to="/experiments/$experimentId"
            params={{ experimentId: e.id }}
          >
            {w.experiment.startSession}
          </Link>
        </div>
      </div>
      <div className="vs c4">
        <div>
          <span className="k">{t.trials}</span>
          <span className="v">{parsed.trials.length}</span>
          <span className="s">{kept(parsed)} kept</span>
        </div>
        <div>
          <span className="k">{t.budget}</span>
          <span className="v">{dollars(e.budgetCapCents)}</span>
          <span className="s">
            <button
              type="button"
              className="textbtn"
              disabled={!canEdit(me?.role)}
              onClick={() => setBudgetOpen(true)}
            >
              {w.projects.changeBudget}
            </button>
          </span>
        </div>
        <div>
          <span className="k">{t.created}</span>
          <span className="v">{dateTime(e.createdAt).split(",")[0]}</span>
        </div>
      </div>
      <Section title={t.definition} note={t.immutable}>
        <div className="ds-b">
          <div className="dr">
            <span className="k">{t.goal}</span>
            <span className="v">
              <span>{e.goal}</span>
            </span>
          </div>
          <div className="dr">
            <span className="k">{t.approvalMode}</span>
            <span className="v">
              <span>{e.followPolicy ? w.home.modeFollow : w.home.modeAsk}</span>
            </span>
          </div>
        </div>
        <Target text={t.definitionTarget} />
      </Section>
      <Section title={t.sealChecks}>
        <Target text={t.sealTarget} />
      </Section>
      <Section title={t.data}>
        <Target text={t.dataTarget} />
      </Section>
      <Section title={t.value}>
        <Target text={t.valueTarget} />
      </Section>
      <Section title={t.spend}>
        <Target text={t.spendTarget} />
      </Section>
      <Section title={t.sessions}>
        {list.length === 0 ? (
          <Empty title={w.experiment.noSessions} />
        ) : (
          <div className="tb" style={{ ["--cols" as string]: cols, marginTop: 12 }}>
            <div className="tb-r th">
              <span>{t.session}</span>
              <span>{t.platform}</span>
              <span>{t.started}</span>
              <span>{t.state}</span>
            </div>
            {list.map((s) => (
              <Link
                key={s.id}
                className="tb-r"
                to="/experiments/$experimentId"
                params={{ experimentId: e.id }}
                search={{ session: s.id }}
              >
                <span className="n">
                  <span>{w.sidebar.session(agentName(s.agent))}</span>
                </span>
                <span>{platformName(s.platform)}</span>
                <span className="ink2">{dateTime(s.createdAt)}</span>
                <span>
                  {w.activity[s.status]}
                  {s.cleanup === "NotRemoved" ? ` · ${t.notRemoved}` : ""}
                </span>
              </Link>
            ))}
          </div>
        )}
      </Section>
      <Section title={t.trials}>
        {parsed.trials.length === 0 ? (
          <Empty title={parsed.note ?? w.experiment.trialsPanelNone} />
        ) : (
          <div
            className="tb"
            style={{ ["--cols" as string]: "56px minmax(0, 2fr) 140px 120px", marginTop: 12 }}
          >
            <div className="tb-r th">
              <span>{t.trial}</span>
              <span>{t.hypothesis}</span>
              <span>{t.outcome}</span>
              <span>{t.value}</span>
            </div>
            {[...parsed.trials].reverse().map((row) => (
              <div key={row.trial} className="tb-r">
                <span className="mono">{row.number}</span>
                <span>{row.hypothesis}</span>
                <span>{OUTCOME_WORDS[row.outcome]}</span>
                <span className="mono">
                  {row.value === null ? "" : `${row.value} ${parsed.unit}`.trim()}
                </span>
              </div>
            ))}
          </div>
        )}
      </Section>
      <Section title={t.wins}>
        {parsed.best === null ? (
          <Empty title={t.noWins} />
        ) : (
          <Link
            className="btn"
            to="/receipts/$trialId"
            params={{ trialId: parsed.best }}
            style={{ alignSelf: "flex-start", marginTop: 12 }}
          >
            {t.viewReceipt}
          </Link>
        )}
      </Section>
      <Section title={t.watch}>
        <Target text={t.watchTarget} />
      </Section>
      <Section title={t.versions} note={t.versionsNote}>
        {parsed.version === null ? null : (
          <p className="ds-n">
            {t.current}: v{parsed.version}
          </p>
        )}
      </Section>
      <BudgetDialog open={budgetOpen} onClose={() => setBudgetOpen(false)} experiment={e} />
    </article>
  );
}

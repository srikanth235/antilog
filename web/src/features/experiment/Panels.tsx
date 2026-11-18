import { Link } from "@tanstack/react-router";
import type { Schemas } from "../../api/client";
import { Icon } from "../../components/icon/Icon";
import { agentName, platformName } from "../../lib/domain";
import { dollars } from "../../lib/format";
import { type History, OUTCOME_COLOR, OUTCOME_WORDS } from "../../lib/history";
import { w } from "../../words";
import type { Panel } from "./Header";

export function PanelFrame(props: {
  panel: Panel;
  onClose: () => void;
  sub?: string;
  children: React.ReactNode;
}) {
  const label = w.experiment[props.panel];
  return (
    <aside className="pane" id={`panel-${props.panel}`} aria-label={label}>
      <div className="pane-h">
        <h2>{label}</h2>
        {props.sub === undefined ? null : <span className="sub">{props.sub}</span>}
        <span className="grow" />
        <button
          type="button"
          className="icon-btn sm"
          aria-label={w.experiment.closePanel(label)}
          onClick={props.onClose}
        >
          <Icon name="close" size="s" />
        </button>
      </div>
      {props.children}
    </aside>
  );
}

export function DetailsPanel(props: {
  experiment: Schemas["Experiment"];
  sessions: Schemas["Session"][];
  onClose: () => void;
  onBudget: () => void;
  canEdit: boolean;
}) {
  const e = props.experiment;
  return (
    <PanelFrame panel="details" onClose={props.onClose}>
      <div className="pane-b scr">
        <section className="dp-sec">
          <span className="t-label">{w.experiment.sections.experiment}</span>
          <div className="dp-r">
            <span>{w.state[e.status]}</span>
          </div>
          <Link
            className="textbtn"
            to="/experiments/$experimentId/details"
            params={{ experimentId: e.id }}
          >
            {w.experiment.experimentDetails}
          </Link>
        </section>
        <section className="dp-sec">
          <span className="t-label">{w.experiment.sections.budget}</span>
          <div className="dp-r">
            <span className="mono">{dollars(e.budgetCapCents)}</span>
          </div>
          <button
            type="button"
            className="textbtn"
            disabled={!props.canEdit}
            onClick={props.onBudget}
          >
            {w.projects.changeBudget}
          </button>
        </section>
        <section className="dp-sec">
          <span className="t-label">{w.experiment.sections.sessions}</span>
          {props.sessions.length === 0 ? (
            <span className="xs mut">{w.experiment.noSessions}</span>
          ) : null}
          {props.sessions.map((session) => (
            <Link
              key={session.id}
              className="dp-r"
              to="/experiments/$experimentId"
              params={{ experimentId: e.id }}
              search={{ session: session.id }}
            >
              <span className="col">
                <span>{agentName(session.agent)}</span>
                <span className="xs mut">
                  {w.activity[session.status]} · {platformName(session.platform)}
                </span>
              </span>
            </Link>
          ))}
        </section>
      </div>
    </PanelFrame>
  );
}

export function TrialsPanel(props: {
  history: History;
  sessions: Schemas["Session"][];
  onClose: () => void;
}) {
  const h = props.history;
  const rows = [...h.trials].reverse();
  const agentOf = (id: string) => agentName(props.sessions.find((s) => s.id === id)?.agent ?? "");
  return (
    <PanelFrame panel="trials" onClose={props.onClose} sub={`${h.trials.length} trials`}>
      <div className="pane-b scr">
        {rows.length === 0 ? (
          <p className="sd-none">{h.note ?? w.experiment.trialsPanelNone}</p>
        ) : null}
        {rows.map((row) => {
          const link = row.outcome === "kept";
          const inner = (
            <>
              <span className="h">
                <span className="n">{row.number}</span>
                <span className="t">{row.hypothesis}</span>
                <span className="tg" style={{ ["--c" as string]: OUTCOME_COLOR[row.outcome] }}>
                  {OUTCOME_WORDS[row.outcome]}
                </span>
              </span>
              <span className="m">
                <span className="by">{agentOf(row.session)}</span>
                {row.value === null ? null : (
                  <span className="tv">{`${row.value} ${h.unit}`.trim()}</span>
                )}
              </span>
            </>
          );
          return link ? (
            <Link
              key={row.trial}
              className="tr-r"
              to="/receipts/$trialId"
              params={{ trialId: row.trial }}
            >
              {inner}
            </Link>
          ) : (
            <div key={row.trial} className="tr-r">
              {inner}
            </div>
          );
        })}
      </div>
    </PanelFrame>
  );
}

/** Changes, Terminal, Preview and Files read from the agent platform, which the API does not serve yet. (target) */
export function PlatformPanel({ panel, onClose }: { panel: Panel; onClose: () => void }) {
  return (
    <PanelFrame panel={panel} onClose={onClose}>
      <p className="sd-none">{w.experiment.notYet}</p>
    </PanelFrame>
  );
}

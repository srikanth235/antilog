import { Link } from "@tanstack/react-router";
import type { Schemas } from "../../api/client";
import { useAgentLabel } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { dollars } from "../../lib/format";
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
  session: Schemas["Session"];
  sessions: Schemas["Session"][];
  onClose: () => void;
  onBudget: () => void;
  canEdit: boolean;
}) {
  const e = props.session;
  const agentLabel = useAgentLabel();
  return (
    <PanelFrame panel="details" onClose={props.onClose}>
      <div className="pane-b scr">
        <section className="dp-sec">
          <span className="t-label">{w.experiment.sections.session}</span>
          <div className="dp-r">
            <span>{w.activity[e.status]}</span>
          </div>
        </section>
        <section className="dp-sec">
          <span className="t-label">{w.experiment.sections.budget}</span>
          <div className="dp-r">
            <span className="mono">
              {e.budgetCapCents == null ? w.home.noBudget : dollars(e.budgetCapCents)}
            </span>
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
              to="/sessions/$sessionId"
              params={{ sessionId: session.id }}
            >
              <span className="col">
                <span>{agentLabel(session.agent)}</span>
                <span className="xs mut">{w.activity[session.status]}</span>
              </span>
            </Link>
          ))}
        </section>
      </div>
    </PanelFrame>
  );
}

/** Changes, Terminal, Preview and Files read from the session's machine, which the API does not serve yet. (target) */
export function PlatformPanel({ panel, onClose }: { panel: Panel; onClose: () => void }) {
  return (
    <PanelFrame panel={panel} onClose={onClose}>
      <p className="sd-none">{w.experiment.notYet}</p>
    </PanelFrame>
  );
}

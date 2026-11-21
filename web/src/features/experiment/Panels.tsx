import { Link } from "@tanstack/react-router";
import type { Schemas } from "../../api/client";
import { useAgentLabel } from "../../api/queries";
import { Glyph, Icon } from "../../components/icon/Icon";
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

/**
 * The overview of the session on screen: what it runs on, what it spent and what it reaches.
 * Each section leads to the panel or page that shows it in full. A row the API cannot fill yet is
 * left out: the environment's name, the repos and branch, the context window, and the outputs
 * (files, patches and the delivered pull request). (target)
 */
export function OverviewPanel(props: {
  experiment: Schemas["Experiment"];
  sessions: Schemas["Session"][];
  spentCents: number | undefined;
  connectors: { id: string; name: string }[];
  onClose: () => void;
}) {
  const e = props.experiment;
  const o = w.experiment.overview;
  const agentLabel = useAgentLabel();
  return (
    <PanelFrame panel="details" onClose={props.onClose}>
      <div className="pane-b scr">
        <section className="dp-sec">
          <div className="dp-lh">
            <span className="t-label">{w.experiment.sections.environment}</span>
          </div>
          <Link className="dp-row" to="/settings/$section" params={{ section: "connections" }}>
            <Glyph name="cloud" />
            <span className="v">{o.environments}</span>
            <Icon name="chev" size="s" />
          </Link>
        </section>
        {props.spentCents === undefined ? null : (
          <section className="dp-sec">
            <div className="dp-lh">
              <span className="t-label">{w.experiment.sections.spend}</span>
            </div>
            <div className="dp-row">
              <Glyph name="gauge" />
              <span className="v">{dollars(props.spentCents)}</span>
            </div>
          </section>
        )}
        <section className="dp-sec">
          <div className="dp-lh">
            <span className="t-label">{w.experiment.sections.connectors}</span>
          </div>
          {props.connectors.length === 0 ? (
            <Link className="dp-row" to="/customize/connectors">
              <Glyph name="blocks" />
              <span className="v">{o.noConnectors}</span>
              <Icon name="chev" size="s" />
            </Link>
          ) : null}
          {props.connectors.map((connector) => (
            <Link
              key={connector.id}
              className="dp-row"
              to="/customize/connectors/$id"
              params={{ id: connector.id }}
            >
              <Glyph name="blocks" />
              <span className="v">{connector.name}</span>
              <Icon name="chev" size="s" />
            </Link>
          ))}
        </section>
        <section className="dp-sec">
          <div className="dp-lh">
            <span className="t-label">{w.experiment.sections.sessions}</span>
          </div>
          {props.sessions.length === 0 ? (
            <span className="xs mut">{w.experiment.noSessions}</span>
          ) : null}
          {props.sessions.map((session) => (
            <Link
              key={session.id}
              className="dp-row"
              to="/experiments/$experimentId"
              params={{ experimentId: e.id }}
              search={{ session: session.id }}
            >
              <span className="v">{agentLabel(session.agent)}</span>
              <span className="r">{w.activity[session.status]}</span>
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
  const agentLabel = useAgentLabel();
  const agentOf = (id: string) => agentLabel(props.sessions.find((s) => s.id === id)?.agent ?? "");
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

/** Changes, Terminal, Preview and Files read from the session's machine, which the API does not serve yet. (target) */
export function PlatformPanel({ panel, onClose }: { panel: Panel; onClose: () => void }) {
  return (
    <PanelFrame panel={panel} onClose={onClose}>
      <p className="sd-none">{w.experiment.notYet}</p>
    </PanelFrame>
  );
}

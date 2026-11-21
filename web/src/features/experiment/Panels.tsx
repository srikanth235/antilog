import { Link } from "@tanstack/react-router";
import { useState } from "react";
import type { Schemas } from "../../api/client";
import { useConnectors, useEnvironments } from "../../api/queries";
import { Glyph, Icon } from "../../components/icon/Icon";
import { dollars, share } from "../../lib/format";
import { w } from "../../words";
import { EnvironmentDialog } from "../settings/Environments";
import { EnvironmentPicker } from "./EnvironmentPicker";
import type { Panel } from "./Header";

export function PanelFrame(props: {
  panel: Panel;
  onClose: () => void;
  sub?: string;
  /** A panel that needs room, such as a diff beside its file list. */
  wide?: boolean;
  /** Shown beside the close button. */
  actions?: React.ReactNode;
  children: React.ReactNode;
}) {
  const label = w.experiment[props.panel];
  return (
    <aside
      className={props.wide ? "pane wide" : "pane"}
      id={`panel-${props.panel}`}
      aria-label={label}
    >
      <div className="pane-h">
        <h2>{label}</h2>
        {props.sub === undefined ? null : <span className="sub">{props.sub}</span>}
        <span className="grow" />
        {props.actions}
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
 * Each section leads to the page that shows it in full. A row the API cannot fill yet is left
 * out: the branch, the context window and the outputs (files, patches, pull requests). (target)
 */
export function OverviewPanel(props: {
  session: Schemas["Session"];
  spentCents: number | undefined;
  onClose: () => void;
  onBudget: () => void;
  canEdit: boolean;
}) {
  const s = props.session;
  const o = w.experiment.overview;
  const connectors = useConnectors();
  const environments = useEnvironments();
  const [picking, setPicking] = useState(false);
  const [editing, setEditing] = useState<Schemas["Environment"] | null>(null);
  const spent = props.spentCents;
  const own = environments.data?.find((row) => row.id === (s.environment ?? "default"));
  const cap = s.budgetCapCents ?? null;
  return (
    <PanelFrame panel="details" onClose={props.onClose}>
      <div className="pane-b scr">
        <section className="dp-sec">
          <div className="dp-lh">
            <span className="t-label">{w.experiment.sections.environment}</span>
            <span className="rw g8">
              <button
                type="button"
                className="textbtn"
                style={{ minHeight: 28, fontSize: 12.5 }}
                aria-haspopup="dialog"
                disabled={!props.canEdit}
                onClick={() => setPicking(true)}
              >
                {o.change}
              </button>
              <button
                type="button"
                className="textbtn"
                style={{ minHeight: 28, fontSize: 12.5 }}
                aria-haspopup="dialog"
                disabled={!props.canEdit || own === undefined || own.builtIn}
                onClick={() => setEditing(own ?? null)}
              >
                {o.edit}
              </button>
            </span>
          </div>
          <div className="dp-row two">
            <Glyph name="cloud" />
            <span className="v">
              {own?.name ?? s.environment ?? w.settings.environments.builtInName}
              <span className="s">{o.version(s.environmentVersion ?? 1)}</span>
            </span>
          </div>
          {s.repos.map((repo) => (
            <div key={repo} className="dp-row">
              <Glyph name="code" />
              <span className="v">{repo}</span>
            </div>
          ))}
        </section>
        <section className="dp-sec">
          <div className="dp-lh">
            <span className="t-label">{w.experiment.sections.spend}</span>
            <button
              type="button"
              className="textbtn"
              style={{ minHeight: 28, fontSize: 12.5 }}
              disabled={!props.canEdit}
              onClick={props.onBudget}
            >
              {w.projects.changeBudget}
            </button>
          </div>
          {spent === undefined ? (
            <div className="dp-row">
              <Glyph name="gauge" />
              <span className="v">{cap === null ? w.home.noBudget : dollars(cap)}</span>
            </div>
          ) : cap === null ? (
            <div className="dp-row">
              <Glyph name="gauge" />
              <span className="v">{dollars(spent)}</span>
            </div>
          ) : (
            <div className="dp-bar" style={{ paddingLeft: 0 }}>
              <div className="m">
                <span>{w.experiment.budgetSpent(spent, cap)}</span>
                <span>{`${Math.round(share(spent, cap) * 100)}%`}</span>
              </div>
              <span className="bar" aria-hidden="true">
                <span style={{ width: `${share(spent, cap) * 100}%` }} />
              </span>
            </div>
          )}
        </section>
        <section className="dp-sec">
          <div className="dp-lh">
            <span className="t-label">{w.experiment.sections.connectors}</span>
          </div>
          {(connectors.data ?? []).length === 0 ? (
            <Link className="dp-row" to="/customize/connectors">
              <Glyph name="blocks" />
              <span className="v">{o.noConnectors}</span>
              <Icon name="chev" size="s" />
            </Link>
          ) : null}
          {(connectors.data ?? []).map((connector) => (
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
      </div>
      <EnvironmentPicker
        open={picking}
        onClose={() => setPicking(false)}
        session={s}
        onEdit={(row) => {
          setPicking(false);
          setEditing(row);
        }}
      />
      <EnvironmentDialog
        open={editing !== null}
        environment={editing}
        onClose={() => setEditing(null)}
      />
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

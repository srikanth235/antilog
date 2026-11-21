import { Link } from "@tanstack/react-router";
import { useState } from "react";
import type { Schemas } from "../../api/client";
import {
  useChanges,
  useCompact,
  useConnectors,
  useEnvironments,
  useSessionContext,
} from "../../api/queries";
import { Glyph, Icon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { contextView, tokens } from "../../lib/context";
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
 * Each section leads to the page that shows it in full. A row we cannot back is left out: the
 * branch and the files come from the Changes report, the context from the runner and the gateway,
 * and the pull request waits for the code host. (target)
 */
export function OverviewPanel(props: {
  session: Schemas["Session"];
  spentCents: number | undefined;
  onClose: () => void;
  onBudget: () => void;
  canEdit: boolean;
  onOpenChanges: () => void;
}) {
  const s = props.session;
  const o = w.experiment.overview;
  const connectors = useConnectors();
  const environments = useEnvironments();
  const changes = useChanges(s.id).data?.repos ?? [];
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
          {s.repos.map((repo) => {
            const branch = changes.find((row) => row.repo === repo)?.headBranch ?? "";
            return (
              <div key={repo} className={branch === "" ? "dp-row" : "dp-row two"}>
                <Glyph name="code" />
                <span className="v">
                  {repo}
                  {branch === "" ? null : <span className="s">{branch}</span>}
                </span>
              </div>
            );
          })}
        </section>
        <ContextSection session={s} canEdit={props.canEdit} />
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
        <section className="dp-sec">
          <div className="dp-lh">
            <span className="t-label">{w.experiment.sections.outputs}</span>
          </div>
          {changes.every((repo) => repo.files.length === 0) ? (
            <div className="dp-row">
              <Glyph name="code" />
              <span className="v">{o.noOutputs}</span>
            </div>
          ) : null}
          {changes
            .filter((repo) => repo.files.length > 0)
            .map((repo) => (
              <button
                key={repo.repo}
                type="button"
                className="dp-row"
                onClick={props.onOpenChanges}
                style={{
                  background: "none",
                  border: 0,
                  padding: 0,
                  font: "inherit",
                  cursor: "pointer",
                  textAlign: "left",
                }}
              >
                <Icon name="diff" />
                <span className="v">
                  {repo.repo}
                  <span className="s">{o.filesChanged(repo.files.length)}</span>
                </span>
                <span className="r">
                  <span className="add">+{repo.files.reduce((n, f) => n + f.additions, 0)}</span>{" "}
                  <span className="del">−{repo.files.reduce((n, f) => n + f.deletions, 0)}</span>
                </span>
                <Icon name="chev" size="s" />
              </button>
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

/** How full the model's window is: a segmented bar, a row per part we measure, a warning from 80%. */
function ContextSection(props: { session: Schemas["Session"]; canEdit: boolean }) {
  const c = w.experiment.overview.context;
  const context = useSessionContext(props.session.id);
  const compact = useCompact(props.session.id);
  const view = contextView(context.data);
  const [error, setError] = useState<string | null>(null);
  const ended = ["done", "stopped", "failed"].includes(props.session.status);
  return (
    <section className="dp-sec">
      <div className="dp-lh">
        <span className="t-label">{w.experiment.sections.context}</span>
        {view === null || ended ? null : (
          <button
            type="button"
            className="textbtn"
            style={{ minHeight: 28, fontSize: 12.5 }}
            disabled={!props.canEdit || compact.isPending}
            onClick={() => {
              setError(null);
              compact.mutate(undefined, { onError: (caught) => setError(reason(caught)) });
            }}
          >
            {compact.isPending ? c.compacting : c.compact}
          </button>
        )}
      </div>
      {view === null ? (
        <div className="dp-row">
          <Glyph name="gauge" />
          <span className="v">{c.none}</span>
        </div>
      ) : (
        <div className={view.warn ? "dp-ctx warn" : "dp-ctx"}>
          <div className="dp-bar" style={{ paddingLeft: 0 }}>
            <div className="m">
              <span>{c.used(view.percent)}</span>
              <span>{c.of(tokens(view.used), tokens(view.window))}</span>
            </div>
            <span className="dp-seg" role="img" aria-label={c.ring(view.percent)}>
              {view.rows
                .filter((row) => row.kind !== "free")
                .map((row) => (
                  <span
                    key={row.kind}
                    className={SWATCH[row.kind]}
                    style={{ width: `${row.percent}%` }}
                  />
                ))}
            </span>
          </div>
          {view.warn ? (
            <p className="dp-warn" role="status">
              {c.warn}
            </p>
          ) : null}
          <div>
            {view.rows.map((row) => (
              <div key={row.kind} className="dp-lr">
                <i className={SWATCH[row.kind]} />
                <span className="n">{c.kinds[row.kind]}</span>
                <span className="t">{tokens(row.tokens)}</span>
                <span className="p">{`${row.percent.toFixed(1)}%`}</span>
              </div>
            ))}
          </div>
        </div>
      )}
      {error === null ? null : (
        <p className="error-line" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}

const SWATCH = {
  system: "s0",
  tools: "s1",
  antilog_tools: "s2",
  messages: "s3",
  free: "sf",
} as const;

/** Changes, Terminal, Preview and Files read from the session's machine, which the API does not serve yet. (target) */
export function PlatformPanel({ panel, onClose }: { panel: Panel; onClose: () => void }) {
  return (
    <PanelFrame panel={panel} onClose={onClose}>
      <p className="sd-none">{w.experiment.notYet}</p>
    </PanelFrame>
  );
}

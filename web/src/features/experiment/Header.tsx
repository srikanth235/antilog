import { Link, useNavigate } from "@tanstack/react-router";
import { type Activity, Icon, StatusIcon } from "../../components/icon/Icon";
import { Menu, MenuItem } from "../../components/menu/Menu";
import { Tip } from "../../components/tooltip/Tooltip";
import { w } from "../../words";

export type Panel = "details" | "trials" | "changes" | "terminal" | "preview" | "files";

const PANELS: {
  id: Panel;
  icon: "list" | "chart" | "diff" | "shell" | "globe" | "folder";
  keys: string;
}[] = [
  { id: "details", icon: "list", keys: "D" },
  { id: "trials", icon: "chart", keys: "R" },
  { id: "changes", icon: "diff", keys: "G" },
  { id: "terminal", icon: "shell", keys: "T" },
  { id: "preview", icon: "globe", keys: "P" },
  { id: "files", icon: "folder", keys: "F" },
];

export function Header(props: {
  project: { id: string; name: string } | undefined;
  title: string;
  experimentId: string;
  state: "not_sealed" | "sealed";
  activity: Activity | null;
  panel: Panel | undefined;
  onPanel: (panel: Panel | undefined) => void;
  canStop: boolean;
  onStop: () => void;
  readOnly: boolean;
}) {
  const navigate = useNavigate();
  return (
    <header className="hdr">
      <div className="hbar">
        <div className="hb-l">
          <h1>
            <span className="hbc">
              {props.project === undefined ? null : (
                <>
                  <Link to="/projects/$projectId" params={{ projectId: props.project.id }}>
                    {props.project.name}
                  </Link>
                  <span className="sep" aria-hidden="true">
                    /
                  </span>
                </>
              )}
              <b className="ell">{props.title}</b>
            </span>
          </h1>
          <Menu
            label={w.experiment.sessionMenu}
            className="icon-btn sm"
            trigger={<Icon name="chev" size="s" />}
          >
            <MenuItem
              onSelect={() =>
                void navigate({
                  to: "/experiments/$experimentId/details",
                  params: { experimentId: props.experimentId },
                })
              }
            >
              {w.experiment.experimentDetails}
            </MenuItem>
            <MenuItem
              onSelect={() =>
                void navigator.clipboard.writeText(window.location.href).catch(() => undefined)
              }
            >
              {w.experiment.copyLink}
            </MenuItem>
            <MenuItem disabled={!props.canStop || props.readOnly} onSelect={props.onStop}>
              {w.experiment.stopSession}
            </MenuItem>
            <MenuItem disabled onSelect={() => undefined}>
              {w.experiment.rename}
            </MenuItem>
            <MenuItem disabled onSelect={() => undefined}>
              {w.experiment.handOff}
            </MenuItem>
            <MenuItem disabled onSelect={() => undefined}>
              {w.experiment.archive}
            </MenuItem>
          </Menu>
          <span className="spill" style={{ ["--c" as string]: "var(--ink3)" }}>
            {w.state[props.state]}
          </span>
          {props.activity === null ? null : (
            <span className="spill" style={{ ["--c" as string]: "var(--accent)" }}>
              <StatusIcon activity={props.activity} />
              {w.activity[props.activity]}
            </span>
          )}
          {props.readOnly ? <span className="xs mut">{w.readOnly.viewersRead}</span> : null}
        </div>
        <div className="hb-r">
          {PANELS.map((panel) => {
            const label = w.experiment[panel.id];
            const on = props.panel === panel.id;
            return (
              <Tip key={panel.id} label={`${label} (⌘⇧${panel.keys})`}>
                <button
                  type="button"
                  className="icon-btn"
                  aria-pressed={on}
                  aria-controls={`panel-${panel.id}`}
                  aria-label={label}
                  aria-keyshortcuts={`Meta+Shift+${panel.keys}`}
                  onClick={() => props.onPanel(on ? undefined : panel.id)}
                >
                  <Icon name={panel.icon} />
                </button>
              </Tip>
            );
          })}
          <span className="hsep" aria-hidden="true" />
          <Menu label={w.experiment.more} trigger={<Icon name="more" />}>
            <MenuItem disabled onSelect={() => undefined}>
              {w.experiment.exportConversation}
            </MenuItem>
            <MenuItem disabled onSelect={() => undefined}>
              {w.experiment.shortcuts}
            </MenuItem>
            <MenuItem disabled onSelect={() => undefined}>
              {w.experiment.report}
            </MenuItem>
          </Menu>
          <button type="button" className="btn sm ghost" disabled title={w.experiment.notYet}>
            <Icon name="share" size="s" />
            {w.experiment.share}
          </button>
        </div>
      </div>
    </header>
  );
}

import { useQueryClient } from "@tanstack/react-query";
import { Link, useMatchRoute, useNavigate } from "@tanstack/react-router";
import { api, unwrap } from "../../api/client";
import { useAgentLabel, useMe } from "../../api/queries";
import { Mark } from "../../components/brand/Brand";
import { Group } from "../../components/collapsible/Collapsible";
import { Icon, StatusIcon } from "../../components/icon/Icon";
import { Menu, MenuItem } from "../../components/menu/Menu";
import { setAppearance, useAppearance } from "../../lib/appearance";
import { activityOf, pendingCards } from "../../lib/domain";
import { initial } from "../../lib/format";
import { setFolded, togglePin, useFolded, usePins } from "../../lib/prefs";
import { w } from "../../words";
import { useSidebarData } from "./useSidebarData";

export function Sidebar() {
  const data = useSidebarData();
  const agentLabel = useAgentLabel();
  const pins = usePins();
  const folded = useFolded();
  const matchRoute = useMatchRoute();
  const open = matchRoute({ to: "/experiments/$experimentId", fuzzy: true });
  const openId = open === false ? null : open.experimentId;
  const onProjects = matchRoute({ to: "/projects", fuzzy: true }) !== false;
  const onCustomize = matchRoute({ to: "/customize", fuzzy: true }) !== false;

  const row = (experiment: (typeof data.all)[number], pinned: boolean) => {
    const sessions = data.sessionsOf.get(experiment.id) ?? [];
    const waiting = pendingCards(data.approvalsOf.get(experiment.id) ?? []).length;
    const activity = activityOf(sessions, waiting);
    const nested = experiment.id === openId || pins.includes(experiment.id);
    return (
      <div key={`${pinned ? "pin" : "row"}-${experiment.id}`}>
        <div className="srow">
          <Link
            className="rec ses"
            to="/experiments/$experimentId"
            params={{ experimentId: experiment.id }}
            data-item={experiment.id}
            activeProps={{ "aria-current": "page" }}
          >
            {activity === null ? null : <StatusIcon activity={activity} />}
            <span className="t">{experiment.goal}</span>
          </Link>
          <button
            type="button"
            className="pinb"
            aria-label={(pins.includes(experiment.id) ? w.sidebar.unpin : w.sidebar.pin)(
              experiment.goal,
            )}
            onClick={() => togglePin(experiment.id)}
          >
            <Icon name="pin" size="s" />
          </button>
        </div>
        {nested && sessions.length > 0 ? (
          <div className="sx-l">
            {sessions.map((session) => (
              <Link
                key={session.id}
                className="rec ses sx"
                to="/experiments/$experimentId"
                params={{ experimentId: experiment.id }}
                search={{ session: session.id }}
              >
                <StatusIcon activity={session.status} />
                <span className="t">{w.sidebar.session(agentLabel(session.agent))}</span>
              </Link>
            ))}
          </div>
        ) : null}
      </div>
    );
  };

  const pinned = data.all.filter((experiment) => pins.includes(experiment.id));
  return (
    <nav className="side" aria-label={w.sidebar.main}>
      <div className="side-top">
        <Link className="brand" to="/">
          <Mark width={36} />
          {w.brand}
        </Link>
        <Link className="btn primary full" to="/" data-item="new">
          <Icon name="plus" size="s" />
          {w.sidebar.newExperiment}
        </Link>
        <div className="col" style={{ gap: 2 }}>
          <button
            type="button"
            className="nav"
            data-item="search"
            disabled
            title={w.trial.searchLater}
          >
            <Icon name="search" />
            {w.sidebar.search}
            <span className="kc" aria-hidden="true" style={{ marginLeft: "auto" }}>
              ⌘K
            </span>
          </button>
          <Link
            className="nav"
            to="/projects"
            data-item="projects"
            aria-current={onProjects ? "page" : undefined}
          >
            <Icon name="folder" />
            {w.sidebar.projects}
          </Link>
          <Link
            className="nav"
            to="/customize/evaluations"
            data-item="customize"
            aria-current={onCustomize ? "page" : undefined}
          >
            <Icon name="sliders" />
            {w.sidebar.customize}
          </Link>
        </div>
      </div>
      <div className="side-scroll">
        {pinned.length > 0 ? (
          <div className="col" style={{ gap: 2 }}>
            <span className="t-label sl">{w.sidebar.pinned}</span>
            {pinned.map((experiment) => row(experiment, true))}
          </div>
        ) : null}
        <div className="col" style={{ gap: 8 }}>
          {data.groups
            .filter((group) => group.experiments.length > 0)
            .map((group) => (
              <Group
                key={group.project.id}
                title={group.project.name}
                open={!folded.includes(group.project.id)}
                onOpenChange={(isOpen) => setFolded(group.project.id, !isOpen)}
              >
                {group.experiments.map((experiment) => row(experiment, false))}
              </Group>
            ))}
          {!data.loading && data.all.length === 0 ? (
            <p className="sd-none">{w.sidebar.emptyRows}</p>
          ) : null}
        </div>
      </div>
      <UserMenu />
    </nav>
  );
}

function UserMenu() {
  const signedIn = useMe().data;
  const appearance = useAppearance();
  const navigate = useNavigate();
  const client = useQueryClient();
  const me = signedIn?.member;
  const name = me?.name ?? w.you;
  const via = me?.identity?.provider;

  const signOut = async () => {
    try {
      unwrap(await api.POST("/auth/sign-out"));
    } finally {
      client.clear();
      await navigate({ to: "/signin" });
    }
  };

  return (
    <div className="side-foot">
      <Menu
        label={w.sidebar.userMenu(name)}
        className="nav"
        trigger={
          <>
            <span
              aria-hidden="true"
              className="avatar round"
              style={{ width: 30, height: 30, fontSize: 14 }}
            >
              {initial(name)}
            </span>
            <span className="col grow" style={{ lineHeight: 1.25, textAlign: "left" }}>
              <span style={{ color: "var(--ink)" }}>{name}</span>
              <span className="xs mut">
                {via === undefined ? signedIn?.organization.name : `via ${via}`}
              </span>
            </span>
            <Icon name="more" size="s" />
          </>
        }
      >
        <MenuItem
          onSelect={() =>
            void navigate({ to: "/settings/$section", params: { section: "profile" } })
          }
        >
          {w.sidebar.settings}
        </MenuItem>
        {(["system", "light", "dark"] as const).map((choice) => (
          <MenuItem key={choice} onSelect={() => setAppearance(choice)}>
            <Icon name={choice === "dark" ? "moon" : "sun"} size="s" />
            {w.settings.appearance[choice]}
            {appearance === choice ? <Icon name="check" size="s" /> : null}
          </MenuItem>
        ))}
        <MenuItem onSelect={() => void signOut()}>{w.sidebar.signOut}</MenuItem>
      </Menu>
    </div>
  );
}

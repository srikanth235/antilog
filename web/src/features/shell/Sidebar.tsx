import { useQueryClient } from "@tanstack/react-query";
import { Link, useMatchRoute, useNavigate } from "@tanstack/react-router";
import { api, unwrap } from "../../api/client";
import { useMe } from "../../api/queries";
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
  const pins = usePins();
  const folded = useFolded();
  const matchRoute = useMatchRoute();
  const open = matchRoute({ to: "/sessions/$sessionId", fuzzy: true });
  const openId = open === false ? null : open.sessionId;
  const onProjects = matchRoute({ to: "/projects", fuzzy: true }) !== false;
  const onCustomize = matchRoute({ to: "/customize", fuzzy: true }) !== false;

  const row = (session: (typeof data.all)[number], pinned: boolean) => {
    const waiting = pendingCards(data.approvalsOf.get(session.id) ?? []).length;
    const activity = activityOf([session], waiting);
    return (
      <div key={`${pinned ? "pin" : "row"}-${session.id}`} className="srow">
        <Link
          className="rec ses"
          to="/sessions/$sessionId"
          params={{ sessionId: session.id }}
          data-item={session.id}
          aria-current={session.id === openId ? "page" : undefined}
        >
          {activity === null ? null : <StatusIcon activity={activity} />}
          <span className="t">{session.title}</span>
        </Link>
        <button
          type="button"
          className="pinb"
          aria-label={(pins.includes(session.id) ? w.sidebar.unpin : w.sidebar.pin)(session.title)}
          onClick={() => togglePin(session.id)}
        >
          <Icon name="pin" size="s" />
        </button>
      </div>
    );
  };

  const pinned = data.all.filter((session) => pins.includes(session.id));
  return (
    <nav className="side" aria-label={w.sidebar.main}>
      <div className="side-top">
        <Link className="brand" to="/">
          <Mark width={36} />
          {w.brand}
        </Link>
        <Link className="btn primary full" to="/" data-item="new">
          <Icon name="plus" size="s" />
          {w.sidebar.newSession}
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
            to="/customize/skills"
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
            {pinned.map((session) => row(session, true))}
          </div>
        ) : null}
        <div className="col" style={{ gap: 8 }}>
          {data.groups.map((group) => (
            <Group
              key={group.project.id}
              title={group.project.name}
              open={!folded.includes(group.project.id)}
              onOpenChange={(isOpen) => setFolded(group.project.id, !isOpen)}
            >
              {group.sessions.length === 0 ? (
                <p className="sd-none">{w.sidebar.emptyRows}</p>
              ) : (
                group.sessions.map((session) => row(session, false))
              )}
            </Group>
          ))}
          {!data.loading && data.groups.length === 0 ? (
            <p className="sd-none">{w.sidebar.noProjects}</p>
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

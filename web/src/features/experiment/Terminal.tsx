import "@xterm/xterm/css/xterm.css";
import { useCallback, useRef, useState } from "react";
import type { Schemas } from "../../api/client";
import { useMember, useResumeSession, useSession } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Menu, MenuItem } from "../../components/menu/Menu";
import { Tip } from "../../components/tooltip/Tooltip";
import { canEdit } from "../../lib/domain";
import { w } from "../../words";
import { type Phase, RESUME_TRIES, Shell } from "./shell";
import { nextTab } from "./terminal";

interface Handle {
  clear(): void;
  focus(): void;
}

type Folder = { label: string; tooltip: string };

/** One shell: a terminal asked of the session's machine, drawn with xterm.js. */
function TerminalTab(props: {
  session: Schemas["Session"];
  /** The tab on screen: it takes the keys. */
  active: boolean;
  canType: boolean;
  onFolder: (folder: Folder | undefined) => void;
  handle: (handle: Handle | undefined) => void;
}) {
  const { session, canType } = props;
  const resume = useResumeSession();
  const [attempt, setAttempt] = useState(0);
  const [phase, setPhase] = useState<Phase>("opening");
  const waiting = useRef({ tries: 0 });
  // What the shell calls back with is always the latest of these, whatever the render.
  const latest = useRef(props);
  latest.current = props;
  const live =
    session.status === "queued" ||
    session.status === "working" ||
    session.status === "waiting_for_you";
  const sessionId = session.id;

  // The xterm widget lives on a DOM node: a ref callback makes it and its cleanup unmakes it.
  // `attempt` is in the list so that a retry builds a fresh one.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `attempt` is a retry trigger
  const mount = useCallback(
    (element: HTMLDivElement | null) => {
      if (element === null || !canType || !live) return;
      const shell = new Shell(
        element,
        sessionId,
        {
          phase: setPhase,
          folder: (folder) => latest.current.onFolder(folder),
          retry: () => setAttempt((n) => n + 1),
          isActive: () => latest.current.active,
        },
        waiting.current,
      );
      latest.current.handle({ clear: () => shell.clear(), focus: () => shell.focus() });
      void shell.open();
      return () => {
        latest.current.handle(undefined);
        shell.dispose();
      };
    },
    [attempt, canType, live, sessionId],
  );

  const shown: Phase = !canType ? "readOnly" : !live ? "ended" : phase;
  const onScreen =
    shown === "opening" || shown === "live" || shown === "shellEnded" || shown === "closed";
  return (
    <div className="term-tab" hidden={!props.active}>
      <div
        ref={mount}
        className="term x"
        role="log"
        aria-label={w.terminal.output}
        hidden={!onScreen}
      />
      {shown === "notRunning" ? (
        <div className="term-empty">
          <Icon name="shell" size="l" />
          <p>{w.terminal.resumeLine}</p>
          <span className="xs mut">{w.terminal.resumeNote}</span>
          <button
            type="button"
            className="btn sm primary"
            disabled={resume.isPending}
            onClick={() => {
              waiting.current.tries = RESUME_TRIES;
              setPhase("resuming");
              resume.mutate(sessionId, { onSettled: () => setAttempt((n) => n + 1) });
            }}
          >
            {w.terminal.resume}
          </button>
        </div>
      ) : null}
      {shown === "resuming" ? (
        <div className="term-empty" role="status">
          <Icon name="shell" size="l" />
          <p>{w.terminal.resuming}</p>
        </div>
      ) : null}
      {shown === "ended" ? (
        <div className="term-empty">
          <Icon name="shell" size="l" />
          <p>{w.terminal.ended}</p>
        </div>
      ) : null}
      {shown === "readOnly" ? (
        <div className="term-empty">
          <Icon name="lock" size="l" />
          <p>{w.terminal.readOnlyLine}</p>
          <span className="xs mut">{w.terminal.readOnlyNote}</span>
        </div>
      ) : null}
      {shown === "failed" ? (
        <div className="term-empty" role="alert">
          <Icon name="shell" size="l" />
          <p>{w.terminal.failed}</p>
          <button type="button" className="btn sm" onClick={() => setAttempt((n) => n + 1)}>
            {w.terminal.retry}
          </button>
        </div>
      ) : null}
    </div>
  );
}

/** The session's Terminal panel: tabs of shells in the session's machine. */
export function TerminalPanel(props: { sessionId: string; visible: boolean; onClose: () => void }) {
  const session = useSession(props.sessionId);
  const me = useMember();
  const canType = canEdit(me?.role);
  // Opened when first shown, then kept running while another panel is on screen.
  const [shown, setShown] = useState(false);
  const [tabs, setTabs] = useState<{ key: number; n: number }[]>([]);
  const [serial, setSerial] = useState(0);
  const [started, setStarted] = useState(false);
  const [active, setActive] = useState(0);
  const [maximised, setMaximised] = useState(false);
  const [folders, setFolders] = useState<Record<number, Folder>>({});
  const handles = useRef(new Map<number, Handle>());

  const add = () => {
    const key = serial + 1;
    setSerial(key);
    setTabs((now) => [...now, { key, n: nextTab(now) }]);
    setActive(key);
  };
  if (props.visible && !shown) setShown(true);
  if (shown && !started && canType) {
    // The first tab, once: closing every tab leaves none until a person asks.
    setStarted(true);
    setSerial(1);
    setTabs([{ key: 1, n: 1 }]);
    setActive(1);
  }

  const choose = (key: number) => {
    setActive(key);
    setTimeout(() => handles.current.get(key)?.focus(), 0);
  };
  const close = (key: number) => {
    const left = tabs.filter((tab) => tab.key !== key);
    setTabs(left);
    if (key === active) setActive(left.at(-1)?.key ?? 0);
    setFolders(({ [key]: _gone, ...rest }) => rest);
  };

  if (!shown || session.data === undefined) return null;
  const folder = folders[active];
  const popOut = () =>
    window.open(`/sessions/${props.sessionId}/terminal`, "_blank", "popup,width=900,height=600");
  return (
    <aside
      className={maximised ? "pane max" : "pane"}
      id="panel-terminal"
      aria-label={w.terminal.title}
      hidden={!props.visible}
    >
      <div className="pane-h">
        <h2>{w.terminal.title}</h2>
        {folder === undefined ? null : (
          <Tip label={folder.tooltip}>
            <span className="sub ell term-dir">{folder.label}</span>
          </Tip>
        )}
        <span className="grow" />
        <Tip label={w.terminal.newTerminal}>
          <button
            type="button"
            className="icon-btn sm"
            aria-label={w.terminal.newTerminal}
            disabled={!canType}
            onClick={add}
          >
            <Icon name="plus" size="s" />
          </button>
        </Tip>
        <Menu
          label={w.terminal.menu}
          className="icon-btn sm"
          trigger={<Icon name="more" size="s" />}
        >
          <MenuItem onSelect={() => handles.current.get(active)?.clear()}>
            {w.terminal.clear}
          </MenuItem>
          <MenuItem disabled={tabs.length === 0} onSelect={() => close(active)}>
            {w.terminal.closeTerminal}
          </MenuItem>
          <MenuItem
            disabled={tabs.length === 0}
            onSelect={() => {
              setTabs([]);
              setActive(0);
              setFolders({});
            }}
          >
            {w.terminal.closeAll}
          </MenuItem>
        </Menu>
        <Tip label={w.terminal.popOut}>
          <button
            type="button"
            className="icon-btn sm"
            aria-label={w.terminal.popOut}
            disabled={!canType}
            onClick={popOut}
          >
            <Icon name="ext" size="s" />
          </button>
        </Tip>
        <Tip label={maximised ? w.terminal.restore : w.terminal.maximise}>
          <button
            type="button"
            className="icon-btn sm"
            aria-pressed={maximised}
            aria-label={maximised ? w.terminal.restore : w.terminal.maximise}
            onClick={() => setMaximised((now) => !now)}
          >
            <Icon name="max" size="s" />
          </button>
        </Tip>
        <Tip label={w.experiment.closePanel(w.terminal.title)}>
          <button
            type="button"
            className="icon-btn sm"
            aria-label={w.experiment.closePanel(w.terminal.title)}
            onClick={props.onClose}
          >
            <Icon name="close" size="s" />
          </button>
        </Tip>
      </div>
      {tabs.length > 1 ? (
        <div className="term-tabs" role="tablist" aria-label={w.terminal.tabs}>
          {tabs.map((tab) => (
            <span
              key={tab.key}
              role="tab"
              tabIndex={tab.key === active ? 0 : -1}
              aria-selected={tab.key === active}
              className={tab.key === active ? "tt on" : "tt"}
              onClick={() => choose(tab.key)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") choose(tab.key);
              }}
            >
              {w.terminal.tab(tab.n)}
              <button
                type="button"
                className="icon-btn xs"
                aria-label={w.terminal.closeTab(tab.n)}
                onClick={(event) => {
                  event.stopPropagation();
                  close(tab.key);
                }}
              >
                <Icon name="close" size="s" />
              </button>
            </span>
          ))}
        </div>
      ) : null}
      {canType ? null : (
        <div className="term-empty">
          <Icon name="lock" size="l" />
          <p>{w.terminal.readOnlyLine}</p>
          <span className="xs mut">{w.terminal.readOnlyNote}</span>
        </div>
      )}
      {canType && tabs.length === 0 ? (
        <div className="term-empty">
          <p>{w.terminal.none}</p>
          <button type="button" className="btn sm" onClick={add}>
            {w.terminal.newTerminal}
          </button>
        </div>
      ) : null}
      {canType
        ? tabs.map((tab) => (
            <TerminalTab
              key={tab.key}
              session={session.data}
              active={tab.key === active}
              canType={canType}
              onFolder={(next) => {
                if (next !== undefined) setFolders((now) => ({ ...now, [tab.key]: next }));
              }}
              handle={(handle) => {
                if (handle === undefined) handles.current.delete(tab.key);
                else handles.current.set(tab.key, handle);
              }}
            />
          ))
        : null}
    </aside>
  );
}

/** The pop-out: one shell filling its own window. */
export function TerminalWindow(props: { sessionId: string }) {
  const session = useSession(props.sessionId);
  const me = useMember();
  if (session.data === undefined) return null;
  return (
    <div className="board app shell" style={{ display: "flex", flexDirection: "column" }}>
      <TerminalTab
        session={session.data}
        active
        canType={canEdit(me?.role)}
        onFolder={(folder) => {
          if (folder !== undefined) document.title = `${folder.label} · ${w.terminal.windowTitle}`;
        }}
        handle={() => undefined}
      />
    </div>
  );
}

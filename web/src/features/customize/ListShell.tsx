import type { ReactNode } from "react";
import { Icon } from "../../components/icon/Icon";
import { w } from "../../words";
import { type Tab, Tabs } from "./Tabs";

/** The ListPage frame of a Customize tab: title, tabs, Yours, search, the add action, a one-line rule, the rows. */
export function ListShell(props: {
  active: Tab;
  search: string;
  query: string;
  onQuery: (query: string) => void;
  action: ReactNode;
  lock?: string;
  children: ReactNode;
}) {
  return (
    <section className="page cust">
      <div className="hd">
        <h1 className="t-display">{w.customize.title}</h1>
      </div>
      <div className="ctl">
        <Tabs active={props.active} />
        <span className="vdiv" aria-hidden="true" />
        <div className="segs">
          <span className="seg on" aria-current="page">
            {w.customize.yours}
          </span>
        </div>
        <div className="ctools">
          <label className="sfield">
            <Icon name="search" />
            <input
              placeholder={props.search}
              aria-label={props.search}
              value={props.query}
              onChange={(event) => props.onQuery(event.target.value)}
            />
          </label>
          {props.action}
        </div>
      </div>
      {props.lock === undefined ? null : (
        <span className="lock">
          <Icon name="lock" size="s" />
          {props.lock}
        </span>
      )}
      {props.children}
    </section>
  );
}

/** The back link, title and subtitle of a detail page. */
export function DetailHead(props: {
  glyph: string;
  title: string;
  subtitle: string;
  state?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <div className="rw between wrap g16">
      <div className="rw g16" style={{ minWidth: 0, flex: "1 1 360px" }}>
        <span className="tile lg">
          <span className={`gi g-${props.glyph}`} />
        </span>
        <div className="col g4" style={{ minWidth: 0 }}>
          <h1 className="t-title">{props.title}</h1>
          <span className="ink2">{props.subtitle}</span>
          {props.state}
        </div>
      </div>
      <div className="rw g8 wrap">{props.actions}</div>
    </div>
  );
}

/** Two columns on a detail page, as the boards draw them. */
export function TwoColumns(props: { left: ReactNode; right: ReactNode }) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "minmax(0, 1.2fr) minmax(0, 1fr)",
        gap: 24,
        alignItems: "start",
      }}
    >
      <div className="col g24">{props.left}</div>
      <div className="col g24">{props.right}</div>
    </div>
  );
}

export function Section(props: { title: string; children: ReactNode }) {
  return (
    <div className="col g8">
      <h2 className="sech">{props.title}</h2>
      {props.children}
    </div>
  );
}

/** A switch as the boards draw it: a labelled row with a button on the right. */
export function SwitchRow(props: {
  label: string;
  on: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
}) {
  return (
    <div className="stg" style={{ padding: "8px 0" }}>
      <div className="tx">
        <span className="t-item">{props.label}</span>
      </div>
      <span className="ct">
        <button
          type="button"
          className={props.on ? "sw on" : "sw"}
          role="switch"
          aria-checked={props.on}
          aria-label={props.label}
          disabled={props.disabled === true}
          onClick={() => props.onChange(!props.on)}
        />
      </span>
    </div>
  );
}

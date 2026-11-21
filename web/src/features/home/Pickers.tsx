import { type FocusEvent, type ReactNode, useId, useRef, useState } from "react";
import { Glyph, Icon } from "../../components/icon/Icon";
import { filterRepos, toggleRepo, typedRepo, validBranch } from "../../lib/repos";
import { w } from "../../words";

type Lead = { glyph: "git" | "blocks" | "box" } | { icon: "folder" | "verify" };

function lead(of: Lead) {
  return "glyph" in of ? <Glyph name={of.glyph} /> : <Icon name={of.icon} />;
}

/** A context button of the composer's top strip, with its popup above it; focus leaving closes it. */
function Pick(props: {
  label: string;
  value: string;
  lead: Lead;
  disabled?: boolean;
  /** The popup's content, given a way to close it. */
  children: (close: () => void) => ReactNode;
  /** "opt" is the quiet button of the bottom row. */
  variant?: "opt";
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLSpanElement>(null);
  const id = useId();
  const leave = (event: FocusEvent<HTMLSpanElement>) => {
    if (!(event.relatedTarget instanceof Node && root.current?.contains(event.relatedTarget)))
      setOpen(false);
  };
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: key and focus events bubble up from the popup inside; the wrapper is no control of its own.
    <span
      className="pkw"
      ref={root}
      onBlur={leave}
      onKeyDown={(event) => {
        if (event.key === "Escape" && open) {
          event.preventDefault();
          setOpen(false);
          root.current?.querySelector<HTMLElement>("button")?.focus();
        }
      }}
    >
      <button
        type="button"
        className={props.variant ?? "pk"}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? id : undefined}
        aria-label={props.label}
        disabled={props.disabled === true}
        onClick={() => setOpen(!open)}
      >
        {lead(props.lead)}
        <span>{props.value}</span>
        <Icon name="chev" size="s" />
      </button>
      {open ? (
        <div className="menu nh-m" id={id}>
          {props.children(() => setOpen(false))}
        </div>
      ) : null}
    </span>
  );
}

/** Repos: a searchable multi-select with a check per repo and "No repository" at the bottom. */
export function RepoPicker(props: {
  repos: readonly string[];
  chosen: readonly string[];
  label: string;
  disabled?: boolean;
  onChange: (chosen: string[]) => void;
}) {
  const [query, setQuery] = useState("");
  const typed = typedRepo(query, [...props.repos, ...props.chosen]);
  // A repo typed in earlier stays listed, ahead of the rest.
  const extra = props.chosen.filter((repo) => !props.repos.includes(repo));
  const shown = filterRepos([...extra, ...props.repos], query);
  const row = (repo: string) => {
    const on = props.chosen.includes(repo);
    return (
      <button
        key={repo}
        type="button"
        className="mi"
        role="option"
        aria-selected={on}
        onClick={() => props.onChange(toggleRepo(props.chosen, repo))}
      >
        {on ? <Icon name="check" className="nh-ck" /> : <span className="nh-ck" />}
        {repo}
      </button>
    );
  };
  return (
    <Pick
      label={w.home.repos}
      value={props.label}
      lead={{ icon: "folder" }}
      disabled={props.disabled === true}
    >
      {() => (
        <div role="listbox" aria-label={w.home.repos} aria-multiselectable="true" className="nh-l">
          <label className="nav sqf">
            <Icon name="search" />
            <input
              className="sqin"
              type="search"
              placeholder={w.home.searchRepos}
              aria-label={w.home.searchReposLabel}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && typed !== null) {
                  event.preventDefault();
                  props.onChange(toggleRepo(props.chosen, typed));
                  setQuery("");
                }
              }}
            />
          </label>
          {typed === null ? null : (
            <button
              type="button"
              className="mi"
              role="option"
              aria-selected={false}
              onClick={() => {
                props.onChange(toggleRepo(props.chosen, typed));
                setQuery("");
              }}
            >
              <span className="nh-ck" />
              {w.home.takeRepo(typed)}
            </button>
          )}
          {shown.map(row)}
          {shown.length === 0 && typed === null ? <p className="mh">{w.home.noMatch}</p> : null}
          <div className="msep" />
          <button
            type="button"
            className="mi"
            role="option"
            aria-selected={props.chosen.length === 0}
            onClick={() => props.onChange([])}
          >
            {props.chosen.length === 0 ? (
              <Icon name="check" className="nh-ck" />
            ) : (
              <span className="nh-ck" />
            )}
            {w.home.noRepository}
          </button>
        </div>
      )}
    </Pick>
  );
}

/** Branch: a typed name; empty is each repo's default branch. */
export function BranchPicker(props: {
  branch: string;
  disabled?: boolean;
  onChange: (branch: string) => void;
}) {
  const bad = props.branch !== "" && !validBranch(props.branch);
  return (
    <Pick
      label={w.home.branch}
      value={props.branch === "" ? w.home.branchDefault : props.branch}
      lead={{ glyph: "git" }}
      disabled={props.disabled === true}
    >
      {(close) => (
        <div className="nh-l">
          <label className="nav sqf">
            <Glyph name="git" />
            <input
              className="sqin"
              type="text"
              placeholder={w.home.branchPlaceholder}
              aria-label={w.home.branchPlaceholder}
              aria-invalid={bad}
              value={props.branch}
              onChange={(event) => props.onChange(event.target.value.trim())}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  if (!bad) close();
                }
              }}
            />
          </label>
          <p className="mh">{bad ? w.home.branchInvalid : w.home.branchHint}</p>
        </div>
      )}
    </Pick>
  );
}

/** One choice from a short list: the project, the environment, the permission mode. */
export function ListPicker<T extends string>(props: {
  label: string;
  lead: Lead;
  value: T | null;
  options: readonly { value: T; label: string }[];
  disabled?: boolean;
  variant?: "opt";
  onChange: (value: T) => void;
}) {
  const current = props.options.find((option) => option.value === props.value);
  return (
    <Pick
      label={props.label}
      value={current?.label ?? ""}
      lead={props.lead}
      disabled={props.disabled === true}
      {...(props.variant === undefined ? {} : { variant: props.variant })}
    >
      {(close) => (
        <div role="listbox" aria-label={props.label} className="nh-l">
          {props.options.map((option) => (
            <button
              key={option.value}
              type="button"
              className="mi"
              role="option"
              aria-selected={option.value === props.value}
              onClick={() => {
                props.onChange(option.value);
                close();
              }}
            >
              {option.value === props.value ? (
                <Icon name="check" className="nh-ck" />
              ) : (
                <span className="nh-ck" />
              )}
              {option.label}
            </button>
          ))}
        </div>
      )}
    </Pick>
  );
}

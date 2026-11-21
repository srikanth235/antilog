import {
  type CSSProperties,
  type FocusEvent,
  type KeyboardEvent,
  type MouseEvent,
  useCallback,
  useId,
  useMemo,
  useRef,
  useState,
} from "react";
import { Icon } from "../../components/icon/Icon";
import {
  byShortcut,
  contextLabel,
  effortAfterSwitch,
  favourites,
  levelLabel,
  levelsOf,
  type ModelInfo,
  priceLabel,
  search,
} from "../../lib/models";
import { w } from "../../words";

type View = "menu" | "effort" | "more";

const EDGE = 12;
/** The main menu, the gap and the Effort submenu side by side. */
const WIDEST = 488;

/** Where the menus open from a button at `rect`: above or below, whichever has more room, and from the side that fits. */
export type Place = { down: boolean; left: boolean; room: number };
export function placeOf(
  rect: { top: number; bottom: number; right: number },
  view: { width: number; height: number; left: number } = {
    width: innerWidth,
    height: innerHeight,
    left: 0,
  },
): Place {
  const above = rect.top - EDGE;
  const below = view.height - rect.bottom - EDGE;
  const down = below > above;
  return {
    down,
    left: rect.right - WIDEST < view.left + EDGE,
    room: Math.max(0, Math.floor((down ? below : above) - 6)),
  };
}

/** The model's name as the button shows it: from the list, else what follows the provider. */
const focusIt = (element: HTMLElement | null) => element?.focus();

/** A mouse press on a menu row keeps focus where it is, so the popover does not close before the click lands. */
function keepFocus(event: MouseEvent<HTMLElement>) {
  if (!(event.target instanceof HTMLInputElement)) event.preventDefault();
}

function nameOf(models: readonly ModelInfo[], id: string): string {
  return models.find((model) => model.id === id)?.name ?? id.split("/").slice(1).join("/");
}

/**
 * The composer's model and effort control, as Claude Code's: a quiet text button, "GLM 5.3 Flash
 * Medium", that opens a small menu of favourite models (number keys pick them), an Effort
 * submenu and a searchable list of every model the keys reach. There is no fast-mode toggle.
 */
export function ModelPicker(props: {
  /** Every model the organization's keys reach. */
  models: readonly ModelInfo[];
  /** The current `provider/model`, or none yet. */
  model: string | null;
  effort: string | null;
  /** The models used lately, newest first. */
  recent: readonly string[];
  disabled?: boolean;
  onChange: (model: string, effort: string | null) => void;
}) {
  const [view, setView] = useState<View | null>(null);
  const [query, setQuery] = useState("");
  const root = useRef<HTMLSpanElement>(null);
  const menuId = useId();
  const open = view !== null;
  const current = props.models.find((model) => model.id === props.model);
  const shown = useMemo(
    () => favourites(props.recent, props.models, props.model),
    [props.recent, props.models, props.model],
  );
  const levels = levelsOf(current);
  const [place, setPlace] = useState<Place>({ down: false, left: false, room: 0 });
  const toggle = () => {
    if (open) return close();
    if (root.current !== null) {
      const page = root.current.closest("main")?.getBoundingClientRect();
      setPlace(
        placeOf(root.current.getBoundingClientRect(), {
          width: innerWidth,
          height: innerHeight,
          left: page?.left ?? 0,
        }),
      );
    }
    setView("menu");
  };

  const close = useCallback(() => {
    setView(null);
    setQuery("");
  }, []);
  const pick = (model: ModelInfo) => {
    props.onChange(model.id, effortAfterSwitch(model, props.effort));
    close();
  };

  const back = () => {
    if (view === "menu" || view === null) close();
    else setView("menu");
  };

  // Keys inside the open menu: Escape steps back or closes, a number picks a favourite, the
  // arrows move between rows.
  const keys = (event: KeyboardEvent<HTMLSpanElement>) => {
    if (view === null) return;
    if (event.key === "Escape") {
      event.preventDefault();
      back();
      return;
    }
    if (view !== "menu" || event.metaKey || event.ctrlKey || event.altKey) return;
    const model = byShortcut(event.key, shown);
    if (model !== undefined) {
      event.preventDefault();
      pick(model);
    }
  };

  // Focus leaving the control closes it. A click on a button inside does not move focus away
  // (`keepFocus`), so a pick is never lost to the close.
  const leave = (event: FocusEvent<HTMLSpanElement>) => {
    if (!(event.relatedTarget instanceof Node && root.current?.contains(event.relatedTarget)))
      close();
  };

  const empty = props.models.length === 0 && props.model === null;
  const label = props.model === null ? w.model.noModels : nameOf(props.models, props.model);

  // Up and Down move between the buttons of the open menu.
  const arrows = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const items = [...event.currentTarget.querySelectorAll<HTMLElement>("button")];
    const at = items.indexOf(document.activeElement as HTMLElement);
    const next = event.key === "ArrowDown" ? at + 1 : at - 1;
    items[(next + items.length) % items.length]?.focus();
    event.preventDefault();
  };

  const groups = search(props.models, query);

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: key and focus events bubble up from the menu inside; the wrapper is no control of its own.
    <span
      className={`mpw${place.down ? " dn" : ""}${place.left ? " lf" : ""}`}
      style={{ "--mp-room": `${place.room}px` } as CSSProperties}
      ref={root}
      onKeyDown={keys}
      onBlur={leave}
    >
      {view === "more" ? (
        <section
          className="mp more"
          aria-label={w.model.moreModels}
          tabIndex={-1}
          ref={focusIt}
          onKeyDown={arrows}
          onMouseDown={keepFocus}
        >
          <input
            className="mp-q"
            type="search"
            ref={focusIt}
            placeholder={w.model.search}
            aria-label={w.model.searchLabel}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <div className="mp-list">
            {groups.length === 0 ? <p className="mp-e">{w.model.none}</p> : null}
            {groups.map((group) => (
              <section key={group.provider} aria-label={w.model.providers[group.provider]}>
                <div className="mp-h">
                  {group.provider === "anthropic" || group.provider === "openai" ? (
                    <span className={`lg lg-${group.provider}`} aria-hidden="true" />
                  ) : null}
                  {w.model.providers[group.provider] ?? group.provider}
                </div>
                {group.models.map((model) => (
                  <button
                    key={model.id}
                    type="button"
                    className="mp-m"
                    data-model={model.id}
                    aria-current={model.id === props.model ? "true" : undefined}
                    title={`${w.model.context(contextLabel(model.contextWindow))} · ${w.model.price}`}
                    onClick={() => pick(model)}
                  >
                    {model.id === props.model ? <Icon name="check" className="ck" /> : <span />}
                    <span className="nm">{model.name}</span>
                    <span className="num">{contextLabel(model.contextWindow)}</span>
                    <span className="num">{priceLabel(model)}</span>
                  </button>
                ))}
              </section>
            ))}
          </div>
        </section>
      ) : open ? (
        <>
          <div className="mp" role="menu" id={menuId} aria-label={w.model.menu} onKeyDown={arrows}>
            {shown.map((model, at) => (
              <button
                key={model.id}
                type="button"
                className="mp-i"
                role="menuitemradio"
                aria-checked={model.id === props.model}
                aria-keyshortcuts={String(at + 1)}
                onClick={() => pick(model)}
              >
                {model.id === props.model ? <Icon name="check" className="ck" /> : <span />}
                <span className="nm">{model.name}</span>
                <span className="k">{at + 1}</span>
              </button>
            ))}
            {shown.length > 0 ? <hr className="mp-sep" /> : null}
            {levels.length === 0 ? null : (
              <>
                <button
                  type="button"
                  className={`mp-i${view === "effort" ? " on" : ""}`}
                  role="menuitem"
                  aria-haspopup="menu"
                  aria-expanded={view === "effort"}
                  onClick={() => setView(view === "effort" ? "menu" : "effort")}
                >
                  <span />
                  <span className="nm">{w.model.effort}</span>
                  <span className="val">
                    {props.effort === null ? "" : levelLabel(props.effort)}
                    <Icon name="chev" size="s" />
                  </span>
                </button>
                <hr className="mp-sep" />
              </>
            )}
            <button
              type="button"
              className="mp-i"
              role="menuitem"
              aria-haspopup="dialog"
              onClick={() => setView("more")}
            >
              <span />
              <span className="nm">{w.model.moreModels}</span>
              <span className="val">
                <Icon name="chev" size="s" />
              </span>
            </button>
          </div>
          {view === "effort" ? (
            <div
              className="mp sub"
              role="menu"
              aria-label={w.model.effort}
              tabIndex={-1}
              onKeyDown={arrows}
              onMouseDown={keepFocus}
            >
              {levels.map((level) => (
                <button
                  key={level}
                  type="button"
                  className="mp-i"
                  role="menuitemradio"
                  aria-checked={level === props.effort}
                  onClick={() => {
                    if (props.model !== null) props.onChange(props.model, level);
                    close();
                  }}
                >
                  {level === props.effort ? <Icon name="check" className="ck" /> : <span />}
                  <span className="nm">{levelLabel(level)}</span>
                  <span />
                </button>
              ))}
            </div>
          ) : null}
        </>
      ) : null}
      <button
        type="button"
        className="mb"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        aria-label={w.model.button}
        title={empty ? w.model.noModelsHint : w.model.button}
        disabled={props.disabled === true || empty}
        onClick={toggle}
      >
        <span>{label}</span>
        {props.effort === null || levels.length === 0 ? null : (
          <span className="ef">{levelLabel(props.effort)}</span>
        )}
      </button>
    </span>
  );
}

import { type FormEvent, type ReactNode, useState } from "react";
import type { Schemas } from "../../api/client";
import { Icon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { composerPlaceholderKey } from "../../lib/domain";
import { dollars } from "../../lib/format";
import { w } from "../../words";

type Status = Schemas["SessionStatus"] | "none";

export function Composer(props: {
  status: Status;
  /** An approval card is showing in the thread. */
  hasCard: boolean;
  canSend: boolean;
  agent: string;
  budgetCents: number | null;
  /** The model and effort picker, shown under the input at the right. */
  picker?: ReactNode;
  onSend: (text: string) => Promise<void>;
  onInterrupt: () => void;
  onContinue: (text: string) => Promise<void>;
}) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const ended =
    props.status === "done" ||
    props.status === "stopped" ||
    props.status === "failed" ||
    props.status === "none";
  const placeholder = props.canSend
    ? w.experiment.composerPlaceholder[composerPlaceholderKey(props.status, props.hasCard)]
    : w.readOnly.viewersRead;

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const value = text.trim();
    if (value === "") return;
    setBusy(true);
    setError(null);
    try {
      await (ended ? props.onContinue(value) : props.onSend(value));
      setText("");
    } catch (caught) {
      setError(reason(caught));
    } finally {
      setBusy(false);
    }
  };

  const budget = props.budgetCents === null ? w.home.noBudget : dollars(props.budgetCents);
  const working = props.status === "working" && text.trim() === "";
  return (
    <div className="dock ac-dock">
      <div className="c760">
        <form className="cmpw" onSubmit={(event) => void submit(event)}>
          <div className="cmp ac-one">
            <button
              type="button"
              className="ib"
              aria-label={w.home.attach}
              title={w.home.attach}
              disabled
            >
              <Icon name="plus" />
            </button>
            <textarea
              className="cmp-in"
              rows={1}
              aria-label={w.trial.message}
              placeholder={placeholder}
              disabled={!props.canSend}
              value={text}
              onChange={(event) => setText(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  event.currentTarget.form?.requestSubmit();
                }
              }}
            />
            <span className="ac-opts">
              <span
                className="opt"
                title={`${w.experiment.agentLabel}: ${props.agent}`}
                role="img"
                aria-label={`${w.experiment.agentLabel}: ${props.agent}`}
              >
                <Icon name="sliders" />
              </span>
              <span
                className="opt"
                role="img"
                aria-label={`${w.home.budget}: ${budget}`}
                title={`${w.home.budget}: ${budget}`}
              >
                <span className="ring" style={{ ["--p" as string]: 0 }} />
              </span>
            </span>
            <span className="go">
              {working ? (
                <button
                  type="button"
                  className="ib stop"
                  aria-label={w.experiment.interrupt}
                  title={w.experiment.interrupt}
                  disabled={!props.canSend}
                  onClick={props.onInterrupt}
                >
                  <i className="ic st-stop" />
                </button>
              ) : (
                <button
                  type="submit"
                  className="ib send"
                  aria-label={w.experiment.send}
                  title={w.experiment.send}
                  disabled={!props.canSend || busy || text.trim() === ""}
                >
                  <Icon name="return" />
                </button>
              )}
            </span>
          </div>
          {props.picker === undefined ? null : <div className="cmp-opt">{props.picker}</div>}
          {error === null ? null : (
            <p className="error-line" role="alert">
              {error}
            </p>
          )}
        </form>
      </div>
    </div>
  );
}

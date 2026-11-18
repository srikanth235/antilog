import { type FormEvent, useState } from "react";
import type { Schemas } from "../../api/client";
import { Icon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { dollars } from "../../lib/format";
import { w } from "../../words";

type Status = Schemas["SessionStatus"] | "none";

export function Composer(props: {
  status: Status;
  canSend: boolean;
  agent: string;
  budgetCents: number;
  onSend: (text: string) => Promise<void>;
  onInterrupt: () => void;
  onContinue: (text: string) => Promise<void>;
}) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const ended = props.status === "Done" || props.status === "Failed" || props.status === "none";
  const placeholder = props.canSend
    ? w.experiment.composerPlaceholder[props.status === "none" ? "none" : props.status]
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

  const working = props.status === "Working" && text.trim() === "";
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
                aria-label={`${w.home.budget}: ${dollars(props.budgetCents)}`}
                title={`${w.home.budget}: ${dollars(props.budgetCents)}`}
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

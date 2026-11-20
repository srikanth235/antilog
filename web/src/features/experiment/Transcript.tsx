import type { Schemas } from "../../api/client";
import { Mark } from "../../components/brand/Brand";
import { Failed } from "../../components/state/State";
import { Markdown } from "../../lib/markdown";
import { w } from "../../words";

/** One thing said in a session: a person's words, the agent's, or the name of a tool it called. Never a tool's output. */
export function Message(props: { event: Schemas["SessionEvent"]; agent: string }) {
  const { event } = props;
  if (event.kind === "person")
    return (
      <div className="bubble">
        <span>{event.text}</span>
      </div>
    );
  if (event.kind === "agent")
    return (
      <div className="am">
        <span className="av">
          <Mark width={20} />
        </span>
        <div className="bd">
          <span className="by">
            <b>{w.experiment.antilog}</b>
            <span className="via">{props.agent}</span>
          </span>
          <div className="tx md">
            <Markdown text={event.text} />
          </div>
        </div>
      </div>
    );
  return <div className="tl">{w.experiment.calledTool(event.text)}</div>;
}

export function TranscriptError(props: { error: unknown; onRetry: () => void }) {
  return props.error === null || props.error === undefined ? null : (
    <Failed error={props.error} retry={props.onRetry} />
  );
}

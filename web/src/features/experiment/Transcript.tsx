import type { Schemas } from "../../api/client";
import { Mark } from "../../components/brand/Brand";
import { Failed } from "../../components/state/State";
import { agentName } from "../../lib/domain";
import { w } from "../../words";

/** What was said in a session: people's words, the agent's, and the name of each tool it called. Never a tool's output. */
export function Transcript(props: {
  events: readonly Schemas["SessionEvent"][];
  agent: string;
  error: unknown;
  onRetry: () => void;
}) {
  return (
    <>
      {props.events.map((event) =>
        event.kind === "person" ? (
          <div key={event.seq} className="bubble">
            <span>{event.text}</span>
          </div>
        ) : event.kind === "agent" ? (
          <div key={event.seq} className="am">
            <span className="av">
              <Mark width={20} />
            </span>
            <div className="bd">
              <span className="by">
                <b>{w.experiment.antilog}</b>
                <span className="via">{agentName(props.agent)}</span>
              </span>
              <p className="tx">{event.text}</p>
            </div>
          </div>
        ) : (
          <div key={event.seq} className="tl">
            {w.experiment.calledTool(event.text)}
          </div>
        ),
      )}
      {props.error === null || props.error === undefined ? null : (
        <Failed error={props.error} retry={props.onRetry} />
      )}
    </>
  );
}

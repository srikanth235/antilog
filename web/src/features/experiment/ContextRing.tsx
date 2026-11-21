import { useSessionContext } from "../../api/queries";
import { contextView } from "../../lib/context";
import { w } from "../../words";

/** A small ring beside the composer's options: how much of the model's window is used. */
export function ContextRing({ sessionId }: { sessionId: string }) {
  const view = contextView(useSessionContext(sessionId).data);
  if (view === null) return null;
  const label = w.experiment.overview.context.ring(view.percent);
  return (
    <span className="opt" role="img" aria-label={label} title={label}>
      <span
        className={view.warn ? "ring warn" : "ring"}
        style={{ ["--p" as string]: view.percent }}
      />
    </span>
  );
}

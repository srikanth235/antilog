import { Link } from "@tanstack/react-router";
import { OUTCOME_COLOR, OUTCOME_WORDS, type TrialRow } from "../../lib/history";
import { w } from "../../words";

/** One trial as the judge reported it. Cost, files and guardrail lines are not served yet. (target) */
export function TrialCard({ row, unit }: { row: TrialRow; unit: string }) {
  const measuring = row.outcome === "measuring";
  return (
    <article className={`cd tc${measuring ? " meas" : ""}`} data-trial={row.trial}>
      <div className="tc-h">
        <span className="tc-n">Trial {row.number}</span>
        <span className="tc-t">{row.hypothesis}</span>
        <span
          className={`tg${measuring ? " hollow" : ""}`}
          style={{ ["--c" as string]: OUTCOME_COLOR[row.outcome] }}
        >
          {OUTCOME_WORDS[row.outcome]}
        </span>
      </div>
      <div className="tc-m">
        {row.value === null ? (
          <span>{measuring ? w.trial.measuring : ""}</span>
        ) : (
          <span className="mono">{`${row.value} ${unit}`.trim()}</span>
        )}
        {row.parent === null ? null : <span>{w.trial.startedFrom}</span>}
        {row.outcome === "kept" ? (
          <Link className="textbtn" to="/receipts/$trialId" params={{ trialId: row.trial }}>
            {w.win.viewReceipt}
          </Link>
        ) : null}
      </div>
    </article>
  );
}

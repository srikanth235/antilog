import type { Schemas } from "../../api/client";
import { dollars } from "../../lib/format";
import { w } from "../../words";

/** Spend rows name a session and a kind, not a trial, so the split is by kind: what the money bought. */
export function SpendBreakdown(props: {
  rows: readonly Schemas["KindSpend"][];
  none: string;
  label: string;
  total: string;
}) {
  const total = props.rows.reduce((sum, row) => sum + row.cents, 0);
  if (props.rows.length === 0) return <p className="sm mut">{props.none}</p>;
  return (
    <dl className="kvl" aria-label={props.label}>
      {props.rows.map((row) => (
        <div key={row.kind} className="stg">
          <dt className="tx t-item">{w.spendKinds[row.kind]}</dt>
          <dd className="ct mono">{dollars(row.cents)}</dd>
        </div>
      ))}
      <div className="stg">
        <dt className="tx t-item">{props.total}</dt>
        <dd className="ct mono">{dollars(total)}</dd>
      </div>
    </dl>
  );
}

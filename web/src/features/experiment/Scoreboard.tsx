import { useExperimentSpend, useReceipt } from "../../api/queries";
import { dollars, share } from "../../lib/format";
import { type History, kept } from "../../lib/history";
import { w } from "../../words";

/** Pinned once the experiment is sealed. It shows only the judge's numbers. */
export function Scoreboard(props: {
  history: History;
  experiment: string;
  budgetCents: number;
  metric: string | null;
}) {
  const { history } = props;
  return history.best === null ? (
    <Strip
      history={history}
      experiment={props.experiment}
      baseline={null}
      bestValue={null}
      budgetCents={props.budgetCents}
      metric={props.metric}
    />
  ) : (
    <WithBest
      history={history}
      experiment={props.experiment}
      budgetCents={props.budgetCents}
      metric={props.metric}
      best={history.best}
    />
  );
}

function WithBest(props: {
  history: History;
  experiment: string;
  budgetCents: number;
  metric: string | null;
  best: string;
}) {
  const receipt = useReceipt(props.best);
  const body = receipt.data?.body.experiment;
  return (
    <Strip
      history={props.history}
      experiment={props.experiment}
      baseline={body?.baseline ?? null}
      budgetCents={props.budgetCents}
      metric={props.metric}
      bestValue={props.history.trials.find((t) => t.trial === props.best)?.value ?? null}
    />
  );
}

function Strip(props: {
  history: History;
  experiment: string;
  baseline: number | null;
  bestValue: number | null;
  budgetCents: number;
  metric: string | null;
}) {
  const { history } = props;
  const spend = useExperimentSpend(props.experiment);
  const spent = spend.data?.spentCents ?? 0;
  const unit = history.unit;
  const bestValue = props.bestValue;
  return (
    <div className="sb">
      <div className="c760">
        <div className="sb-in">
          <div className="sb-l">
            <span className="sb-k">{props.metric ?? w.experiment.stateLabel}</span>
            <span className="sb-big">
              {props.baseline === null || bestValue === null ? (
                <span>{w.experiment.scoreboardNone}</span>
              ) : (
                <>
                  <span>{`${props.baseline} ${unit}`.trim()}</span>
                  <span className="to">to</span>
                  <span>{`${bestValue} ${unit}`.trim()}</span>
                  <span className="dlt">{`${(((bestValue - props.baseline) / props.baseline) * 100).toFixed(1)}%`}</span>
                </>
              )}
            </span>
            <span className="sb-s">
              <span>
                <b>{history.trials.length}</b> {w.scoreboard.trials}
              </span>
              <span>
                <b>{kept(history)}</b> {w.scoreboard.kept}
              </span>
              <span>
                <b>{dollars(spent)}</b> {w.scoreboard.of} {dollars(props.budgetCents)}{" "}
                {w.scoreboard.budget}
              </span>
              <span className="bar" aria-hidden="true">
                <span style={{ width: `${Math.round(share(spent, props.budgetCents) * 100)}%` }} />
              </span>
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

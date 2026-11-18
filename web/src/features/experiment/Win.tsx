import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { api, unwrap } from "../../api/client";
import { useReceipt } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { useToast } from "../../components/toast/Toast";
import { dollars } from "../../lib/format";
import type { History } from "../../lib/history";
import { w } from "../../words";
import { useWinValue, WinState } from "../value/WinState";

/** The best Kept trial: the receipt in short, and "Open pull request", which raises a card. */
export function Win(props: {
  history: History;
  trial: string;
  experiment: string;
  canDeliver: boolean;
  onCard: () => void;
}) {
  const receipt = useReceipt(props.trial);
  const toast = useToast();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const row = props.history.trials.find((t) => t.trial === props.trial);
  const winValue = useWinValue(props.experiment, props.trial);
  const body = receipt.data?.body;
  const unit = props.history.unit;
  if (row === undefined) return null;
  const baseline = body?.experiment.baseline;
  const change =
    baseline === undefined || row.value === null ? null : ((row.value - baseline) / baseline) * 100;

  const deliver = async () => {
    setBusy(true);
    setError(null);
    try {
      unwrap(
        await api.POST("/experiments/{id}/tools/{tool}", {
          params: { path: { id: props.experiment, tool: "deliver" } },
          body: { input: { trial: props.trial } },
        }),
      );
      toast(w.win.prRaised, w.win.prWaits);
      props.onCard();
    } catch (caught) {
      setError(reason(caught));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="cd wn" aria-label={w.win.heading}>
      <div className="wn-h">
        <Icon name="verify" />
        <h3>
          {w.win.heading}:{" "}
          {baseline === undefined || row.value === null
            ? ""
            : w.win.from(`${baseline} ${unit}`, `${row.value} ${unit}`)}
        </h3>
        <span className="tg" style={{ ["--c" as string]: "var(--good)" }}>
          {w.win.kept}
        </span>
        {change === null ? null : (
          <span className="dlt" style={{ marginLeft: "auto" }}>{`${change.toFixed(1)}%`}</span>
        )}
      </div>
      {body === undefined ? null : (
        <>
          <p className="wn-r">
            {w.win.summary(
              body.experiment.version,
              body.experiment.repeats,
              body.experiment.noise,
              body.experiment.trialsTried,
              body.experiment.holdout === "passed" ? w.receipt.passed : w.receipt.failed,
            )}
          </p>
          <p className="ac-vl">
            {winValue?.yearlyCents == null ? (
              <span>{w.receipt.noValue}</span>
            ) : (
              <span>
                <b>
                  {w.receipt.worth}: {w.value.yearly(dollars(winValue.yearlyCents))}
                </b>{" "}
                <WinState state={winValue.state} /> {w.value.measuredOffline}
              </span>
            )}
            <span>{w.win.cost(dollars(body.experiment.costOfWin))}</span>
          </p>
          {body.experiment.secondOpinion == null ? null : (
            <div className="wn-o">
              <Icon name="check" size="s" />
              <span>
                {body.experiment.secondOpinion.flags.length === 0
                  ? w.receipt.clean(body.experiment.secondOpinion.provider)
                  : w.receipt.flagged(
                      body.experiment.secondOpinion.provider,
                      body.experiment.secondOpinion.flags.join(", "),
                    )}
              </span>
            </div>
          )}
        </>
      )}
      {error === null ? null : (
        <p className="error-line" role="alert">
          {error}
        </p>
      )}
      <div className="wn-f">
        <Link className="btn sm" to="/receipts/$trialId" params={{ trialId: props.trial }}>
          {w.win.viewReceipt}
        </Link>
        <button
          type="button"
          className="btn sm primary"
          disabled={!props.canDeliver || busy}
          onClick={() => void deliver()}
        >
          {w.win.openPr}
        </button>
        <span className="xs mut">{w.experiment.judgeNote}</span>
      </div>
    </section>
  );
}

import { Link } from "@tanstack/react-router";
import { useExperimentValue, useMembers } from "../../api/queries";
import { Empty, Failed, Loading } from "../../components/state/State";
import { dollars } from "../../lib/format";
import { fromTo, modelLine, returnLabel } from "../../lib/value";
import { w } from "../../words";
import { SourceText } from "../projects/ProjectValue";
import { WinState } from "../value/WinState";

const t = w.value;

/** The Value section of the detail page: the sealed value model, each win's state and worth, and Return. */
export function ExperimentValueBlock({ experimentId }: { experimentId: string }) {
  const value = useExperimentValue(experimentId);
  const members = useMembers();
  if (value.isPending) return <Loading rows={1} />;
  if (value.isError) return <Failed error={value.error} retry={() => void value.refetch()} />;
  const { valueModel, wins, figures } = value.data;
  const who = (id: string) => members.data?.find((member) => member.id === id)?.name ?? t.unknown;
  return (
    <>
      <div className="ds-b">
        <div className="dr">
          <span className="k">{t.model}</span>
          <span className="v">
            {valueModel == null ? (
              <>
                {t.noValueSet}
                <span className="m">{t.noModelOnExperimentHint}</span>
              </>
            ) : (
              <>
                {modelLine(valueModel.model)}
                <span className="m">
                  {t.sourceLabel}: <SourceText source={valueModel.model.source} /> · {t.setBy}{" "}
                  {who(valueModel.model.setBy)} · {t.sealedWith(valueModel.model.version)}
                </span>
              </>
            )}
          </span>
        </div>
        <div className="dr">
          <span className="k">{t.return}</span>
          <span className="v">
            <span>
              <span className="mono">{returnLabel(figures.returnX)}</span>
              {figures.returnX == null ? null : ` ${t.expReturn(dollars(figures.spentCents))}`}
            </span>
            <span className="m">
              {figures.returnX == null
                ? t.noSpendYet
                : `${t.overSpent(dollars(figures.deliveredCents), dollars(figures.spentCents))}, ${t.measuredOffline}`}
            </span>
          </span>
        </div>
      </div>
      {wins.length === 0 ? (
        <Empty title={t.winsNone} />
      ) : (
        <div
          className="tb"
          style={{
            ["--cols" as string]: "minmax(0, 1.6fr) 170px minmax(0, 1.2fr) 100px",
            marginTop: 12,
          }}
        >
          <div className="tb-r th">
            <span>{w.experimentDetail.wins}</span>
            <span>{t.cell.state}</span>
            <span>{t.cell.worth}</span>
            <span />
          </div>
          {wins.map((win) => (
            <div key={win.trialId} className="tb-r">
              <span className="n">
                <span>{t.winRow(win.version)}</span>
                <span className="m mono">{fromTo(win)}</span>
              </span>
              <span>
                <WinState state={win.state} />
              </span>
              <span className="n">
                {win.yearlyCents == null ? (
                  <span className="m">{t.noValueSet}</span>
                ) : (
                  <>
                    <span className="mono">{t.yearly(dollars(win.yearlyCents))}</span>
                    <span className="m">{t.measuredOffline}</span>
                  </>
                )}
              </span>
              <Link
                className="btn sm"
                to="/receipts/$trialId"
                params={{ trialId: win.trialId }}
                style={{ justifySelf: "end" }}
              >
                {w.experimentDetail.viewReceipt}
              </Link>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

import type { Schemas } from "../../api/client";
import { dollars } from "../../lib/format";
import { returnLabel } from "../../lib/value";
import { w } from "../../words";

const t = w.value;

/** Delivered, Pending, At risk and Return as the stat strip; a Spent tile goes first when asked for. */
export function ValueTiles(props: {
  figures: Schemas["ValueFigures"];
  label: string;
  /** The Report's first tile: Spent, with what went on experiments with no win. */
  spent?: { noWinCents: number };
}) {
  const f = props.figures;
  const spent = props.spent;
  return (
    <>
      <section className={`vs ${spent === undefined ? "c4" : "c5"}`} aria-label={props.label}>
        {spent === undefined ? null : (
          <div>
            <span className="k">{w.report.spent}</span>
            <span className="v">{dollars(f.spentCents)}</span>
            <span className="s">
              {f.spentCents === 0
                ? w.report.nothingSpent
                : spent.noWinCents === 0
                  ? w.report.allWithWin
                  : w.report.includesNoWin(dollars(spent.noWinCents))}
            </span>
          </div>
        )}
        <div>
          <span className="k">{t.delivered}</span>
          <span className="v">{dollars(f.deliveredCents)}</span>
          <span className="s">
            {t.aYear(`${t.mergedWins(f.mergedWins)}, ${t.measuredOffline}`)}
          </span>
        </div>
        <div>
          <span className="k">{t.pending}</span>
          <span className="v">{dollars(f.pendingCents)}</span>
          <span className="s">{t.aYear(t.openPrs(f.pendingWins))}</span>
        </div>
        <div>
          <span className="k">{t.atRisk}</span>
          <span className="v">{dollars(f.atRiskCents)}</span>
          <span className="s">
            {t.aYear(f.atRiskCents === 0 ? t.nothingRegressed : t.regressedSince)}
          </span>
        </div>
        <div>
          <span className="k">{t.return}</span>
          <span className="v">{returnLabel(f.returnX)}</span>
          <span className="s">
            {f.returnX == null
              ? t.noSpendYet
              : t.overSpent(dollars(f.deliveredCents), dollars(f.spentCents))}
          </span>
        </div>
      </section>
      {f.unvaluedWins === 0 ? null : <p className="target-note">{t.unvalued(f.unvaluedWins)}</p>}
    </>
  );
}

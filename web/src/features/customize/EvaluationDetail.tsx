import { Link } from "@tanstack/react-router";
import { Icon } from "../../components/icon/Icon";
import { Empty } from "../../components/state/State";
import { w } from "../../words";
import { evaluation } from "./stub";

export function EvaluationDetail({ id }: { id: string }) {
  const item = evaluation(id);
  if (item === undefined) {
    return (
      <section className="page">
        <Link className="textbtn" to="/customize/evaluations" style={{ alignSelf: "flex-start" }}>
          <Icon name="back" size="s" />
          {w.customize.back}
        </Link>
        <Empty title={w.customize.empty} />
      </section>
    );
  }
  return (
    <section className="page">
      <Link className="textbtn" to="/customize/evaluations" style={{ alignSelf: "flex-start" }}>
        <Icon name="back" size="s" />
        {w.customize.back}
      </Link>
      <div className="rw between wrap g16">
        <div className="rw g16" style={{ minWidth: 0, flex: "1 1 360px" }}>
          <span className="tile lg">
            <span className="gi g-gauge" />
          </span>
          <div className="col g4" style={{ minWidth: 0 }}>
            <h1 className="t-title">{item.name}</h1>
            <span className="ink2">{item.summary}.</span>
            <span
              className="rw g8 sm"
              style={{ color: item.sealChecksPassed ? "var(--good)" : "var(--warn)" }}
            >
              <i
                className="dot"
                style={{ background: item.sealChecksPassed ? "var(--good)" : "var(--warn)" }}
              />
              {item.sealChecksPassed ? w.customize.sealPassed : w.trial.notChecked}
            </span>
          </div>
        </div>
        <div className="rw g8 wrap">
          <button type="button" className="btn primary" disabled title={w.customize.target}>
            {w.customize.checkAgain}
          </button>
        </div>
      </div>
      <p className="target-note">{w.customize.target}</p>
      <h2 className="sech">{w.customize.theTemplate}</h2>
      <div className="card" style={{ padding: "4px 20px" }}>
        <div className="kv">
          <span>{w.customize.columns.kind}</span>
          <span>{w.customize.kinds[item.kind]}</span>
        </div>
        <div className="kv">
          <span>{w.trial.metric}</span>
          <span>{item.metric}</span>
        </div>
        <div className="kv">
          <span>{w.customize.addCommand}</span>
          <span>
            <span className="mono xs">{item.command}</span>
          </span>
        </div>
        <div className="kv">
          <span>{w.customize.addPaths}</span>
          <span>
            <span className="mono xs">{item.protectedPaths.join(", ")}</span>
          </span>
        </div>
      </div>
      <h2 className="sech">{w.customize.usedBy}</h2>
      <Empty title={w.customize.notUsed} />
    </section>
  );
}

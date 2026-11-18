import { Link } from "@tanstack/react-router";
import { ApiFailure } from "../../api/client";
import { useEvaluation } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading } from "../../components/state/State";
import { w } from "../../words";

export function EvaluationDetail({ id }: { id: string }) {
  const found = useEvaluation(id);
  const item = found.data;
  if (item === undefined) {
    return (
      <section className="page">
        <Link className="textbtn" to="/customize/evaluations" style={{ alignSelf: "flex-start" }}>
          <Icon name="back" size="s" />
          {w.customize.back}
        </Link>
        {found.isPending ? (
          <Loading />
        ) : found.isError && found.error instanceof ApiFailure && found.error.status !== 404 ? (
          <Failed error={found.error} retry={() => void found.refetch()} />
        ) : (
          <Empty title={w.customize.empty} />
        )}
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
          </div>
        </div>
      </div>
      <p className="sm mut">{w.customize.sealChecksLater}</p>
      <h2 className="sech">{w.customize.theTemplate}</h2>
      <div className="card" style={{ padding: "4px 20px" }}>
        <div className="kv">
          <span>{w.customize.columns.kind}</span>
          <span>{w.customize.kinds[item.kind]}</span>
        </div>
        <div className="kv">
          <span>{w.trial.metric}</span>
          <span>
            {item.metric}, {item.lowerIsBetter ? w.customize.lower : w.customize.higher}
          </span>
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
    </section>
  );
}

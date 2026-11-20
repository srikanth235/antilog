import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { useEvaluations } from "../../api/queries";
import { Glyph, Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading } from "../../components/state/State";
import { w } from "../../words";
import { Row, Table } from "../settings/Table";
import { Tabs } from "./Tabs";

export function Evaluations() {
  const [query, setQuery] = useState("");
  const evaluations = useEvaluations();
  const rows = (evaluations.data ?? []).filter((item) =>
    item.name.toLowerCase().includes(query.toLowerCase()),
  );
  const cols = "44px minmax(0, 2.3fr) 80px minmax(0, 1.4fr) 100px";
  return (
    <section className="page cust">
      <div className="hd">
        <h1 className="t-display">{w.customize.title}</h1>
      </div>
      <div className="ctl">
        <Tabs active="evaluations" />
        <span className="vdiv" aria-hidden="true" />
        <div className="segs">
          <span className="seg on" aria-current="page">
            {w.customize.yours}
          </span>
        </div>
        <div className="ctools">
          <label className="sfield">
            <Icon name="search" />
            <input
              placeholder={w.customize.search}
              aria-label={w.customize.search}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
          <Link className="btn primary sm" to="/customize/evaluations/new">
            <Icon name="plus" size="s" />
            {w.customize.add}
          </Link>
        </div>
      </div>
      {evaluations.isPending ? (
        <Loading />
      ) : evaluations.isError ? (
        <Failed error={evaluations.error} retry={() => void evaluations.refetch()} />
      ) : rows.length === 0 ? (
        <Empty title={w.customize.empty} hint={w.customize.emptyHint}>
          <Link className="btn primary" to="/customize/evaluations/new">
            {w.customize.add}
          </Link>
        </Empty>
      ) : (
        <Table
          columns={cols}
          min={720}
          label={w.customize.evaluations}
          head={[
            "",
            w.customize.columns.evaluation,
            w.customize.columns.kind,
            w.customize.columns.metric,
            "",
          ]}
        >
          {rows.map((item) => (
            <Row key={item.id} columns={cols}>
              <span className="tile">
                <span className="gi g-gauge" />
              </span>
              <span className="col" style={{ gap: 1, minWidth: 0 }}>
                <span className="t-item ell">{item.name}</span>
                <span className="xs mut ell">{item.summary}</span>
              </span>
              <span className="sm ink2">{w.customize.kinds[item.kind]}</span>
              <span className="sm ink2 ell">
                {item.metric}, {item.lowerIsBetter ? w.customize.lower : w.customize.higher}
              </span>
              <Link
                className="btn sm"
                to="/customize/evaluations/$id"
                params={{ id: item.id }}
                aria-label={`${w.customize.configure} ${item.name}`}
              >
                {w.customize.configure}
              </Link>
            </Row>
          ))}
        </Table>
      )}
      <span className="sr-only">
        <Glyph name="gauge" />
      </span>
    </section>
  );
}

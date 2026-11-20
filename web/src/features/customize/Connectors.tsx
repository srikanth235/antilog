import { Link } from "@tanstack/react-router";
import { useState } from "react";
import type { Schemas } from "../../api/client";
import { useConnectors } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading } from "../../components/state/State";
import { w } from "../../words";
import { Row, Table } from "../settings/Table";
import { ListShell } from "./ListShell";

const c = w.connector;

const HEALTH_COLOR: Record<Schemas["ConnectorHealth"], string> = {
  connected: "var(--good)",
  reconnect: "var(--warn)",
  not_checked: "var(--ink3)",
};

/** Word and dot together: an unchecked connector is never drawn as healthy. */
export function Health({ health }: { health: Schemas["ConnectorHealth"] }) {
  return (
    <span
      className="rw g8 sm"
      style={{ color: health === "connected" ? "var(--good)" : undefined }}
    >
      <i className="dot" style={{ background: HEALTH_COLOR[health] }} />
      {c.health[health]}
    </span>
  );
}

export function Connectors() {
  const [query, setQuery] = useState("");
  const connectors = useConnectors();
  const rows = (connectors.data ?? []).filter((item) =>
    item.name.toLowerCase().includes(query.toLowerCase()),
  );
  const cols = "44px minmax(0, 3fr) 140px 110px 100px";
  const add = (
    <Link className="btn primary sm" to="/customize/connectors/new">
      <Icon name="plus" size="s" />
      {c.add}
    </Link>
  );
  return (
    <ListShell
      active="connectors"
      search={c.search}
      query={query}
      onQuery={setQuery}
      action={add}
      lock={c.lock}
    >
      {connectors.isPending ? (
        <Loading />
      ) : connectors.isError ? (
        <Failed error={connectors.error} retry={() => void connectors.refetch()} />
      ) : rows.length === 0 ? (
        <Empty title={c.empty} hint={c.emptyHint}>
          <Link className="btn primary" to="/customize/connectors/new">
            {c.add}
          </Link>
        </Empty>
      ) : (
        <Table
          columns={cols}
          min={720}
          label={w.customize.connectors}
          head={["", c.columns.connector, c.columns.status, c.columns.tools, ""]}
        >
          {rows.map((item) => (
            <Row key={item.id} columns={cols}>
              <span className="tile">
                <span className="gi g-cloud" />
              </span>
              <span className="col" style={{ gap: 1, minWidth: 0 }}>
                <span className="t-item ell">{item.name}</span>
                <span className="xs mut ell">{item.url}</span>
              </span>
              <Health health={item.health} />
              <span className="sm ink2 ell">{c.tools(item.tools.length)}</span>
              <Link
                className="btn sm"
                to="/customize/connectors/$id"
                params={{ id: item.id }}
                aria-label={`${w.customize.configure} ${item.name}`}
              >
                {w.customize.configure}
              </Link>
            </Row>
          ))}
        </Table>
      )}
    </ListShell>
  );
}

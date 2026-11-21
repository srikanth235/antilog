import { Link } from "@tanstack/react-router";
import { useState } from "react";
import type { Schemas } from "../../api/client";
import { usePlugins } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading } from "../../components/state/State";
import { w } from "../../words";
import { Row, Table } from "../settings/Table";
import { ListShell } from "./ListShell";

const p = w.plugin;

/** What a plugin brought, in words: "2 skill packs, 1 connector". */
export function adds(parts: Schemas["PluginPart"][]): string {
  const count = (kind: Schemas["PartKind"]) => parts.filter((part) => part.kind === kind).length;
  const words = [
    [count("skill"), p.skills],
    [count("connector"), p.connectors],
  ] as const;
  const said = words.filter(([n]) => n > 0).map(([n, say]) => say(n));
  return said.length === 0 ? p.nothing : said.join(", ");
}

export function Plugins() {
  const [query, setQuery] = useState("");
  const plugins = usePlugins();
  const rows = (plugins.data ?? []).filter((item) =>
    item.name.toLowerCase().includes(query.toLowerCase()),
  );
  const cols = "44px minmax(0, 2.3fr) 80px minmax(0, 2fr) 100px";
  const add = (
    <Link className="btn primary sm" to="/customize/plugins/new">
      <Icon name="plus" size="s" />
      {p.add}
    </Link>
  );
  return (
    <ListShell
      active="plugins"
      search={p.search}
      query={query}
      onQuery={setQuery}
      action={add}
      lock={p.lock}
    >
      {plugins.isPending ? (
        <Loading />
      ) : plugins.isError ? (
        <Failed error={plugins.error} retry={() => void plugins.refetch()} />
      ) : rows.length === 0 ? (
        <Empty title={p.empty} hint={p.emptyHint}>
          <Link className="btn primary" to="/customize/plugins/new">
            {p.add}
          </Link>
        </Empty>
      ) : (
        <Table
          columns={cols}
          min={720}
          label={w.customize.plugins}
          head={["", p.columns.plugin, p.columns.version, p.columns.adds, ""]}
        >
          {rows.map((item) => (
            <Row key={item.id} columns={cols}>
              <span className="tile">
                <span className="gi g-blocks" />
              </span>
              <span className="col" style={{ gap: 1, minWidth: 0 }}>
                <span className="t-item ell">{item.name}</span>
                <span className="xs mut ell">{item.description}</span>
              </span>
              <span className="mono xs">{p.version(item.version)}</span>
              <span className="sm ink2 ell">{adds(item.parts)}</span>
              <Link
                className="btn sm"
                to="/customize/plugins/$id"
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

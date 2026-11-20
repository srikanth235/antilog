import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { useSkills } from "../../api/queries";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading } from "../../components/state/State";
import { w } from "../../words";
import { Row, Table } from "../settings/Table";
import { ListShell } from "./ListShell";

const s = w.skill;

export function Skills() {
  const [query, setQuery] = useState("");
  const skills = useSkills();
  const rows = (skills.data ?? []).filter((item) =>
    item.name.toLowerCase().includes(query.toLowerCase()),
  );
  const cols = "44px minmax(0, 3fr) 80px minmax(0, 1.4fr) 100px";
  const add = (
    <Link className="btn primary sm" to="/customize/skills/new">
      <Icon name="plus" size="s" />
      {s.add}
    </Link>
  );
  return (
    <ListShell
      active="skills"
      search={s.search}
      query={query}
      onQuery={setQuery}
      action={add}
      lock={s.lock}
    >
      {skills.isPending ? (
        <Loading />
      ) : skills.isError ? (
        <Failed error={skills.error} retry={() => void skills.refetch()} />
      ) : rows.length === 0 ? (
        <Empty title={s.empty} hint={s.emptyHint}>
          <Link className="btn primary" to="/customize/skills/new">
            {s.add}
          </Link>
        </Empty>
      ) : (
        <Table
          columns={cols}
          min={720}
          label={w.customize.skills}
          head={["", s.columns.skill, s.columns.version, s.columns.from, ""]}
        >
          {rows.map((item) => (
            <Row key={item.name} columns={cols}>
              <span className="tile">
                <span className="gi g-book" />
              </span>
              <span className="col" style={{ gap: 1, minWidth: 0 }}>
                <span className="t-item ell">{item.name}</span>
                <span className="xs mut ell">{item.description}</span>
              </span>
              <span className="mono xs">{s.version(item.latest)}</span>
              <span className="sm ink2 ell">
                {item.fromPlugin == null ? s.uploaded : s.fromPlugin(item.fromPlugin)}
              </span>
              <Link
                className="btn sm"
                to="/customize/skills/$name"
                params={{ name: item.name }}
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

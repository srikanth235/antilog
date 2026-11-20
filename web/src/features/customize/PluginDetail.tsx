import { Link, useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { ApiFailure, type Schemas } from "../../api/client";
import {
  useConnectors,
  useEvaluations,
  useMember,
  usePlugins,
  useRemovePlugin,
} from "../../api/queries";
import { Dialog } from "../../components/dialog/Dialog";
import { Icon } from "../../components/icon/Icon";
import { Empty, Failed, Loading, reason } from "../../components/state/State";
import { canEdit } from "../../lib/domain";
import { w } from "../../words";
import { Health } from "./Connectors";
import { DetailHead, Section, TwoColumns } from "./ListShell";

const p = w.plugin;

function Back() {
  return (
    <Link className="textbtn" to="/customize/plugins" style={{ alignSelf: "flex-start" }}>
      <Icon name="back" size="s" />
      {p.back}
    </Link>
  );
}

export function PluginDetail({ id }: { id: string }) {
  const plugins = usePlugins();
  const item = plugins.data?.find((plugin) => plugin.id === id);
  if (item === undefined) {
    return (
      <section className="page">
        <Back />
        {plugins.isPending ? (
          <Loading />
        ) : plugins.isError &&
          plugins.error instanceof ApiFailure &&
          plugins.error.status !== 404 ? (
          <Failed error={plugins.error} retry={() => void plugins.refetch()} />
        ) : (
          <Empty title={p.empty} />
        )}
      </section>
    );
  }
  return <Loaded item={item} />;
}

function PartRows(props: {
  rows: {
    key: string;
    glyph: string;
    name: string;
    note: React.ReactNode;
    link: React.ReactNode;
  }[];
}) {
  return (
    <div className="table">
      <div style={{ minWidth: 380 }}>
        {props.rows.map((row) => (
          <div
            key={row.key}
            className="grid"
            style={{ gridTemplateColumns: "44px minmax(0, 1fr) auto" }}
          >
            <span className="tile">
              <span className={`gi g-${row.glyph}`} />
            </span>
            <span className="t-item ell">{row.link}</span>
            <span className="sm ink2">{row.note}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function Loaded({ item }: { item: Schemas["Plugin"] }) {
  const me = useMember();
  const editable = canEdit(me?.role);
  const connectors = useConnectors();
  const evaluations = useEvaluations();
  const [removing, setRemoving] = useState(false);
  const part = (kind: Schemas["PartKind"]) => item.parts.filter((x) => x.kind === kind);
  const skills = part("skill");
  const connectorParts = part("connector");
  const evaluationParts = part("evaluation");
  return (
    <section className="page">
      <Back />
      <DetailHead
        glyph="blocks"
        title={item.name}
        subtitle={item.description}
        state={
          <span className="rw g8 sm">
            <span className="mono xs">{p.version(item.version)}</span>
          </span>
        }
        actions={
          <button
            type="button"
            className="btn danger"
            disabled={!editable}
            onClick={() => setRemoving(true)}
          >
            {p.remove}
          </button>
        }
      />
      {editable ? null : <p className="sm mut">{w.readOnly.onlyEditors}</p>}
      <TwoColumns
        left={
          <>
            <Section title={p.skillsTitle(skills.length)}>
              <PartRows
                rows={skills.map((x) => ({
                  key: x.name,
                  glyph: "book",
                  name: x.name,
                  note: "",
                  link: (
                    <Link to="/customize/skills/$name" params={{ name: x.name }}>
                      {x.name}
                    </Link>
                  ),
                }))}
              />
            </Section>
            <Section title={p.connectorsTitle(connectorParts.length)}>
              <PartRows
                rows={connectorParts.map((x) => {
                  const found = connectors.data?.find((connector) => connector.id === x.name);
                  return {
                    key: x.name,
                    glyph: "cloud",
                    name: found?.name ?? x.name,
                    note: found === undefined ? p.gone : <Health health={found.health} />,
                    link:
                      found === undefined ? (
                        p.gone
                      ) : (
                        <Link to="/customize/connectors/$id" params={{ id: found.id }}>
                          {found.name}
                        </Link>
                      ),
                  };
                })}
              />
            </Section>
          </>
        }
        right={
          <>
            <Section title={p.evaluationsTitle(evaluationParts.length)}>
              <PartRows
                rows={evaluationParts.map((x) => {
                  const found = evaluations.data?.find((template) => template.id === x.name);
                  return {
                    key: x.name,
                    glyph: "gauge",
                    name: found?.name ?? x.name,
                    note: found === undefined ? "" : w.customize.kinds[found.kind],
                    link:
                      found === undefined ? (
                        x.name
                      ) : (
                        <Link to="/customize/evaluations/$id" params={{ id: found.id }}>
                          {found.name}
                        </Link>
                      ),
                  };
                })}
              />
              <span className="xs mut">{p.evaluationsStay}</span>
            </Section>
            <Section title={p.about}>
              <div className="card" style={{ padding: "4px 20px" }}>
                <div className="kv">
                  <span>{p.versionLabel}</span>
                  <span>{item.version}</span>
                </div>
              </div>
            </Section>
            <span className="lock">
              <Icon name="lock" size="s" />
              {p.holdout}
            </span>
          </>
        }
      />
      <RemoveDialog
        item={item}
        open={removing}
        onClose={() => setRemoving(false)}
        connectorName={(id) => connectors.data?.find((x) => x.id === id)?.name ?? id}
      />
    </section>
  );
}

function RemoveDialog(props: {
  item: Schemas["Plugin"];
  open: boolean;
  onClose: () => void;
  connectorName: (id: string) => string;
}) {
  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={p.removeTitle(props.item.name)}
      description={p.removeLead(props.item.name)}
    >
      <RemoveForm {...props} />
    </Dialog>
  );
}

function RemoveForm(props: {
  item: Schemas["Plugin"];
  onClose: () => void;
  connectorName: (id: string) => string;
}) {
  const remove = useRemovePlugin(props.item.id);
  const navigate = useNavigate();
  const [keep, setKeep] = useState<Set<string>>(new Set());
  const id = (x: Schemas["PluginPart"]) => `${x.kind}:${x.name}`;
  const toggle = (x: Schemas["PluginPart"]) =>
    setKeep((now) => {
      const next = new Set(now);
      if (!next.delete(id(x))) next.add(id(x));
      return next;
    });
  const group = (kind: "skill" | "connector", title: string) => {
    const parts = props.item.parts.filter((x) => x.kind === kind);
    if (parts.length === 0) return null;
    return (
      <fieldset className="col g4" style={{ border: 0, padding: 0, margin: 0 }}>
        <legend className="t-label">{title}</legend>
        {parts.map((x) => {
          const label = p.keep(kind === "skill" ? x.name : props.connectorName(x.name));
          return (
            <label key={id(x)} className="rw g8" style={{ minHeight: 32 }}>
              <input type="checkbox" checked={keep.has(id(x))} onChange={() => toggle(x)} />
              <span>{label}</span>
            </label>
          );
        })}
      </fieldset>
    );
  };
  const templates = props.item.parts.filter((x) => x.kind === "evaluation").length;
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    await remove.mutateAsync(props.item.parts.filter((x) => keep.has(id(x))));
    props.onClose();
    await navigate({ to: "/customize/plugins" });
  };
  return (
    <form className="form-col" onSubmit={(event) => void submit(event).catch(() => undefined)}>
      {group("skill", p.skillsGroup)}
      {group("connector", p.connectorsGroup)}
      {templates === 0 ? null : <span className="sm ink2">{p.templatesStay(templates)}</span>}
      {remove.isError ? (
        <p className="error-line" role="alert">
          {reason(remove.error)}
        </p>
      ) : null}
      <div className="rw g12">
        <button type="submit" className="btn primary" disabled={remove.isPending}>
          {p.removeConfirm}
        </button>
        <button type="button" className="btn" onClick={props.onClose}>
          {p.removeKeep}
        </button>
      </div>
    </form>
  );
}

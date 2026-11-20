import { Link, useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { ApiFailure, type Schemas } from "../../api/client";
import {
  useCheckConnector,
  useConnector,
  useMember,
  useRemoveConnector,
  useSetConnectorCredential,
  useSetToolMode,
} from "../../api/queries";
import { Confirm, Dialog } from "../../components/dialog/Dialog";
import { Icon } from "../../components/icon/Icon";
import { Select } from "../../components/select/Select";
import { Empty, Failed, Loading, reason } from "../../components/state/State";
import { canEdit } from "../../lib/domain";
import { w } from "../../words";
import { Row, Table } from "../settings/Table";
import { Health } from "./Connectors";
import { DetailHead, Section, TwoColumns } from "./ListShell";

const c = w.connector;

function Back() {
  return (
    <Link className="textbtn" to="/customize/connectors" style={{ alignSelf: "flex-start" }}>
      <Icon name="back" size="s" />
      {c.back}
    </Link>
  );
}

export function ConnectorDetail({ id }: { id: string }) {
  const found = useConnector(id);
  if (found.data === undefined) {
    return (
      <section className="page">
        <Back />
        {found.isPending ? (
          <Loading />
        ) : found.isError && found.error instanceof ApiFailure && found.error.status !== 404 ? (
          <Failed error={found.error} retry={() => void found.refetch()} />
        ) : (
          <Empty title={c.empty} />
        )}
      </section>
    );
  }
  return <Loaded item={found.data} />;
}

function Loaded({ item }: { item: Schemas["Connector"] }) {
  const me = useMember();
  const editable = canEdit(me?.role);
  const navigate = useNavigate();
  const check = useCheckConnector(item.id);
  const remove = useRemoveConnector(item.id);
  const mode = useSetToolMode(item.id);
  const [confirming, setConfirming] = useState(false);
  const [updating, setUpdating] = useState(false);
  const cols = "minmax(0, 1fr) 170px";
  const modes = (["on", "off", "ask"] as const).map((value) => ({
    value,
    label: c.modes[value],
  }));
  const failure = [check, remove, mode].find((call) => call.isError)?.error;
  return (
    <section className="page">
      <Back />
      <DetailHead
        glyph="cloud"
        title={item.name}
        subtitle={c.subtitle}
        state={<Health health={item.health} />}
        actions={
          <>
            <button
              type="button"
              className="btn sm"
              disabled={!editable || check.isPending}
              onClick={() => check.mutate()}
            >
              {c.checkAgain}
            </button>
            <button
              type="button"
              className="btn sm primary"
              disabled={!editable}
              onClick={() => setUpdating(true)}
            >
              {c.updateCredential}
            </button>
            <button
              type="button"
              className="btn sm danger"
              disabled={!editable}
              aria-label={`${c.remove} ${item.name}`}
              onClick={() => setConfirming(true)}
            >
              {c.remove}
            </button>
          </>
        }
      />
      {editable ? null : <p className="sm mut">{w.readOnly.onlyEditors}</p>}
      {item.health === "reconnect" ? (
        <div className="sd-ro" role="alert" style={{ borderColor: "var(--warn)", marginTop: 0 }}>
          <i className="dot" style={{ background: "var(--warn)" }} />
          <span className="col grow" style={{ gap: 2 }}>
            <span style={{ color: "var(--ink)", fontWeight: 500 }}>
              {c.reconnectTitle(item.name)}
            </span>
            <span>{c.reconnectText}</span>
          </span>
        </div>
      ) : null}
      {failure === undefined ? null : (
        <p className="error-line" role="alert">
          {reason(failure)}
        </p>
      )}
      <TwoColumns
        left={
          <Section title={c.toolsTitle}>
            {item.tools.length === 0 ? (
              <Empty title={c.noTools} />
            ) : (
              <Table columns={cols} min={420} label={c.toolsTitle} head={[c.tool, c.mode]}>
                {item.tools.map((tool) => (
                  <Row key={tool.name} columns={cols}>
                    <span className="col" style={{ gap: 1, minWidth: 0 }}>
                      <span className="t-item ell mono">{tool.name}</span>
                      <span className="xs mut ell">{tool.description}</span>
                    </span>
                    <Select
                      label={c.modeFor(tool.name)}
                      value={tool.mode}
                      options={modes}
                      disabled={!editable || mode.isPending}
                      onChange={(next) => mode.mutate({ tool: tool.name, mode: next })}
                    />
                  </Row>
                ))}
              </Table>
            )}
            <span className="xs mut">{c.toolsNote(item.slug)}</span>
          </Section>
        }
        right={
          <Section title={c.accessTitle}>
            <div className="card" style={{ padding: "4px 20px" }}>
              <div className="kv">
                <span>{c.address}</span>
                <span>
                  <span className="mono xs">{item.url}</span>
                </span>
              </div>
              <div className="kv">
                <span>{c.credential}</span>
                <span>{item.hasCredential ? c.credentialText : c.noCredential}</span>
              </div>
              <div className="kv">
                <span>{c.usedBy}</span>
                <span>{c.usedByText}</span>
              </div>
            </div>
          </Section>
        }
      />
      <Confirm
        open={confirming}
        onOpenChange={setConfirming}
        title={c.removeTitle(item.name)}
        description={c.removeText(item.name)}
        confirm={c.removeConfirm}
        keep={c.removeKeep}
        onConfirm={() => {
          setConfirming(false);
          remove
            .mutateAsync()
            .then(() => navigate({ to: "/customize/connectors" }))
            .catch(() => undefined);
        }}
      />
      <CredentialDialog id={item.id} open={updating} onClose={() => setUpdating(false)} />
    </section>
  );
}

function CredentialDialog(props: { id: string; open: boolean; onClose: () => void }) {
  return (
    <Dialog open={props.open} onClose={props.onClose} title={c.credentialTitle} size="narrow">
      <Form id={props.id} onClose={props.onClose} />
    </Dialog>
  );
}

function Form({ id, onClose }: { id: string; onClose: () => void }) {
  const set = useSetConnectorCredential(id);
  const [value, setValue] = useState("");
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    await set.mutateAsync(value);
    onClose();
  };
  return (
    <form className="form-col" onSubmit={(event) => void submit(event).catch(() => undefined)}>
      <label className="field">
        {c.credential}
        <input
          className="input mono"
          type="password"
          autoComplete="off"
          required
          value={value}
          onChange={(event) => setValue(event.target.value)}
        />
        <span className="xs mut">{c.credentialHint}</span>
      </label>
      {set.isError ? (
        <p className="error-line" role="alert">
          {reason(set.error)}
        </p>
      ) : null}
      <button type="submit" className="btn primary" disabled={set.isPending || value === ""}>
        {c.credentialSave}
      </button>
    </form>
  );
}

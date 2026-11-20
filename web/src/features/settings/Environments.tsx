import { useState } from "react";
import { ApiFailure, type Schemas } from "../../api/client";
import {
  useArchiveEnvironment,
  useCreateEnvironment,
  useEditEnvironment,
  useEnvironments,
  useMember,
  useMembers,
} from "../../api/queries";
import { Confirm } from "../../components/dialog/Dialog";
import { Icon } from "../../components/icon/Icon";
import { Select } from "../../components/select/Select";
import { Failed, Loading, reason } from "../../components/state/State";
import { useToast } from "../../components/toast/Toast";
import { canEdit } from "../../lib/domain";
import { date } from "../../lib/format";
import { w } from "../../words";

type Environment = Schemas["Environment"];
type Network = Schemas["NetworkLevel"];

const NETWORKS: readonly Network[] = ["none", "limited", "full"];
const CPUS = [1, 2, 4, 8, 16];
const MEMORY_MB = [4096, 8192, 16384, 32768, 65536];

/** `NAME=value` lines to a map; null when a line is not one. */
export function parseVariables(text: string): Record<string, string> | null {
  const variables: Record<string, string> = {};
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (line === "") continue;
    const at = line.indexOf("=");
    const name = at < 0 ? "" : line.slice(0, at).trim();
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) return null;
    variables[name] = line.slice(at + 1);
  }
  return variables;
}

function variableLines(variables: Record<string, string>): string {
  return Object.entries(variables)
    .map(([name, value]) => `${name}=${value}`)
    .join("\n");
}

/** Host names split on lines, spaces and commas. */
export function parseHosts(text: string): string[] {
  return text.split(/[\s,]+/).filter((host) => host !== "");
}

function sizeOptions(current: number, all: readonly number[]): number[] {
  return all.includes(current) ? [...all] : [...all, current].sort((a, b) => a - b);
}

function Form(props: { environment: Environment | null; onDone: () => void }) {
  const t = w.settings.environments;
  const { environment } = props;
  const create = useCreateEnvironment();
  const edit = useEditEnvironment();
  const toast = useToast();
  const [name, setName] = useState(environment?.name ?? "");
  const [network, setNetwork] = useState<Network>(environment?.network ?? "limited");
  const [hosts, setHosts] = useState((environment?.allowedHosts ?? []).join("\n"));
  const [registries, setRegistries] = useState(environment?.allowPackageManagers ?? true);
  const [variables, setVariables] = useState(variableLines(environment?.variables ?? {}));
  const [script, setScript] = useState(environment?.setupScript ?? "");
  const [cpus, setCpus] = useState(environment?.cpus ?? 2);
  const [memory, setMemory] = useState(environment?.memoryMb ?? 8192);
  const [error, setError] = useState<string | null>(null);
  const pending = create.isPending || edit.isPending;
  const next = (environment?.version ?? 0) + 1;

  const save = async () => {
    setError(null);
    const parsed = parseVariables(variables);
    if (parsed === null) {
      setError(t.variablesBad);
      return;
    }
    const body: Schemas["NewEnvironment"] = {
      name,
      network,
      allowedHosts: parseHosts(hosts),
      allowPackageManagers: registries,
      variables: parsed,
      setupScript: script,
      cpus,
      memoryMb: memory,
    };
    try {
      if (environment === null) {
        await create.mutateAsync(body);
        toast(t.added);
      } else {
        const made = await edit.mutateAsync({ id: environment.id, body });
        toast(t.saved(made.version));
      }
      props.onDone();
    } catch (caught) {
      setError(caught instanceof ApiFailure && caught.status === 409 ? t.exists : reason(caught));
    }
  };

  return (
    <form
      className="col g12"
      style={{ marginTop: 16 }}
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <label className="fld">
        <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
          {t.name}
        </span>
        <input
          className="in"
          required
          maxLength={200}
          aria-label={t.name}
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
      </label>
      <div className="fld">
        <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
          {t.network}
        </span>
        <div className="segs">
          {NETWORKS.map((level) => (
            <button
              key={level}
              type="button"
              className={level === network ? "seg on" : "seg"}
              aria-pressed={level === network}
              onClick={() => setNetwork(level)}
            >
              {t.networks[level]}
            </button>
          ))}
        </div>
        <span className="xs mut">{t.networkHint}</span>
      </div>
      {network === "limited" ? (
        <>
          <label className="fld">
            <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
              {t.hosts}
            </span>
            <textarea
              className="in mono"
              rows={3}
              aria-label={t.hosts}
              spellCheck={false}
              value={hosts}
              onChange={(event) => setHosts(event.target.value)}
            />
            <span className="xs mut">{t.hostsHint}</span>
          </label>
          <div className="card" style={{ padding: "2px 16px" }}>
            <div className="stg" style={{ padding: "8px 0" }}>
              <div className="tx" style={{ flexDirection: "row", alignItems: "baseline", gap: 10 }}>
                <span className="t-item">{t.registries}</span>
                <span className="dd">{t.registriesHint}</span>
              </div>
              <span className="ct">
                <button
                  type="button"
                  className={registries ? "sw on" : "sw"}
                  role="switch"
                  aria-checked={registries}
                  aria-label={t.registries}
                  onClick={() => setRegistries(!registries)}
                />
              </span>
            </div>
          </div>
        </>
      ) : null}
      <label className="fld">
        <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
          {t.variables}
        </span>
        <textarea
          className="in mono"
          rows={3}
          aria-label={t.variables}
          spellCheck={false}
          autoComplete="off"
          value={variables}
          onChange={(event) => setVariables(event.target.value)}
        />
        <span className="xs mut">{t.variablesHint}</span>
      </label>
      <p className="ap-sec">
        <Icon name="lock" size="s" />
        {t.variablesWarning}
      </p>
      <label className="fld">
        <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
          {t.script}
        </span>
        <textarea
          className="in mono"
          rows={4}
          aria-label={t.script}
          spellCheck={false}
          value={script}
          onChange={(event) => setScript(event.target.value)}
        />
        <span className="xs mut">{t.scriptHint}</span>
      </label>
      <div className="rw g12 wrap" style={{ alignItems: "flex-start" }}>
        <div className="col" style={{ flex: "1 1 220px" }}>
          <div className="fld">
            <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
              {t.cpus}
            </span>
            <Select
              label={t.cpus}
              value={String(cpus)}
              options={sizeOptions(cpus, CPUS).map((n) => ({
                value: String(n),
                label: t.cpuOption(n),
              }))}
              onChange={(value) => setCpus(Number(value))}
            />
          </div>
        </div>
        <div className="col" style={{ flex: "1 1 220px" }}>
          <div className="fld">
            <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
              {t.memory}
            </span>
            <Select
              label={t.memory}
              value={String(memory)}
              options={sizeOptions(memory, MEMORY_MB).map((mb) => ({
                value: String(mb),
                label: mb % 1024 === 0 ? `${mb / 1024} GiB` : `${mb} MB`,
              }))}
              onChange={(value) => setMemory(Number(value))}
            />
          </div>
        </div>
      </div>
      {error === null ? null : (
        <p className="error-line" role="alert">
          {error}
        </p>
      )}
      <div className="rw g12" style={{ marginTop: 8 }}>
        <button type="submit" className="btn primary" disabled={pending || name.trim() === ""}>
          {environment === null ? t.saveNew : t.saveVersion(next)}
        </button>
        <button type="button" className="btn" onClick={props.onDone}>
          {t.cancel}
        </button>
      </div>
    </form>
  );
}

/** Environments: what a session runs on. Everyone reads them; owners and editors add, change
 * (a new version) and archive. */
export function Environments() {
  const t = w.settings.environments;
  const environments = useEnvironments();
  const members = useMembers();
  const archive = useArchiveEnvironment();
  const toast = useToast();
  const me = useMember();
  const editor = canEdit(me?.role);
  // `false` is the list; `null` the form for a new environment; an environment, its change.
  const [editing, setEditing] = useState<Environment | null | false>(false);
  const [archiving, setArchiving] = useState<Environment | null>(null);

  const who = (id: string | null | undefined) =>
    members.data?.find((member) => member.id === id)?.name ?? "someone";
  const note = (row: Environment) => {
    if (row.builtIn) return `${t.builtIn} · ${t.version(row.version)}`;
    const when = date(row.createdAt);
    if (row.archived) return t.archivedOn(row.version, when);
    return row.version === 1
      ? t.madeBy(who(row.createdBy), when)
      : t.changedBy(row.version, who(row.createdBy), when);
  };

  if (editing !== false) {
    return (
      <div className="sd-sec">
        <h3 className="t-section">{editing === null ? t.newTitle : t.changeTitle(editing.name)}</h3>
        <p className="sd-lead">
          {editing === null ? t.newLead : t.changeLead(editing.version + 1)}
        </p>
        <Form environment={editing} onDone={() => setEditing(false)} />
      </div>
    );
  }

  return (
    <div className="sd-sec">
      <h3 className="t-section">{w.settings.sections.connections}</h3>
      <p className="sd-lead">{t.lead}</p>
      <div className="rw g12" style={{ alignItems: "baseline" }}>
        <h4 className="sd-gh grow">{t.heading}</h4>
        <button
          type="button"
          className="btn primary sm"
          disabled={!editor}
          onClick={() => setEditing(null)}
        >
          {t.add}
        </button>
      </div>
      {environments.isPending ? (
        <Loading rows={3} />
      ) : environments.isError ? (
        <Failed error={environments.error} retry={() => void environments.refetch()} />
      ) : (
        <div className="table qt">
          <div style={{ minWidth: 600 }}>
            {environments.data.map((row) => (
              <div
                key={row.id}
                className="grid"
                data-environment={row.id}
                style={{
                  gridTemplateColumns: "minmax(0, 1.4fr) minmax(0, 1.4fr) auto",
                  ...(row.archived ? { opacity: 0.6 } : {}),
                }}
              >
                <span className="col" style={{ gap: 1, minWidth: 0 }}>
                  <span className="t-item ell">{row.name}</span>
                  <span className="xs mut ell">
                    {t.summary(row.network, row.allowedHosts.length, row.allowPackageManagers)}
                    {" · "}
                    {t.size(row.cpus, row.memoryMb)}
                  </span>
                </span>
                <span className="xs mut">{note(row)}</span>
                <span className="rw g8">
                  {row.builtIn ? (
                    <span className="xs mut">{t.builtIn}</span>
                  ) : row.archived ? (
                    <span className="xs mut">{t.archived}</span>
                  ) : (
                    <>
                      <button
                        type="button"
                        className="btn sm"
                        disabled={!editor}
                        aria-label={`${t.edit} ${row.name}`}
                        onClick={() => setEditing(row)}
                      >
                        {t.edit}
                      </button>
                      <button
                        type="button"
                        className="btn sm danger"
                        disabled={!editor}
                        aria-label={`${t.archive} ${row.name}`}
                        onClick={() => setArchiving(row)}
                      >
                        {t.archive}
                      </button>
                    </>
                  )}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
      <p className="ap-sec" style={{ marginTop: 12 }}>
        <Icon name="lock" size="s" />
        {t.editorsOnly}
      </p>
      <Confirm
        open={archiving !== null}
        onOpenChange={(open) => {
          if (!open) setArchiving(null);
        }}
        title={archiving === null ? "" : t.archiveTitle(archiving.name)}
        description={t.archiveConfirm}
        confirm={t.archive}
        keep={t.keepIt}
        onConfirm={() => {
          if (archiving !== null)
            archive.mutate(archiving.id, {
              onSuccess: () => {
                setArchiving(null);
                toast(t.archivedToast);
              },
            });
        }}
      />
    </div>
  );
}

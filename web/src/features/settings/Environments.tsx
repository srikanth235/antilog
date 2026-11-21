import { type ReactNode, useState } from "react";
import { ApiFailure, type Schemas } from "../../api/client";
import {
  useArchiveEnvironment,
  useCreateEnvironment,
  useEditEnvironment,
  useEnvironments,
  useMember,
  useMembers,
} from "../../api/queries";
import { Confirm, Dialog } from "../../components/dialog/Dialog";
import { Glyph, Icon } from "../../components/icon/Icon";
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

function memoryLabel(mb: number): string {
  return mb % 1024 === 0 ? `${mb / 1024} GiB` : `${mb} MB`;
}

/** A network secret in the form: `value` is only what the person typed now. A stored value is
 * never sent to the browser, so a stored secret with no `value` keeps the one it has. */
type SecretRow = {
  host: string;
  header: string;
  prefix: string;
  value: string;
  stored: boolean;
};

type Section = "network" | "secrets" | "script" | "variables" | "size";

const REPO = /^[A-Za-z0-9._-]+\/[A-Za-z0-9._-]+$/;

/** One row of the dialog: its name and a summary, and a chevron that opens its editor. */
function Row(props: {
  name: string;
  summary: string;
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
}) {
  return (
    <>
      <button type="button" className="stg nv" aria-expanded={props.open} onClick={props.onToggle}>
        <span className="tx">
          <span className="t-item">{props.name}</span>
        </span>
        <span className="ct">
          <span className="val">{props.summary}</span>
          <Icon name="chev" size="s" />
        </span>
      </button>
      {props.open ? <div className="nvp">{props.children}</div> : null}
    </>
  );
}

function Form(props: { environment: Environment | null; onDone: () => void }) {
  const t = w.settings.environments;
  const { environment } = props;
  const create = useCreateEnvironment();
  const edit = useEditEnvironment();
  const archive = useArchiveEnvironment();
  const toast = useToast();
  const me = useMember();
  const [name, setName] = useState(environment?.name ?? "");
  const [repos, setRepos] = useState<string[]>(environment?.repos ?? []);
  const [repoDraft, setRepoDraft] = useState<string | null>(null);
  const [network, setNetwork] = useState<Network>(environment?.network ?? "limited");
  const [hosts, setHosts] = useState((environment?.allowedHosts ?? []).join("\n"));
  const [registries, setRegistries] = useState(environment?.allowPackageManagers ?? true);
  const [secrets, setSecrets] = useState<SecretRow[]>(
    (environment?.networkSecrets ?? []).map((secret) => ({
      host: secret.host,
      header: secret.header,
      prefix: secret.prefix ?? "",
      value: "",
      stored: true,
    })),
  );
  const [fresh, setFresh] = useState<SecretRow>({
    host: "",
    header: "Authorization",
    prefix: "Bearer",
    value: "",
    stored: false,
  });
  const [variables, setVariables] = useState(variableLines(environment?.variables ?? {}));
  const [script, setScript] = useState(environment?.setupScript ?? "");
  const [cpus, setCpus] = useState(environment?.cpus ?? 2);
  const [memory, setMemory] = useState(environment?.memoryMb ?? 8192);
  const [open, setOpen] = useState<Section | null>(null);
  const [archiving, setArchiving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pending = create.isPending || edit.isPending;
  const next = (environment?.version ?? 0) + 1;
  const toggle = (section: Section) => setOpen(open === section ? null : section);

  const parsedVariables = parseVariables(variables);
  const scriptLines = script.trim() === "" ? 0 : script.trimEnd().split("\n").length;

  const addRepo = () => {
    const repo = (repoDraft ?? "").trim();
    if (repo === "") {
      setRepoDraft(null);
      return;
    }
    if (!REPO.test(repo)) {
      setError(t.repoBad);
      return;
    }
    setError(null);
    if (!repos.includes(repo)) setRepos([...repos, repo]);
    setRepoDraft(null);
  };

  const addSecret = () => {
    setError(null);
    if (fresh.host.trim() === "" || fresh.value === "") {
      setError(t.secretNeedsValue);
      return;
    }
    setSecrets([...secrets, { ...fresh, host: fresh.host.trim() }]);
    setFresh({ ...fresh, host: "", value: "" });
  };

  const save = async () => {
    setError(null);
    if (parsedVariables === null) {
      setError(t.variablesBad);
      setOpen("variables");
      return;
    }
    const body: Schemas["NewEnvironment"] = {
      name,
      network,
      allowedHosts: parseHosts(hosts),
      allowPackageManagers: registries,
      variables: parsedVariables,
      setupScript: script,
      repos,
      networkSecrets: secrets.map((secret) => ({
        host: secret.host,
        header: secret.header,
        prefix: secret.prefix.trim() === "" ? null : secret.prefix.trim(),
        // A value is sent only when one was typed; otherwise the stored one is kept.
        ...(secret.value === "" ? {} : { value: secret.value }),
      })),
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

  const networkSummary = t.networks[network];

  return (
    <form
      className="col g12"
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <p className="sd-lead" style={{ margin: 0 }}>
        {environment === null ? t.newLead : t.changeLead(next)}
      </p>
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
          {t.repos}
        </span>
        <span className="xs mut">{t.reposHint}</span>
        {repos.length === 0 ? null : (
          <ul className="rl" aria-label={t.repos}>
            {repos.map((repo) => (
              <li key={repo} className="rli">
                <Glyph name="git" />
                <span className="mono grow ell">{repo}</span>
                <button
                  type="button"
                  className="ib"
                  aria-label={t.removeRepo(repo)}
                  onClick={() => setRepos(repos.filter((have) => have !== repo))}
                >
                  <Icon name="close" size="s" />
                </button>
              </li>
            ))}
          </ul>
        )}
        {repoDraft === null ? (
          <div>
            <button type="button" className="btn sm" onClick={() => setRepoDraft("")}>
              <Icon name="plus" size="s" />
              {t.addRepo}
            </button>
          </div>
        ) : (
          <div className="rw g8">
            <input
              className="in mono"
              aria-label={t.repoPlaceholder}
              placeholder={t.repoPlaceholder}
              spellCheck={false}
              value={repoDraft}
              onChange={(event) => setRepoDraft(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  addRepo();
                }
              }}
            />
            <button type="button" className="btn sm" onClick={addRepo}>
              {t.addRepo}
            </button>
          </div>
        )}
      </div>
      <div className="card" style={{ padding: "2px 16px" }}>
        <Row
          name={t.rows.network}
          summary={networkSummary}
          open={open === "network"}
          onToggle={() => toggle("network")}
        >
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
              <div className="stg" style={{ padding: "4px 0" }}>
                <div
                  className="tx"
                  style={{ flexDirection: "row", alignItems: "baseline", gap: 10 }}
                >
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
            </>
          ) : null}
        </Row>
        <Row
          name={t.rows.secrets}
          summary={secrets.length === 0 ? t.none : t.secretCount(secrets.length)}
          open={open === "secrets"}
          onToggle={() => toggle("secrets")}
        >
          <p className="xs mut" style={{ margin: 0 }}>
            {t.secretsLead}
          </p>
          {secrets.length === 0 ? null : (
            <ul className="rl" aria-label={t.rows.secrets}>
              {secrets.map((secret) => (
                <li key={`${secret.host}|${secret.header}`} className="rli">
                  <Icon name="lock" size="s" />
                  <span className="col grow" style={{ minWidth: 0 }}>
                    <span className="mono ell">{secret.host}</span>
                    <span className="xs mut ell">
                      {secret.header}
                      {secret.prefix === "" ? "" : `: ${secret.prefix}`} ••••••••
                    </span>
                  </span>
                  <input
                    className="in mono"
                    type="password"
                    style={{ maxWidth: 160 }}
                    placeholder={t.secretReplace}
                    aria-label={t.secretNewValue(secret.host)}
                    autoComplete="off"
                    spellCheck={false}
                    value={secret.value}
                    onChange={(event) =>
                      setSecrets(
                        secrets.map((row) =>
                          row === secret ? { ...row, value: event.target.value } : row,
                        ),
                      )
                    }
                  />
                  <button
                    type="button"
                    className="ib"
                    aria-label={t.secretRemove(secret.host)}
                    onClick={() => setSecrets(secrets.filter((row) => row !== secret))}
                  >
                    <Icon name="close" size="s" />
                  </button>
                </li>
              ))}
            </ul>
          )}
          <div className="col g8">
            <div className="rw g8 wrap">
              <label className="fld" style={{ flex: "2 1 160px" }}>
                <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
                  {t.secretHost}
                </span>
                <input
                  className="in mono"
                  aria-label={t.secretHost}
                  placeholder={t.secretHostPlaceholder}
                  spellCheck={false}
                  value={fresh.host}
                  onChange={(event) => setFresh({ ...fresh, host: event.target.value })}
                />
              </label>
              <label className="fld" style={{ flex: "1 1 120px" }}>
                <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
                  {t.secretHeader}
                </span>
                <input
                  className="in mono"
                  aria-label={t.secretHeader}
                  spellCheck={false}
                  value={fresh.header}
                  onChange={(event) => setFresh({ ...fresh, header: event.target.value })}
                />
              </label>
              <label className="fld" style={{ flex: "1 1 90px" }}>
                <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
                  {t.secretPrefix}
                </span>
                <input
                  className="in mono"
                  aria-label={t.secretPrefix}
                  spellCheck={false}
                  value={fresh.prefix}
                  onChange={(event) => setFresh({ ...fresh, prefix: event.target.value })}
                />
              </label>
            </div>
            <label className="fld">
              <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
                {t.secretValue}
              </span>
              <span className="insec">
                <Icon name="lock" size="s" />
                <input
                  className="in mono"
                  type="password"
                  aria-label={t.secretValue}
                  autoComplete="off"
                  spellCheck={false}
                  value={fresh.value}
                  onChange={(event) => setFresh({ ...fresh, value: event.target.value })}
                />
              </span>
            </label>
            <div>
              <button type="button" className="btn sm" onClick={addSecret}>
                <Icon name="plus" size="s" />
                {t.secretAdd}
              </button>
            </div>
          </div>
        </Row>
        <Row
          name={t.rows.script}
          summary={scriptLines === 0 ? t.none : t.lines(scriptLines)}
          open={open === "script"}
          onToggle={() => toggle("script")}
        >
          <label className="fld">
            <span className="sr-only">{t.script}</span>
            <textarea
              className="in mono"
              rows={6}
              aria-label={t.script}
              spellCheck={false}
              value={script}
              onChange={(event) => setScript(event.target.value)}
            />
            <span className="xs mut">{t.scriptHint}</span>
          </label>
        </Row>
        <Row
          name={t.rows.variables}
          summary={
            parsedVariables === null
              ? "?"
              : Object.keys(parsedVariables).length === 0
                ? t.none
                : t.variableCount(Object.keys(parsedVariables).length)
          }
          open={open === "variables"}
          onToggle={() => toggle("variables")}
        >
          <label className="fld">
            <span className="sr-only">{t.variables}</span>
            <textarea
              className="in mono"
              rows={5}
              aria-label={t.variables}
              spellCheck={false}
              autoComplete="off"
              value={variables}
              onChange={(event) => setVariables(event.target.value)}
            />
            <span className="xs mut">{t.variablesHint}</span>
          </label>
          <p className="ap-sec" style={{ margin: 0 }}>
            <Icon name="lock" size="s" />
            {t.variablesWarning}
          </p>
        </Row>
        <Row
          name={t.rows.size}
          summary={w.settings.environments.size(cpus, memory)}
          open={open === "size"}
          onToggle={() => toggle("size")}
        >
          <div className="rw g12 wrap" style={{ alignItems: "flex-start" }}>
            <div className="col" style={{ flex: "1 1 180px" }}>
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
            <div className="col" style={{ flex: "1 1 180px" }}>
              <div className="fld">
                <span className="cap" style={{ fontSize: 13, color: "var(--ink2)" }}>
                  {t.memory}
                </span>
                <Select
                  label={t.memory}
                  value={String(memory)}
                  options={sizeOptions(memory, MEMORY_MB).map((mb) => ({
                    value: String(mb),
                    label: memoryLabel(mb),
                  }))}
                  onChange={(value) => setMemory(Number(value))}
                />
              </div>
            </div>
          </div>
        </Row>
      </div>
      {error === null ? null : (
        <p className="error-line" role="alert">
          {error}
        </p>
      )}
      <div className="rw g12 sp" style={{ marginTop: 8 }}>
        {environment === null ? (
          <span />
        ) : (
          <button
            type="button"
            className="btn danger"
            disabled={!canEdit(me?.role)}
            onClick={() => setArchiving(true)}
          >
            {t.archiveEnvironment}
          </button>
        )}
        <span className="rw g12">
          <button type="button" className="btn" onClick={props.onDone}>
            {t.cancel}
          </button>
          <button type="submit" className="btn primary" disabled={pending || name.trim() === ""}>
            {environment === null ? t.saveNew : t.saveChanges}
          </button>
        </span>
      </div>
      <Confirm
        open={archiving}
        onOpenChange={setArchiving}
        title={environment === null ? "" : t.archiveTitle(environment.name)}
        description={t.archiveConfirm}
        confirm={t.archive}
        keep={t.keepIt}
        onConfirm={() => {
          if (environment !== null)
            archive.mutate(environment.id, {
              onSuccess: () => {
                setArchiving(false);
                toast(t.archivedToast);
                props.onDone();
              },
            });
        }}
      />
    </form>
  );
}

/** Environments: what a session runs on. Everyone reads them; owners and editors add, change
 * (a new version) and archive. */
export function Environments() {
  const t = w.settings.environments;
  const environments = useEnvironments();
  const members = useMembers();
  const me = useMember();
  const editor = canEdit(me?.role);
  // `false` is no dialog; `null` the dialog for a new environment; an environment, its edit.
  const [editing, setEditing] = useState<Environment | null | false>(false);

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
                    <button
                      type="button"
                      className="btn sm"
                      disabled={!editor}
                      aria-label={`${t.edit} ${row.name}`}
                      onClick={() => setEditing(row)}
                    >
                      {t.edit}
                    </button>
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
      <Dialog
        open={editing !== false}
        onClose={() => setEditing(false)}
        title={
          editing === null ? w.settings.environments.addTitle : w.settings.environments.editTitle
        }
        width={600}
      >
        {editing === false ? null : (
          <Form
            key={editing === null ? "new" : `${editing.id}-${editing.version}`}
            environment={editing}
            onDone={() => setEditing(false)}
          />
        )}
      </Dialog>
    </div>
  );
}

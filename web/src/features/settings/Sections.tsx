import { useState } from "react";
import type { Schemas } from "../../api/client";
import {
  useAddMember,
  useAudit,
  useBudgets,
  useCreateToken,
  useMember,
  useMembers,
  useOrganization,
  usePolicies,
  useProjects,
  useProviders,
  useRevokeToken,
  useSetMemberRole,
  useSetOrganizationLimits,
  useSetPolicy,
  useTokens,
} from "../../api/queries";
import { Confirm, Dialog } from "../../components/dialog/Dialog";
import { Icon } from "../../components/icon/Icon";
import { ROLE_OPTIONS } from "../../components/select/roles";
import { Select } from "../../components/select/Select";
import { Empty, Failed, Loading, reason } from "../../components/state/State";
import { useToast } from "../../components/toast/Toast";
import { setAppearance, useAppearance } from "../../lib/appearance";
import { isOwner } from "../../lib/domain";
import { date, dateTime, dollars, initial, parseDollars, share } from "../../lib/format";
import { w } from "../../words";
import { ReadOnly, Row, Table } from "./Table";

export function Profile() {
  const me = useMember();
  const organization = useOrganization();
  const p = w.settings.profile;
  return (
    <div className="sd-sec">
      <h3 className="t-section">{w.settings.sections.profile}</h3>
      <p className="sd-lead">{p.lead}</p>
      <dl className="kvl">
        <Pair k={p.name} v={me?.name ?? ""} />
        <Pair k={p.role} v={me === undefined ? "" : w.roles[me.role]} />
        <Pair k={p.signedInWith} v={me?.identity?.provider ?? ""} />
        <Pair k={p.organization} v={organization.data?.name ?? ""} />
      </dl>
    </div>
  );
}

function Pair({ k, v }: { k: string; v: string }) {
  return (
    <div className="stg">
      <div className="tx">
        <span className="t-item">{k}</span>
      </div>
      <div className="ct">
        <span>{v}</span>
      </div>
    </div>
  );
}

export function Appearance() {
  const current = useAppearance();
  const a = w.settings.appearance;
  return (
    <div className="sd-sec">
      <h3 className="t-section">{w.settings.sections.appearance}</h3>
      <p className="sd-lead">{a.lead}</p>
      <div className="segs" style={{ marginTop: 12, alignSelf: "flex-start" }}>
        {(["system", "light", "dark"] as const).map((choice) => (
          <button
            key={choice}
            type="button"
            className={`seg${current === choice ? " on" : ""}`}
            aria-pressed={current === choice}
            onClick={() => setAppearance(choice)}
          >
            {a[choice]}
          </button>
        ))}
      </div>
    </div>
  );
}

export function Tokens() {
  const tokens = useTokens();
  const create = useCreateToken();
  const revoke = useRevokeToken();
  const toast = useToast();
  const me = useMember();
  const [name, setName] = useState("");
  const [shown, setShown] = useState<string | null>(null);
  const [revoking, setRevoking] = useState<Schemas["AccessToken"] | null>(null);
  const t = w.settings.tokens;
  const cols = "minmax(0, 1.6fr) minmax(0, 1fr) 100px";
  const list = (tokens.data ?? []).filter((token) => token.revokedAt == null);

  const submit = async () => {
    if (name.trim() === "") return;
    const issued = await create.mutateAsync({ name: name.trim() });
    setShown(issued.secret);
    setName("");
  };

  return (
    <div className="sd-sec">
      <h3 className="t-section">{w.settings.sections.tokens}</h3>
      <p className="sd-lead">{t.lead}</p>
      <form
        className="rw g12"
        style={{ marginTop: 12 }}
        onSubmit={(event) => {
          event.preventDefault();
          void submit().catch(() => undefined);
        }}
      >
        <label className="field grow">
          {t.name}
          <input className="input" value={name} onChange={(event) => setName(event.target.value)} />
        </label>
        <button
          type="submit"
          className="btn primary sm"
          style={{ alignSelf: "flex-end" }}
          disabled={create.isPending || name.trim() === ""}
        >
          {t.create}
        </button>
      </form>
      {create.isError ? (
        <p className="error-line" role="alert">
          {reason(create.error)}
        </p>
      ) : null}
      {shown === null ? null : (
        <div className="sd-tok" role="status">
          <span className="t-label" style={{ color: "var(--accent)" }}>
            {t.shownOnce}
          </span>
          <div className="rw between g12 wrap">
            <span
              className="mono"
              style={{ fontSize: 14, overflowWrap: "anywhere" }}
              data-testid="token-secret"
            >
              {shown}
            </span>
            <button
              type="button"
              className="btn sm"
              onClick={() =>
                void navigator.clipboard
                  .writeText(shown)
                  .then(() => toast(t.copied))
                  .catch(() => undefined)
              }
            >
              {t.copy}
            </button>
          </div>
        </div>
      )}
      {tokens.isPending ? (
        <Loading rows={2} />
      ) : tokens.isError ? (
        <Failed error={tokens.error} retry={() => void tokens.refetch()} />
      ) : list.length === 0 && shown === null ? (
        <Empty title={t.empty} hint={t.emptyHint} />
      ) : (
        <Table columns={cols} head={[t.name, t.created, ""]} label={w.settings.sections.tokens}>
          {list.map((token) => (
            <Row key={token.id} columns={cols}>
              <span className="rw g12" style={{ minWidth: 0 }}>
                <span className="sd-key" aria-hidden="true">
                  <i className="ic i-key" />
                </span>
                <span className="t-item ell">{token.name}</span>
              </span>
              <span className="sm ink2">
                {date(token.createdAt)}
                {token.memberId === me?.id ? "" : ` · ${t.service}`}
              </span>
              <button
                type="button"
                className="btn sm danger"
                aria-label={`${t.revoke} ${token.name}`}
                onClick={() => setRevoking(token)}
              >
                {t.revoke}
              </button>
            </Row>
          ))}
        </Table>
      )}
      <Confirm
        open={revoking !== null}
        onOpenChange={(open) => {
          if (!open) setRevoking(null);
        }}
        title={t.confirmTitle}
        description={revoking === null ? "" : t.confirm(revoking.name)}
        confirm={t.revoke}
        keep={t.keepIt}
        onConfirm={() => {
          if (revoking !== null) revoke.mutate(revoking.id, { onSuccess: () => setRevoking(null) });
        }}
      />
    </div>
  );
}

export function General() {
  const organization = useOrganization();
  const set = useSetOrganizationLimits();
  const me = useMember();
  const toast = useToast();
  const g = w.settings.general;
  const o = organization.data;
  if (o === undefined) return <Loading rows={2} />;
  return (
    <GeneralForm
      key={o.id}
      organization={o}
      owner={isOwner(me?.role)}
      set={set}
      onSaved={() => toast(g.saved)}
      title={g}
    />
  );
}

function GeneralForm(props: {
  organization: Schemas["Organization"];
  owner: boolean;
  set: ReturnType<typeof useSetOrganizationLimits>;
  onSaved: () => void;
  title: typeof w.settings.general;
}) {
  const g = props.title;
  const o = props.organization;
  const [cap, setCap] = useState(o.monthlyCapCents == null ? "" : String(o.monthlyCapCents / 100));
  const [def, setDef] = useState(
    o.defaultBudgetCents == null ? "" : String(o.defaultBudgetCents / 100),
  );
  const bad = (text: string) => text.trim() !== "" && parseDollars(text) === null;
  return (
    <div className="sd-sec">
      <h3 className="t-section">{w.settings.sections.general}</h3>
      <p className="sd-lead">{g.lead}</p>
      {props.owner ? null : <ReadOnly text={w.readOnly.onlyOwners} />}
      <dl className="kvl">
        <Pair k={g.name} v={o.name} />
        <Pair k={g.created} v={date(o.createdAt)} />
      </dl>
      <form
        className="form-col"
        style={{ marginTop: 16 }}
        onSubmit={(event) => {
          event.preventDefault();
          props.set.mutate(
            {
              monthlyCapCents: cap.trim() === "" ? null : parseDollars(cap),
              defaultBudgetCents: def.trim() === "" ? null : parseDollars(def),
            },
            { onSuccess: props.onSaved },
          );
        }}
      >
        <label className="field">
          {g.monthlyCap}
          <input
            className="input"
            inputMode="decimal"
            disabled={!props.owner}
            value={cap}
            onChange={(event) => setCap(event.target.value)}
          />
          <span className="xs mut">
            {g.noCap}: {w.projects.clearCap}
          </span>
        </label>
        <label className="field">
          {g.defaultBudget}
          <input
            className="input"
            inputMode="decimal"
            disabled={!props.owner}
            value={def}
            onChange={(event) => setDef(event.target.value)}
          />
        </label>
        {props.set.isError ? (
          <p className="error-line" role="alert">
            {reason(props.set.error)}
          </p>
        ) : null}
        <button
          type="submit"
          className="btn primary sm"
          style={{ alignSelf: "flex-start" }}
          disabled={!props.owner || props.set.isPending || bad(cap) || bad(def)}
        >
          {g.save}
        </button>
      </form>
    </div>
  );
}

function providerName(id: string): string {
  return w.signIn.providerNames[id] ?? id;
}

export function Members() {
  const members = useMembers();
  const add = useAddMember();
  const setRole = useSetMemberRole();
  const me = useMember();
  const owner = isOwner(me?.role);
  const m = w.settings.members;
  const [name, setName] = useState("");
  const [subject, setSubject] = useState("");
  const providers = useProviders();
  const offered = providers.data ?? [];
  const [chosenProvider, setProvider] = useState<string | null>(null);
  const provider = chosenProvider ?? offered[0] ?? "";
  const [adding, setAdding] = useState(false);
  const [role, setNewRole] = useState<Schemas["Role"]>("editor");
  const cols = "minmax(0, 1.6fr) minmax(0, 1.2fr) 130px";

  return (
    <div className="sd-sec">
      <h3 className="t-section">{w.settings.sections.members}</h3>
      <p className="sd-lead">{m.lead}</p>
      {owner ? null : <ReadOnly text={w.readOnly.onlyOwners} />}
      {members.isPending ? (
        <Loading rows={2} />
      ) : members.isError ? (
        <Failed error={members.error} retry={() => void members.refetch()} />
      ) : (
        <Table
          columns={cols}
          head={[m.name, m.provider, m.role]}
          label={w.settings.sections.members}
        >
          {(members.data ?? []).map((member) => (
            <Row key={member.id} columns={cols}>
              <span className="rw g12" style={{ minWidth: 0 }}>
                <span className="avatar round" aria-hidden="true">
                  {initial(member.name)}
                </span>
                <span className="col" style={{ gap: 1, minWidth: 0 }}>
                  <span className="t-item ell">
                    {member.name}
                    {member.id === me?.id ? ` (${m.you})` : ""}
                  </span>
                  <span className="xs mut ell">
                    {member.kind === "service" ? m.service : m.person}
                  </span>
                </span>
              </span>
              <span className="xs mut ell">{member.identity?.provider ?? ""}</span>
              <Select
                label={`${m.role}: ${member.name}`}
                value={member.role}
                options={ROLE_OPTIONS}
                disabled={!owner}
                onChange={(next) => setRole.mutate({ id: member.id, role: next })}
              />
            </Row>
          ))}
        </Table>
      )}
      {setRole.isError ? (
        <p className="error-line" role="alert">
          {reason(setRole.error)}
        </p>
      ) : null}
      {owner ? (
        <div className="rw g12" style={{ marginTop: 16 }}>
          <button type="button" className="btn sm" onClick={() => setAdding(true)}>
            <Icon name="plus" size="s" />
            {m.add}
          </button>
        </div>
      ) : null}
      <Dialog open={adding} onClose={() => setAdding(false)} title={m.add} size="narrow">
        <form
          className="form-col"
          onSubmit={(event) => {
            event.preventDefault();
            add.mutate(
              {
                name,
                role,
                identity: subject.trim() === "" ? null : { provider, subject: subject.trim() },
              },
              {
                onSuccess: () => {
                  setName("");
                  setSubject("");
                  setAdding(false);
                },
              },
            );
          }}
        >
          <label className="field">
            {m.name}
            <input
              className="input"
              required
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </label>
          {offered.length === 0 ? null : (
            <div className="field">
              <span>{m.provider}</span>
              <Select
                label={m.provider}
                value={provider}
                options={offered.map((id) => ({ value: id, label: providerName(id) }))}
                onChange={setProvider}
              />
            </div>
          )}
          <label className="field">
            {m.subject}
            <input
              className="input"
              value={subject}
              onChange={(event) => setSubject(event.target.value)}
            />
          </label>
          <div className="field">
            <span>{m.role}</span>
            <Select label={m.role} value={role} options={ROLE_OPTIONS} onChange={setNewRole} />
          </div>
          {add.isError ? (
            <p className="error-line" role="alert">
              {reason(add.error)}
            </p>
          ) : null}
          <div className="rw g12">
            <button
              type="submit"
              className="btn primary sm"
              disabled={add.isPending || name.trim() === ""}
            >
              {m.add}
            </button>
            <button type="button" className="btn sm" onClick={() => setAdding(false)}>
              {w.cancel}
            </button>
          </div>
        </form>
      </Dialog>
    </div>
  );
}

export function Approvals() {
  const policies = usePolicies();
  const set = useSetPolicy();
  const me = useMember();
  const owner = isOwner(me?.role);
  const a = w.settings.approvals;
  const cols = "minmax(0, 1.4fr) 160px 120px";
  const modes = (["always_ask", "allow_up_to", "allow"] as const).map((value) => ({
    value,
    label: a.modes[value],
  }));
  return (
    <div className="sd-sec">
      <h3 className="t-section">{w.settings.sections.approvals}</h3>
      <p className="sd-lead">{a.lead}</p>
      {owner ? null : <ReadOnly text={w.readOnly.onlyOwners} />}
      {policies.isPending ? (
        <Loading rows={3} />
      ) : policies.isError ? (
        <Failed error={policies.error} retry={() => void policies.refetch()} />
      ) : (
        <Table
          columns={cols}
          head={[a.card, a.mode, a.limit]}
          label={w.settings.sections.approvals}
        >
          {(policies.data ?? []).map((policy) => (
            <PolicyRow
              key={policy.card}
              policy={policy}
              cols={cols}
              modes={modes}
              disabled={!owner || policy.card === "add_credential"}
              onChange={(next) => set.mutate(next)}
            />
          ))}
        </Table>
      )}
      <p className="target-note">{a.addCredentialNote}</p>
      {set.isError ? (
        <p className="error-line" role="alert">
          {reason(set.error)}
        </p>
      ) : null}
    </div>
  );
}

function PolicyRow(props: {
  policy: Schemas["Policy"];
  cols: string;
  modes: { value: Schemas["PolicyMode"]; label: string }[];
  disabled: boolean;
  onChange: (policy: Schemas["Policy"]) => void;
}) {
  const { policy } = props;
  const [limit, setLimit] = useState(
    policy.limitCents == null ? "" : String(policy.limitCents / 100),
  );
  return (
    <Row columns={props.cols}>
      <span className="t-item">{w.card.titles[policy.card]}</span>
      <Select
        label={`${w.card.titles[policy.card]}: ${w.settings.approvals.mode}`}
        value={policy.mode}
        options={props.modes}
        disabled={props.disabled}
        onChange={(mode) =>
          props.onChange({
            card: policy.card,
            mode,
            limitCents: mode === "allow_up_to" ? (parseDollars(limit) ?? 0) : null,
          })
        }
      />
      {policy.mode === "allow_up_to" ? (
        <input
          className="input"
          inputMode="decimal"
          aria-label={`${w.card.titles[policy.card]}: ${w.settings.approvals.limit}`}
          disabled={props.disabled}
          value={limit}
          onChange={(event) => setLimit(event.target.value)}
          onBlur={() => {
            const cents = parseDollars(limit);
            if (cents !== null && cents !== policy.limitCents)
              props.onChange({ card: policy.card, mode: policy.mode, limitCents: cents });
          }}
        />
      ) : (
        <span />
      )}
    </Row>
  );
}

export function Budgets() {
  const budgets = useBudgets();
  const projects = useProjects();
  const b = w.settings.budgets;
  const cols = "minmax(0, 1.4fr) 120px 120px minmax(0, 1fr)";
  const names = new Map((projects.data ?? []).map((project) => [project.id, project.name]));
  const bar = (spent: number, cap: number | null | undefined) =>
    cap == null ? null : (
      <span className="bar" aria-hidden="true">
        <span style={{ width: `${Math.round(share(spent, cap) * 100)}%` }} />
      </span>
    );
  return (
    <div className="sd-sec">
      <h3 className="t-section">{w.settings.sections.budgets}</h3>
      <p className="sd-lead">{b.lead}</p>
      {budgets.isPending ? (
        <Loading rows={2} />
      ) : budgets.isError ? (
        <Failed error={budgets.error} retry={() => void budgets.refetch()} />
      ) : (
        <Table
          columns={cols}
          head={[b.organization, b.spent, b.cap, ""]}
          label={w.settings.sections.budgets}
        >
          <Row columns={cols}>
            <span className="t-item">{b.organization}</span>
            <span className="mono">{dollars(budgets.data.organization.spentCents)}</span>
            <span className="mono">
              {budgets.data.organization.capCents == null
                ? b.noCap
                : dollars(budgets.data.organization.capCents)}
            </span>
            {bar(budgets.data.organization.spentCents, budgets.data.organization.capCents)}
          </Row>
          {budgets.data.projects.map((entry) => (
            <Row key={entry.projectId} columns={cols}>
              <span className="ell">{names.get(entry.projectId) ?? b.project}</span>
              <span className="mono">{dollars(entry.spentCents)}</span>
              <span className="mono">
                {entry.capCents == null ? b.noCap : dollars(entry.capCents)}
              </span>
              {bar(entry.spentCents, entry.capCents)}
            </Row>
          ))}
        </Table>
      )}
    </div>
  );
}

/** An action this client has no label for still reads as words, never as a code. */
function readable(action: string): string {
  const text = action.replace(/[._]/g, " ");
  return text.charAt(0).toUpperCase() + text.slice(1);
}

export function Audit() {
  const me = useMember();
  const audit = useAudit();
  const members = useMembers();
  const a = w.settings.audit;
  const cols = "150px minmax(0, 1fr) minmax(0, 1.4fr) 130px";
  if (me !== undefined && !isOwner(me.role)) {
    return (
      <div className="sd-sec">
        <h3 className="t-section">{w.settings.sections.audit}</h3>
        <ReadOnly text={w.readOnly.onlyOwners} />
      </div>
    );
  }
  const entries = audit.data?.pages.flatMap((page) => page.entries) ?? [];
  const who = (id: string | null | undefined) =>
    id == null ? a.system : (members.data?.find((member) => member.id === id)?.name ?? id);
  return (
    <div className="sd-sec">
      <h3 className="t-section">{w.settings.sections.audit}</h3>
      <p className="sd-lead">{a.lead}</p>
      {audit.isPending ? (
        <Loading rows={3} />
      ) : audit.isError ? (
        <Failed error={audit.error} retry={() => void audit.refetch()} />
      ) : entries.length === 0 ? (
        <Empty title={a.empty} />
      ) : (
        <Table
          columns={cols}
          head={[a.time, a.actor, a.action, a.outcome]}
          label={w.settings.sections.audit}
          min={640}
        >
          {entries.map((entry) => (
            <Row key={entry.id} columns={cols}>
              <span className="xs mut">{dateTime(entry.at)}</span>
              <span className="ell">{who(entry.actor)}</span>
              <span className="col" style={{ minWidth: 0 }}>
                <span className="ell">{a.actions[entry.action] ?? readable(entry.action)}</span>
                <span className="mono xs mut ell">{entry.target}</span>
              </span>
              <span className="tag">{a.outcomes[entry.outcome]}</span>
            </Row>
          ))}
        </Table>
      )}
      {audit.hasNextPage ? (
        <button
          type="button"
          className="btn sm"
          style={{ alignSelf: "flex-start", marginTop: 12 }}
          disabled={audit.isFetchingNextPage}
          onClick={() => void audit.fetchNextPage()}
        >
          {a.more}
        </button>
      ) : null}
    </div>
  );
}

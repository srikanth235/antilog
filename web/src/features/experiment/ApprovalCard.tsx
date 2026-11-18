import type { Schemas } from "../../api/client";
import { useDecide } from "../../api/queries";
import { Icon, StatusIcon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { dollars, time } from "../../lib/format";
import type { History } from "../../lib/history";
import { w } from "../../words";
import { cardRows, splitSummary } from "./cardRows";

/** A card is a frame: a title, what would happen, what it may cost, two buttons. */
export function ApprovalCard(props: {
  approval: Schemas["Approval"];
  experiment: string;
  canDecide: boolean;
  decidedBy: string;
  budgetCapCents: number;
  spentCents: number | undefined;
  history: History;
}) {
  const { approval } = props;
  const decide = useDecide(props.experiment);
  const title = w.card.titles[approval.card];

  if (approval.status !== "pending") {
    const line =
      approval.status === "approved"
        ? w.card.collapsed.approved(title, props.decidedBy)
        : approval.status === "declined"
          ? w.card.collapsed.declined(title, props.decidedBy)
          : w.card.collapsed.approved_by_policy(title);
    const at = approval.decidedAt ?? approval.createdAt;
    return (
      <div className="apc" data-card={approval.card}>
        <Icon name={approval.status === "declined" ? "close" : "check"} size="s" />
        {line} · {time(at)}
      </div>
    );
  }

  const credential = approval.card === "add_credential";
  const keepGoing = approval.card === "keep_going";
  const cost = approval.costCents;
  const rows = cardRows(approval, props);
  const lead = rows.some((row) => row.key === "why")
    ? splitSummary(approval.summary).lead
    : approval.summary;
  return (
    <section className="cd ap wait" aria-label={title} data-card={approval.card}>
      <div className="ap-h">
        <h3>{title}</h3>
        <span className="spill" style={{ ["--c" as string]: "var(--warn)" }}>
          <StatusIcon activity="waiting_for_you" />
          {w.activity.waiting_for_you}
        </span>
      </div>
      <p className="ap-lead">{lead}</p>
      <div className="ap-b">
        {rows.map((row) => (
          <div key={row.key} className="ap-r">
            <span className="k">{row.label}</span>
            <span className="v">
              {row.lines.map((line) => (
                <span key={line} className={row.mono ? "mono" : undefined}>
                  {line}
                </span>
              ))}
            </span>
          </div>
        ))}
        <div className="ap-r">
          <span className="k">{w.card.whatCost}</span>
          <span className="v">
            <span>{cost == null || cost === 0 ? w.card.costsNothing : w.card.mayCost(cost)}</span>
          </span>
        </div>
        {approval.needsOwner ? (
          <div className="ap-r">
            <span className="k">{w.value.sealValue}</span>
            <span className="v">
              <span>{w.value.needsOwner}</span>
            </span>
          </div>
        ) : null}
        {credential ? (
          <div className="ap-r">
            <span className="k">{w.card.titles.add_credential}</span>
            <span className="v">
              <span>{w.card.credentialNote}</span>
              <span className="target-note">{w.card.credentialTarget}</span>
            </span>
          </div>
        ) : null}
      </div>
      {decide.isError ? (
        <p className="error-line" role="alert">
          {reason(decide.error)}
        </p>
      ) : null}
      <div className="ap-f">
        <div className="nt">
          <span>
            {cost == null || cost === 0 ? w.card.costsNothing : `${w.card.mayCost(cost)}`}
          </span>
          <span className="xs">
            {props.canDecide
              ? approval.needsOwner
                ? w.card.waitingOwner
                : w.card.nothingHappens
              : w.readOnly.approve}
          </span>
        </div>
        <div className="bt">
          <button
            type="button"
            className="btn primary"
            disabled={!props.canDecide || decide.isPending || credential}
            onClick={() => decide.mutate({ id: approval.id, decision: "approve" })}
          >
            {w.card.approve[approval.card]}
          </button>
          <button
            type="button"
            className="btn"
            disabled={!props.canDecide || decide.isPending}
            onClick={() => decide.mutate({ id: approval.id, decision: "decline" })}
          >
            {keepGoing ? w.card.declineKeepGoing : w.card.decline}
          </button>
        </div>
      </div>
    </section>
  );
}

export function costLabel(cents: number): string {
  return dollars(cents);
}

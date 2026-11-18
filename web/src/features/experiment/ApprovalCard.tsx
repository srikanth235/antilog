import type { Schemas } from "../../api/client";
import { useDecide } from "../../api/queries";
import { Icon, StatusIcon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { dollars, time } from "../../lib/format";
import { w } from "../../words";

/** A card is a frame: a title, what would happen, what it may cost, two buttons. */
export function ApprovalCard(props: {
  approval: Schemas["Approval"];
  experiment: string;
  canDecide: boolean;
  decidedBy: string;
}) {
  const { approval } = props;
  const decide = useDecide(props.experiment);
  const title = w.card.titles[approval.card];

  if (approval.status !== "Pending") {
    const line =
      approval.status === "Approved"
        ? w.card.collapsed.Approved(title, props.decidedBy)
        : approval.status === "Declined"
          ? w.card.collapsed.Declined(title, props.decidedBy)
          : w.card.collapsed.ApprovedByPolicy(title);
    const at = approval.decidedAt ?? approval.createdAt;
    return (
      <div className="apc" data-card={approval.card}>
        <Icon name={approval.status === "Declined" ? "close" : "check"} size="s" />
        {line} · {time(at)}
      </div>
    );
  }

  const credential = approval.card === "AddCredential";
  const keepGoing = approval.card === "KeepGoing";
  const cost = approval.costCents;
  return (
    <section className="cd ap wait" aria-label={title} data-card={approval.card}>
      <div className="ap-h">
        <h3>{title}</h3>
        <span className="spill" style={{ ["--c" as string]: "var(--warn)" }}>
          <StatusIcon activity="WaitingForYou" />
          {w.activity.WaitingForYou}
        </span>
      </div>
      <p className="ap-lead">{approval.summary}</p>
      <div className="ap-b">
        <div className="ap-r">
          <span className="k">{w.card.whatCost}</span>
          <span className="v">
            <span>{cost == null || cost === 0 ? w.card.costsNothing : w.card.mayCost(cost)}</span>
          </span>
        </div>
        {credential ? (
          <div className="ap-r">
            <span className="k">{w.card.titles.AddCredential}</span>
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
            onClick={() => decide.mutate({ id: approval.id, decision: "Approve" })}
          >
            {w.card.approve[approval.card]}
          </button>
          <button
            type="button"
            className="btn"
            disabled={!props.canDecide || decide.isPending}
            onClick={() => decide.mutate({ id: approval.id, decision: "Decline" })}
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

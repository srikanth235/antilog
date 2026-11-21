import type { Schemas } from "../../api/client";
import { useDecide } from "../../api/queries";
import { Icon, StatusIcon } from "../../components/icon/Icon";
import { reason } from "../../components/state/State";
import { dollars, time } from "../../lib/format";
import { w } from "../../words";
import { cardLead, cardRows } from "./cardRows";

/** A card is a frame: a title, what would happen, what it may cost, two buttons. */
export function ApprovalCard(props: {
  approval: Schemas["Approval"];
  session: string;
  canDecide: boolean;
  decidedBy: string;
  budgetCapCents: number | null;
  spentCents: number | undefined;
}) {
  const { approval } = props;
  const decide = useDecide(props.session);
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
  const cost = approval.costCents;
  const rows = cardRows(approval, props);
  const lead = cardLead(approval);
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
              {row.lines.map((line, at) => (
                <span
                  // biome-ignore lint/suspicious/noArrayIndexKey: a row's lines are fixed text, never reordered
                  key={at}
                  className={
                    row.mono ? "mono" : at > 0 && row.key === "environment" ? "xs mut" : undefined
                  }
                >
                  {line}
                </span>
              ))}
              {row.diff === undefined ? null : (
                <section className="diff" aria-label={row.label}>
                  {row.diff.map((group) => (
                    <div key={group.title}>
                      <div className="hk">{group.title}</div>
                      {group.lines.map((line, at) => (
                        <div
                          // biome-ignore lint/suspicious/noArrayIndexKey: a diff's lines are fixed, never reordered
                          key={at}
                          className={line.sign === "+" ? "l add" : "l del"}
                        >
                          <span />
                          <span />
                          <span className="sg">{line.sign === "+" ? "+" : "−"}</span>
                          <span>{line.text}</span>
                        </div>
                      ))}
                    </div>
                  ))}
                </section>
              )}
            </span>
          </div>
        ))}
        <div className="ap-r">
          <span className="k">{w.card.whatCost}</span>
          <span className="v">
            <span>{cost == null || cost === 0 ? w.card.costsNothing : w.card.mayCost(cost)}</span>
          </span>
        </div>
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
          <span className="xs">{props.canDecide ? w.card.nothingHappens : w.readOnly.approve}</span>
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
            {w.card.decline}
          </button>
        </div>
      </div>
    </section>
  );
}

export function costLabel(cents: number): string {
  return dollars(cents);
}

import { Link } from "@tanstack/react-router";
import { Icon, StatusIcon } from "../../components/icon/Icon";
import { pendingCards } from "../../lib/domain";
import { ago } from "../../lib/format";
import { w } from "../../words";
import type { useSidebarData } from "../shell/useSidebarData";

/** Sessions that need a person. With none the section is absent, not an empty box. */
export function Waiting({ data }: { data: ReturnType<typeof useSidebarData> }) {
  const now = Date.now();
  const rows = data.all.flatMap((session) =>
    pendingCards(data.approvalsOf.get(session.id) ?? [])
      .slice(0, 1)
      .map((approval) => ({ session, approval })),
  );
  if (rows.length === 0) return null;
  return (
    <section className="wl" aria-labelledby="waiting-h">
      <div className="wl-h">
        <h2 className="t-label" id="waiting-h">
          {w.home.waiting}
        </h2>
        <span className="xs mut">{rows.length}</span>
      </div>
      {rows.map(({ session, approval }) => (
        <Link
          key={approval.id}
          className="wr"
          to="/sessions/$sessionId"
          params={{ sessionId: session.id }}
        >
          <StatusIcon activity="waiting_for_you" />
          <span className="t">{session.title}</span>
          <span className="w">{w.card.titles[approval.card]}</span>
          <span className="a">{ago(approval.createdAt, now)}</span>
          <Icon name="chev" size="s" />
        </Link>
      ))}
    </section>
  );
}

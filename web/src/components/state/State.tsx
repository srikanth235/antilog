import type { ReactNode } from "react";
import { ApiFailure } from "../../api/client";
import { w } from "../../words";

/** The page type owns loading, empty, error and denied states (design/behaviour). */
export function Loading({ rows = 3 }: { rows?: number }) {
  return (
    <div role="status" aria-label={w.loading} className="col" style={{ gap: 8 }}>
      {[1, 2, 3, 4, 5, 6].slice(0, rows).map((row) => (
        <div
          key={row}
          style={{
            height: 56,
            borderBottom: "1px solid var(--line)",
            background: "var(--sunk)",
            borderRadius: "var(--r1)",
          }}
        />
      ))}
    </div>
  );
}

export function Empty(props: { title: string; hint?: string; children?: ReactNode }) {
  return (
    <div className="empty">
      <span className="t-section">{props.title}</span>
      {props.hint === undefined ? null : <span className="ink2 sm">{props.hint}</span>}
      {props.children}
    </div>
  );
}

export function Failed({ error, retry }: { error: unknown; retry?: () => void }) {
  const denied = error instanceof ApiFailure && error.status === 403;
  const text = denied ? w.denied : error instanceof ApiFailure ? error.message : w.somethingFailed;
  return (
    <div className="empty" role="alert">
      <span className="t-section">{text}</span>
      {denied || retry === undefined ? null : (
        <button type="button" className="btn" onClick={retry}>
          {w.tryAgain}
        </button>
      )}
    </div>
  );
}

export function reason(error: unknown): string {
  return error instanceof ApiFailure ? error.message : w.somethingFailed;
}

import type { ReactNode } from "react";

/** A quiet table in the board's grid style, as a list of rows so every row is announced. */
export function Table(props: {
  columns: string;
  head: string[];
  label: string;
  children: ReactNode;
  min?: number;
}) {
  return (
    <div className="table qt">
      <ul
        aria-label={props.label}
        style={{ minWidth: props.min ?? 560, margin: 0, padding: 0, listStyle: "none" }}
      >
        <li className="grid th" style={{ gridTemplateColumns: props.columns }}>
          {props.head.map((name, index) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: a fixed header row; names can repeat when blank
            <span key={index}>{name}</span>
          ))}
        </li>
        {props.children}
      </ul>
    </div>
  );
}

export function Row(props: { columns: string; children: ReactNode }) {
  return (
    <li className="grid" style={{ gridTemplateColumns: props.columns }}>
      {props.children}
    </li>
  );
}

export function ReadOnly({ text }: { text: string }) {
  return (
    <p className="sd-ro">
      <i className="ic i-lock" aria-hidden="true" />
      <span>{text}</span>
    </p>
  );
}

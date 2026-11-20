import type { ReactNode } from "react";

/**
 * A small markdown renderer for what an agent says: paragraphs, line breaks, lists, inline code,
 * code blocks, bold, italic, links. It builds React elements and never reads text as HTML, so raw
 * HTML from an agent shows as the text it is. Links are http, https or mailto only.
 */

const INLINE =
  /(`+)([^\n]*?[^`\n])\1(?!`)|\*\*([^\n]+?)\*\*|__([^\n]+?)__|\*([^*\s][^*\n]*?)\*|(?<![\w])_([^_\s][^_\n]*?)_(?![\w])|\[([^\]\n]+)\]\(([^)\s]+)\)/g;

function safeHref(url: string): string | null {
  return /^(https?:\/\/|mailto:)/i.test(url) ? url : null;
}

export function renderInline(text: string, keyBase = "i"): ReactNode[] {
  const out: ReactNode[] = [];
  let last = 0;
  let n = 0;
  for (const m of text.matchAll(INLINE)) {
    const at = m.index ?? 0;
    if (at > last) out.push(text.slice(last, at));
    const key = `${keyBase}-${n++}`;
    if (m[2] !== undefined) out.push(<code key={key}>{m[2]}</code>);
    else if (m[3] !== undefined || m[4] !== undefined)
      out.push(<strong key={key}>{renderInline(m[3] ?? m[4] ?? "", key)}</strong>);
    else if (m[5] !== undefined || m[6] !== undefined)
      out.push(<em key={key}>{renderInline(m[5] ?? m[6] ?? "", key)}</em>);
    else {
      const href = safeHref(m[8] ?? "");
      if (href === null) out.push(m[0]);
      else
        out.push(
          <a key={key} href={href} target="_blank" rel="noopener noreferrer">
            {renderInline(m[7] ?? "", key)}
          </a>,
        );
    }
    last = at + m[0].length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

function lines(text: string, key: string): ReactNode[] {
  const out: ReactNode[] = [];
  let n = 0;
  for (const line of text.split("\n")) {
    if (n > 0) out.push(<br key={`${key}-br${n}`} />);
    out.push(...renderInline(line, `${key}-${n}`));
    n++;
  }
  return out;
}

const BULLET = /^\s{0,3}[-*+]\s+(.*)$/;
const ORDERED = /^\s{0,3}\d{1,9}[.)]\s+(.*)$/;
const FENCE = /^\s{0,3}(```+|~~~+)/;
const HEADING = /^\s{0,3}#{1,6}\s+(.*?)\s*#*\s*$/;

export function renderMarkdown(source: string): ReactNode[] {
  const rows = source.replace(/\r\n?/g, "\n").split("\n");
  const out: ReactNode[] = [];
  let i = 0;
  let para: string[] = [];
  const flush = () => {
    if (para.length > 0) {
      const key = `p${out.length}`;
      out.push(<p key={key}>{lines(para.join("\n"), key)}</p>);
      para = [];
    }
  };
  while (i < rows.length) {
    const row = rows[i] ?? "";
    const fence = FENCE.exec(row);
    if (fence !== null) {
      flush();
      const mark = fence[1] ?? "```";
      const body: string[] = [];
      i++;
      while (i < rows.length && !(rows[i] ?? "").trim().startsWith(mark))
        body.push(rows[i++] ?? "");
      i++;
      out.push(
        <pre key={`c${out.length}`}>
          <code>{body.join("\n")}</code>
        </pre>,
      );
      continue;
    }
    const list = BULLET.test(row) ? BULLET : ORDERED.test(row) ? ORDERED : null;
    if (list !== null) {
      flush();
      const items: ReactNode[] = [];
      const key = `l${out.length}`;
      while (i < rows.length) {
        const m = list.exec(rows[i] ?? "");
        if (m === null) break;
        let item = m[1] ?? "";
        i++;
        // An indented line under an item continues it.
        while (
          i < rows.length &&
          /^\s{2,}\S/.test(rows[i] ?? "") &&
          !BULLET.test(rows[i] ?? "") &&
          !ORDERED.test(rows[i] ?? "")
        ) {
          item += `\n${(rows[i] ?? "").trim()}`;
          i++;
        }
        items.push(<li key={`${key}-${items.length}`}>{lines(item, `${key}-${items.length}`)}</li>);
      }
      out.push(list === BULLET ? <ul key={key}>{items}</ul> : <ol key={key}>{items}</ol>);
      continue;
    }
    const heading = HEADING.exec(row);
    if (heading !== null) {
      flush();
      out.push(
        <p key={`h${out.length}`}>
          <strong>{renderInline(heading[1] ?? "", `h${out.length}`)}</strong>
        </p>,
      );
      i++;
      continue;
    }
    if (row.trim() === "") flush();
    else para.push(row);
    i++;
  }
  flush();
  return out;
}

export function Markdown(props: { text: string }) {
  return <>{renderMarkdown(props.text)}</>;
}

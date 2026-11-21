/** A note is one line; words after a blank line (a compaction's summary) are folded behind it. */
export function splitNote(text: string): { line: string; detail: string } {
  const [line = "", ...rest] = text.split(/\n\s*\n/);
  return { line, detail: rest.join("\n\n").trim() };
}

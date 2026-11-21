import type { Schemas } from "../api/client";

export type FileChange = Schemas["FileChange"];

export type DiffLine = {
  kind: "context" | "add" | "del";
  /** The line number in the old file; null for an added line. */
  old: number | null;
  /** The line number in the new file; null for a removed line. */
  next: number | null;
  text: string;
};

export type Hunk = {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  /** What follows the second `@@`: git's guess at the enclosing function. */
  heading: string;
  lines: DiffLine[];
};

const HEADER = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@ ?(.*)$/;

/** Reads the hunks of one file's unified diff (from its first `@@` line on). */
export function parsePatch(patch: string): Hunk[] {
  const hunks: Hunk[] = [];
  let hunk: Hunk | null = null;
  let old = 0;
  let next = 0;
  for (const raw of patch.split("\n")) {
    const header = HEADER.exec(raw);
    if (header !== null) {
      old = Number(header[1]);
      next = Number(header[3]);
      hunk = {
        oldStart: old,
        oldLines: header[2] === undefined ? 1 : Number(header[2]),
        newStart: next,
        newLines: header[4] === undefined ? 1 : Number(header[4]),
        heading: header[5] ?? "",
        lines: [],
      };
      hunks.push(hunk);
      continue;
    }
    if (hunk === null || raw.startsWith("\\")) continue;
    const sign = raw[0];
    const text = raw.slice(1);
    if (sign === "+") {
      hunk.lines.push({ kind: "add", old: null, next: next++, text });
    } else if (sign === "-") {
      hunk.lines.push({ kind: "del", old: old++, next: null, text });
    } else if (sign === " ") {
      hunk.lines.push({ kind: "context", old: old++, next: next++, text });
    }
  }
  return hunks;
}

/** Unchanged lines before a hunk, between it and the one before (or the file's start). */
export function unmodifiedBefore(hunks: Hunk[], index: number): number {
  const hunk = hunks[index];
  if (hunk === undefined) return 0;
  const previous = hunks[index - 1];
  const from = previous === undefined ? 1 : previous.oldStart + previous.oldLines;
  const start = hunk.oldLines === 0 ? hunk.oldStart + 1 : hunk.oldStart;
  return Math.max(0, start - from);
}

export type Totals = { files: number; additions: number; deletions: number };

export function totals(files: FileChange[]): Totals {
  let additions = 0;
  let deletions = 0;
  for (const file of files) {
    additions += file.additions;
    deletions += file.deletions;
  }
  return { files: files.length, additions, deletions };
}

/** A path as the file list shows it: the name, and the folder it is in (empty at the root). */
export function splitPath(path: string): { name: string; folder: string } {
  const at = path.lastIndexOf("/");
  return at < 0
    ? { name: path, folder: "" }
    : { name: path.slice(at + 1), folder: path.slice(0, at) };
}

/** Why a file has no diff to show, or null when it has one. */
export function withheld(file: FileChange): "binary" | "tooLarge" | null {
  if (file.binary) return "binary";
  if (file.tooLarge) return "tooLarge";
  return null;
}

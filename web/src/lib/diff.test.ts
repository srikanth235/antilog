import { describe, expect, it } from "vitest";
import { type FileChange, parsePatch, splitPath, totals, unmodifiedBefore, withheld } from "./diff";

const file = (patch: Partial<FileChange>): FileChange => ({
  path: "a.ts",
  oldPath: null,
  status: "modified",
  additions: 0,
  deletions: 0,
  binary: false,
  tooLarge: false,
  patch: "",
  ...patch,
});

const PATCH = [
  "@@ -9,5 +9,6 @@ export function createPool(",
  "   return new Pool({",
  "     host: cfg.host,",
  "-    max: 16,",
  "+    max: 48,",
  "+    min: 8,",
  "     idle: 1,",
  "   });",
  "@@ -31,2 +32,3 @@ export async function warm(",
  " export async function warm(pool: Pool) {",
  "+  await pool.ping();",
  " }",
  "",
].join("\n");

describe("parsePatch", () => {
  it("numbers old and new lines through each hunk", () => {
    const hunks = parsePatch(PATCH);
    expect(hunks).toHaveLength(2);
    expect(hunks[0]).toMatchObject({ oldStart: 9, oldLines: 5, newStart: 9, newLines: 6 });
    expect(hunks[0]?.heading).toBe("export function createPool(");
    expect(hunks[0]?.lines.map((l) => [l.kind, l.old, l.next])).toEqual([
      ["context", 9, 9],
      ["context", 10, 10],
      ["del", 11, null],
      ["add", null, 11],
      ["add", null, 12],
      ["context", 12, 13],
      ["context", 13, 14],
    ]);
    expect(hunks[1]?.lines[1]).toEqual({
      kind: "add",
      old: null,
      next: 33,
      text: "  await pool.ping();",
    });
  });

  it("keeps a blank context line and skips the no-newline marker", () => {
    const hunks = parsePatch("@@ -1,2 +1,2 @@\n \n-a\n+b\n\\ No newline at end of file\n");
    expect(hunks[0]?.lines.map((l) => [l.kind, l.text])).toEqual([
      ["context", ""],
      ["del", "a"],
      ["add", "b"],
    ]);
  });

  it("reads a header without counts as one line, and an empty patch as no hunks", () => {
    expect(parsePatch("@@ -0,0 +1 @@\n+x\n")[0]).toMatchObject({ oldLines: 0, newLines: 1 });
    expect(parsePatch("")).toEqual([]);
  });
});

describe("unmodifiedBefore", () => {
  it("counts the lines hidden before a hunk and between hunks", () => {
    const hunks = parsePatch(PATCH);
    expect(unmodifiedBefore(hunks, 0)).toBe(8);
    expect(unmodifiedBefore(hunks, 1)).toBe(31 - (9 + 5));
    expect(unmodifiedBefore(hunks, 2)).toBe(0);
  });

  it("counts none before a new file", () => {
    expect(unmodifiedBefore(parsePatch("@@ -0,0 +1,2 @@\n+a\n+b\n"), 0)).toBe(0);
  });
});

describe("the file list", () => {
  it("adds up the counts of every file", () => {
    expect(
      totals([
        file({ additions: 9, deletions: 3 }),
        file({ additions: 22 }),
        file({ deletions: 31, status: "deleted" }),
      ]),
    ).toEqual({ files: 3, additions: 31, deletions: 34 });
    expect(totals([])).toEqual({ files: 0, additions: 0, deletions: 0 });
  });

  it("splits a path into its name and folder", () => {
    expect(splitPath("src/db/pool.ts")).toEqual({ name: "pool.ts", folder: "src/db" });
    expect(splitPath("README.md")).toEqual({ name: "README.md", folder: "" });
  });

  it("says why a file shows no diff", () => {
    expect(withheld(file({ binary: true }))).toBe("binary");
    expect(withheld(file({ tooLarge: true }))).toBe("tooLarge");
    expect(withheld(file({ patch: "@@ -1 +1 @@\n-a\n+b\n" }))).toBeNull();
  });
});

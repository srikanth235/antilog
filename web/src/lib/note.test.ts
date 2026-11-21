import { describe, expect, it } from "vitest";
import { splitNote } from "./note";

describe("splitNote", () => {
  it("keeps a plain note as one line", () => {
    expect(splitNote("Switched to Model 2 · High")).toEqual({
      line: "Switched to Model 2 · High",
      detail: "",
    });
  });
  it("folds a compaction's summary behind its line", () => {
    expect(splitNote("Context compacted.\n\nWe fixed the bug.\n\nNext: tests.")).toEqual({
      line: "Context compacted.",
      detail: "We fixed the bug.\n\nNext: tests.",
    });
  });
});

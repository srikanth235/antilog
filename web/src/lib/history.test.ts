import { describe, expect, it } from "vitest";
import { kept, parseHistory } from "./history";

describe("parseHistory", () => {
  it("reads the not-sealed answer", () => {
    const history = parseHistory({
      note: "not sealed: call seal with a definition first",
      trials: [],
    });
    expect(history.trials).toEqual([]);
    expect(history.note).toContain("not sealed");
  });
  it("numbers trials and drops rows it cannot read", () => {
    const history = parseHistory({
      version: 1,
      unit: "ms",
      best: "t2",
      trials: [
        { trial: "t1", session: "s", parent: null, hypothesis: "a", outcome: "worse", value: 1200 },
        { trial: "t2", session: "s", parent: "t1", hypothesis: "b", outcome: "kept", value: 1100 },
        { trial: "t3", session: "s", outcome: "mystery" },
        "nonsense",
      ],
    });
    expect(history.trials.map((row) => row.number)).toEqual([1, 2]);
    expect(kept(history)).toBe(1);
    expect(history.best).toBe("t2");
  });
  it("survives anything", () => {
    expect(parseHistory(null).trials).toEqual([]);
    expect(parseHistory("x").version).toBeNull();
  });
});

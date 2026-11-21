import { describe, expect, it } from "vitest";
import { contextView, tokens } from "./context";

describe("tokens", () => {
  it("writes them as Claude Code does", () => {
    expect(tokens(950)).toBe("950");
    expect(tokens(12_340)).toBe("12.3k");
    expect(tokens(129_600)).toBe("129.6k");
    expect(tokens(200_000)).toBe("200k");
    expect(tokens(1_000_000)).toBe("1M");
    expect(tokens(1_250_000)).toBe("1.3M");
    expect(tokens(999_990)).toBe("1M");
  });
});

describe("contextView", () => {
  const parts = [
    { kind: "messages", tokens: 80_000 },
    { kind: "system", tokens: 10_000 },
  ] as const;
  it("shows the measured parts in order, then the free space", () => {
    const view = contextView({
      used: 129_600,
      window: 1_000_000,
      parts: [...parts],
      reportedAt: 1,
    });
    expect(view?.percent).toBe(13);
    expect(view?.warn).toBe(false);
    expect(view?.rows.map((row) => row.kind)).toEqual(["system", "messages", "free"]);
    expect(view?.rows.at(-1)).toEqual({ kind: "free", tokens: 870_400, percent: 87 });
  });
  it("warns at 80 percent", () => {
    expect(contextView({ used: 160_000, window: 200_000, parts: [], reportedAt: 1 })?.warn).toBe(
      true,
    );
    expect(contextView({ used: 159_000, window: 200_000, parts: [], reportedAt: 1 })?.warn).toBe(
      false,
    );
  });
  it("leaves out what it does not know", () => {
    expect(contextView(undefined)).toBeNull();
    expect(contextView({ used: null, window: 1000, parts: [], reportedAt: null })).toBeNull();
    expect(contextView({ used: 5, window: null, parts: [], reportedAt: 1 })).toBeNull();
    // Parts without a use: no view, no row of a figure we cannot back.
    expect(contextView({ used: null, window: null, parts: [...parts], reportedAt: 1 })).toBeNull();
  });
});

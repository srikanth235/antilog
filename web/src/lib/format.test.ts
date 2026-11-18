import { describe, expect, it } from "vitest";
import { ago, dollars, parseDollars, plural, share } from "./format";

describe("format", () => {
  it("writes whole dollars without cents", () => {
    expect(dollars(4200)).toBe("$42");
    expect(dollars(270)).toBe("$2.70");
  });
  it("parses typed dollars", () => {
    expect(parseDollars("$12.50")).toBe(1250);
    expect(parseDollars("abc")).toBeNull();
    expect(parseDollars("")).toBeNull();
  });
  it("pluralises", () => {
    expect(plural(1, "experiment")).toBe("1 experiment");
    expect(plural(3, "experiment")).toBe("3 experiments");
  });
  it("says how long ago", () => {
    expect(ago(0, 5 * 60000)).toBe("5 min ago");
  });
  it("caps a share at one", () => {
    expect(share(150, 100)).toBe(1);
    expect(share(0, 0)).toBe(1);
  });
});

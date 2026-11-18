import { describe, expect, it } from "vitest";
import {
  agentTotals,
  centsPerUnit,
  checkValueForm,
  costPerWin,
  fromTo,
  isHttpUrl,
  modelLine,
  monthOptions,
  percent,
  readableUnitSize,
  returnLabel,
  valueSentence,
  winChange,
} from "./value";

const say = {
  metric: "m",
  unit: "u",
  unitSize: "s",
  dollarsPerYear: "d",
  source: "so",
  link: "l",
};
const form = {
  metric: "Bundle size",
  unit: "KB",
  unitSize: "10",
  dollarsPerYear: "$420",
  source: "CDN bill",
  link: "",
};

describe("value model conversion", () => {
  it("turns one unit and a yearly worth into cents per unit", () => {
    expect(centsPerUnit(10, 420)).toBe(4200);
    expect(centsPerUnit(100, 3600)).toBe(3600);
    expect(centsPerUnit(0, 10)).toBeNull();
    expect(centsPerUnit(1, 0)).toBeNull();
  });
  it("writes the live sentence", () => {
    expect(valueSentence("production bundle size", "KB", 10, 420)).toBe(
      "Each 10 KB of production bundle size is worth $420 a year. A win of 40 KB would be worth $1,680 a year.",
    );
    expect(valueSentence("", "ms", 100, 3600)).toContain("Each 100 ms is worth $3,600 a year.");
    expect(valueSentence("x", "ms", 0, 10)).toBeNull();
  });
  it("reads a stored model back as a round sentence", () => {
    expect(readableUnitSize(3600)).toBe(10);
    expect(readableUnitSize(35000)).toBe(1);
    expect(modelLine({ unit: "ms", centsPerUnit: 3600 })).toBe("Each 10 ms is worth $360 a year");
    expect(modelLine({ unit: "minute", centsPerUnit: 35000 })).toBe(
      "Each minute is worth $350 a year",
    );
  });
});

describe("returns", () => {
  it("words Return", () => {
    expect(returnLabel(59.3)).toBe("59×");
    expect(returnLabel(4.24)).toBe("4.2×");
    expect(returnLabel(0)).toBe("0×");
    expect(returnLabel(null)).toBe("-");
    expect(returnLabel(undefined)).toBe("-");
  });
  it("computes shares and cost per win", () => {
    expect(percent(27, 165)).toBe(16);
    expect(percent(1, 0)).toBe(0);
    expect(costPerWin(16500, 2)).toBe(8250);
    expect(costPerWin(6100, 0)).toBeNull();
  });
  it("sums the agent rows", () => {
    const row = {
      agent: "a",
      trials: 2,
      kept: 1,
      winsDelivered: 1,
      winsPending: 0,
      pendingCents: 0,
      spentCents: 100,
      deliveredCents: 500,
    };
    expect(agentTotals([row, { ...row, agent: "b", winsPending: 1, pendingCents: 7 }])).toEqual({
      trials: 4,
      kept: 2,
      winsDelivered: 2,
      winsPending: 1,
      pendingCents: 7,
      spentCents: 200,
      deliveredCents: 1000,
    });
  });
});

describe("the form", () => {
  it("builds the request body", () => {
    expect(checkValueForm(form, say)).toEqual({
      ok: true,
      body: {
        metric: "Bundle size",
        unit: "KB",
        centsPerUnit: 4200,
        source: { text: "CDN bill", link: null },
      },
    });
  });
  it("names every field that is wrong", () => {
    const bad = checkValueForm(
      { metric: " ", unit: "", unitSize: "0", dollarsPerYear: "abc", source: "", link: "ftp://x" },
      say,
    );
    expect(bad).toEqual({
      ok: false,
      errors: {
        metric: "m",
        unit: "u",
        unitSize: "s",
        dollarsPerYear: "d",
        source: "so",
        link: "l",
      },
    });
  });
  it("accepts only http and https links", () => {
    expect(isHttpUrl("https://example.com/study")).toBe(true);
    expect(isHttpUrl("javascript:alert(1)")).toBe(false);
    expect(isHttpUrl("example.com")).toBe(false);
  });
});

describe("months and wins", () => {
  it("lists the last months in UTC, newest first", () => {
    const months = monthOptions(new Date(Date.UTC(2026, 0, 15)), 3);
    expect(months).toEqual([
      { value: "2026-01", label: "January 2026" },
      { value: "2025-12", label: "December 2025" },
      { value: "2025-11", label: "November 2025" },
    ]);
  });
  it("words a win's change", () => {
    expect(winChange(1310, 1102)).toBe("-15.9%");
    expect(winChange(0, 1)).toBeNull();
    expect(fromTo({ baseline: 1310, best: 1102, unit: "ms" })).toBe("1,310 ms to 1,102 ms");
  });
});

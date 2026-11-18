import { describe, expect, it } from "vitest";
import { fromWire, toPascal, toSnake, toWire } from "./wire";

describe("wire", () => {
  it("converts names both ways", () => {
    expect(toSnake("ApprovedByPolicy")).toBe("approved_by_policy");
    expect(toPascal("waiting_for_you")).toBe("WaitingForYou");
  });

  it("reads server enums as the schema names them", () => {
    const body = fromWire([{ role: "owner", status: "not_sealed", goal: "done" }]);
    expect(body).toEqual([{ role: "Owner", status: "NotSealed", goal: "done" }]);
  });

  it("leaves unknown values and health alone", () => {
    expect(fromWire({ status: "ok" })).toEqual({ status: "ok" });
  });

  it("leaves a tool answer untouched", () => {
    const answer = { status: "waiting for a person" };
    expect(fromWire({ session: "s", answer })).toEqual({ session: "s", answer });
  });

  it("writes enums the way the server reads them", () => {
    expect(toWire({ card: "RaiseBudget", mode: "AllowUpTo", limitCents: 5 })).toEqual({
      card: "raise_budget",
      mode: "allow_up_to",
      limitCents: 5,
    });
  });
});

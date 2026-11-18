import { describe, expect, it } from "vitest";
import type { Schemas } from "../api/client";
import {
  activityOf,
  agentName,
  canEdit,
  defaultBudgetCents,
  leadSession,
  parseRepos,
} from "./domain";

const session = (
  status: Schemas["SessionStatus"],
  createdAt = 1,
  parent?: string,
): Schemas["Session"] =>
  ({ status, createdAt, parent: parent ?? null, id: `s${createdAt}` }) as Schemas["Session"];

describe("domain", () => {
  it("puts what needs a person first", () => {
    expect(activityOf([session("Working")], 1)).toBe("WaitingForYou");
    expect(activityOf([session("Done"), session("Working", 2)], 0)).toBe("Working");
    expect(activityOf([session("Failed")], 0)).toBe("Failed");
    expect(activityOf([], 0)).toBeNull();
  });
  it("finds the lead session", () => {
    expect(leadSession([session("Done", 5), session("Done", 2), session("Done", 1, "x")])?.id).toBe(
      "s2",
    );
  });
  it("lets only viewers read", () => {
    expect(canEdit("Viewer")).toBe(false);
    expect(canEdit("Editor")).toBe(true);
  });
  it("falls back from project to organization to ten dollars", () => {
    expect(defaultBudgetCents({ defaultBudgetCents: 5000 }, { defaultBudgetCents: 100 })).toBe(
      5000,
    );
    expect(defaultBudgetCents({ defaultBudgetCents: null }, { defaultBudgetCents: 2000 })).toBe(
      2000,
    );
    expect(defaultBudgetCents(undefined, undefined)).toBe(1000);
  });
  it("keeps only owner/name repos", () => {
    expect(parseRepos("acme/checkout, nope,acme/pricing")).toEqual([
      "acme/checkout",
      "acme/pricing",
    ]);
  });
  it("names agents", () => {
    expect(agentName("claude-code")).toBe("Claude Code");
  });
});

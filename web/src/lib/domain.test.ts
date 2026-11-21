import { describe, expect, it } from "vitest";
import type { Schemas } from "../api/client";
import {
  activityOf,
  agentLabel,
  canEdit,
  composerPlaceholderKey,
  defaultBudgetCents,
  headLabel,
  leadSession,
  parseRepos,
} from "./domain";

const session = (status: Schemas["SessionStatus"], createdAt = 1): Schemas["Session"] =>
  ({ status, createdAt, id: `s${createdAt}` }) as Schemas["Session"];

describe("domain", () => {
  it("puts what needs a person first", () => {
    expect(activityOf([session("working")], 1)).toBe("waiting_for_you");
    expect(activityOf([session("done"), session("working", 2)], 0)).toBe("working");
    expect(activityOf([session("failed")], 0)).toBe("failed");
    expect(activityOf([], 0)).toBeNull();
  });
  it("tells queued, stopped and failed apart from done", () => {
    expect(activityOf([session("queued")], 0)).toBe("queued");
    expect(activityOf([session("stopped")], 0)).toBe("stopped");
    expect(activityOf([session("failed")], 0)).toBe("failed");
    expect(activityOf([session("done")], 0)).toBe("done");
    expect(activityOf([session("queued"), session("working", 2)], 0)).toBe("working");
  });
  it("names where a repo's work stands, whether or not the agent committed", () => {
    const sha = "0123456789abcdef";
    expect(headLabel({ headBranch: "main", headSha: sha })).toBe("main");
    expect(headLabel({ headBranch: "feat/cache", headSha: sha })).toBe("feat/cache");
    expect(headLabel({ headBranch: "HEAD", headSha: sha })).toBe("0123456");
  });
  it("finds the lead session", () => {
    expect(leadSession([session("done", 5), session("done", 2), session("done", 1)])?.id).toBe(
      "s1",
    );
  });
  it("lets only viewers read", () => {
    expect(canEdit("viewer")).toBe(false);
    expect(canEdit("editor")).toBe(true);
  });
  it("falls back from project to organization, and never invents one", () => {
    expect(defaultBudgetCents({ defaultBudgetCents: 5000 }, { defaultBudgetCents: 100 })).toBe(
      5000,
    );
    expect(defaultBudgetCents({ defaultBudgetCents: null }, { defaultBudgetCents: 2000 })).toBe(
      2000,
    );
    expect(defaultBudgetCents(undefined, undefined)).toBeNull();
  });
  it("keeps only owner/name repos", () => {
    expect(parseRepos("acme/checkout, nope,acme/pricing")).toEqual([
      "acme/checkout",
      "acme/pricing",
    ]);
  });
  it("names agents by their display name, never by their id", () => {
    const agents = [{ id: "opencode", name: "OpenCode" }];
    expect(agentLabel(agents, "opencode")).toBe("OpenCode");
    expect(agentLabel(agents, "mcp")).toBe("Your agent");
    expect(agentLabel(agents, "claude-code")).toBe("An agent");
    expect(agentLabel(undefined, "opencode")).toBe("An agent");
  });
});

describe("composerPlaceholderKey", () => {
  it("asks to approve the card only while a card shows", () => {
    expect(composerPlaceholderKey("waiting_for_you", true)).toBe("waiting_for_you");
    expect(composerPlaceholderKey("waiting_for_you", false)).toBe("waiting_no_card");
    expect(composerPlaceholderKey("working", false)).toBe("working");
    expect(composerPlaceholderKey("none", false)).toBe("none");
  });
});

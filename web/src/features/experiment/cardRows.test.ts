import { describe, expect, it } from "vitest";
import type { Schemas } from "../../api/client";
import { cardLead, cardRows } from "./cardRows";

const ctx = { budgetCapCents: 10_000, spentCents: 4_000 };

function card(payload: Schemas["CardPayload"] | null, summary = "legacy summary: reason") {
  return {
    id: "a",
    organizationId: "o",
    sessionId: "s",
    card: payload?.kind ?? "ask_first",
    summary,
    payload,
    costCents: null,
    status: "pending",
    createdAt: 0,
  } as unknown as Schemas["Approval"];
}

describe("cardRows", () => {
  it("builds Raise budget from its typed fields, not from the summary", () => {
    const approval = card(
      { kind: "raise_budget", nowCents: 10_000, requestedCents: 5_000, why: "more trials" },
      "a summary that says something else entirely",
    );
    expect(cardLead(approval)).toBe("Raise the budget by $50");
    expect(cardRows(approval, ctx).map((row) => [row.key, row.lines])).toEqual([
      ["now", ["$40 of $100"]],
      ["requested", ["$150"]],
      ["why", ["more trials"]],
    ]);
  });

  it("shows Attach repo's repo, access and branch", () => {
    const approval = card({
      kind: "attach_repo",
      repo: "acme/app",
      access: "push",
      branch: "antilog/p95",
    });
    expect(cardLead(approval)).toBe("Attach repo acme/app");
    expect(cardRows(approval, ctx).map((row) => row.key)).toEqual(["repo", "access", "branch"]);
  });

  it("leaves out environment fields that were not asked for", () => {
    const approval = card({ kind: "change_environment", size: "large" });
    expect(cardRows(approval, ctx).map((row) => row.key)).toEqual(["size"]);
  });

  it("shows Use a connector tool's connector, tool and input digest", () => {
    const approval = card({
      kind: "use_connector_tool",
      connector: "Orders warehouse",
      tool: "lookup",
      inputDigest: "9f2c41ab",
    });
    expect(cardLead(approval)).toBe("Use lookup on Orders warehouse");
    expect(cardRows(approval, ctx).map((row) => [row.key, row.lines])).toEqual([
      ["connector", ["Orders warehouse"]],
      ["tool", ["lookup"]],
      ["input", ["9f2c41ab"]],
    ]);
  });

  it("falls back to the summary for a card raised before payloads, and parses nothing", () => {
    const approval = card(null);
    expect(cardLead(approval)).toBe("legacy summary: reason");
    expect(cardRows(approval, ctx)).toEqual([]);
  });
});

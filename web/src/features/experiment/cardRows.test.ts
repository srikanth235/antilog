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

  it("shows a Change environment card as a diff of what changes, grouped by when it applies", () => {
    const view = {
      network: "limited" as const,
      allowedHosts: ["api.stripe.com"],
      allowPackageManagers: true,
      variables: { STRIPE_KEY: "sk_test", OLD: "1" },
      setupScript: "npm ci",
      repos: ["acme/app"],
    };
    const approval = card({
      kind: "change_environment",
      environment: "bench",
      environmentName: "Bench",
      sessionVersion: 3,
      baseVersion: 3,
      createsEnvironment: false,
      change: {},
      before: view,
      after: {
        ...view,
        allowedHosts: ["registry.example.com"],
        variables: { STRIPE_KEY: "sk_live", REGION: "eu" },
        setupScript: "npm ci\nnpm run build",
        repos: ["acme/app", "acme/web"],
      },
    });
    const rows = cardRows(approval, ctx);
    expect(rows.map((row) => row.key)).toEqual(["environment", "change"]);
    expect(rows[0]?.lines[1]).toContain("version 4");
    expect(
      rows[1]?.diff?.map((group) => [group.title, group.lines.map((l) => l.sign + l.text)]),
    ).toEqual([
      ["Network, applies now", ["-api.stripe.com", "+registry.example.com"]],
      [
        "Variables, for commands run from now on",
        ["-OLD=1", "+REGION=eu", "-STRIPE_KEY=sk_test", "+STRIPE_KEY=sk_live"],
      ],
      ["Setup script, runs now", ["-npm ci", "+npm ci", "+npm run build"]],
      ["Repositories, cloned now", ["+acme/web"]],
    ]);
  });

  it("says a session on the default gets an environment of its own", () => {
    const view = {
      network: "limited" as const,
      allowedHosts: [],
      allowPackageManagers: true,
      variables: {},
      setupScript: "",
      repos: [],
    };
    const approval = card({
      kind: "change_environment",
      environment: "default",
      environmentName: "Session 018f3b1e",
      sessionVersion: 1,
      baseVersion: 1,
      createsEnvironment: true,
      change: {},
      before: view,
      after: { ...view, variables: { REGION: "eu" } },
    });
    expect(cardRows(approval, ctx)[0]?.lines[1]).toContain("environment of its own");
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

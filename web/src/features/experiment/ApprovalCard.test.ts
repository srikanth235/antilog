import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { Schemas } from "../../api/client";
import { ApprovalCard } from "./ApprovalCard";

function render(costCents: number | null): string {
  const approval = {
    id: "a",
    organizationId: "o",
    sessionId: "s",
    card: "ask_first",
    summary: "opencode asks to use external_directory: /tmp/x/*",
    payload: { kind: "ask_first", tool: "external_directory", detail: "/tmp/x/*", permission: "p" },
    costCents,
    status: "pending",
    createdAt: 0,
  } as unknown as Schemas["Approval"];
  return renderToStaticMarkup(
    createElement(
      QueryClientProvider,
      { client: new QueryClient() },
      createElement(ApprovalCard, {
        approval,
        session: "s",
        canDecide: true,
        decidedBy: "Ada",
        budgetCapCents: null,
        spentCents: undefined,
      }),
    ),
  );
}

describe("ApprovalCard", () => {
  it("says what it costs once", () => {
    expect(render(null).split("Costs nothing").length - 1).toBe(1);
    expect(render(0).split("Costs nothing").length - 1).toBe(1);
  });

  it("says what a card may cost once", () => {
    const html = render(250);
    expect(html.split("$2.50").length - 1).toBe(1);
    expect(html).not.toContain("Costs nothing");
  });
});

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { expect, test } from "@playwright/test";
import { signUp, unique } from "./helpers";

/** A tiny remote MCP server: JSON-RPC over POST, JSON answers, one tool named `lookup`. */
async function mockMcp(): Promise<{ url: string; server: Server }> {
  const server = createServer((request, response) => {
    let body = "";
    request.on("data", (chunk) => {
      body += chunk;
    });
    request.on("end", () => {
      const call = JSON.parse(body === "" ? "{}" : body) as { id?: number; method?: string };
      if (call.method === "notifications/initialized") {
        response.writeHead(202).end();
        return;
      }
      const result: Record<string, unknown> =
        call.method === "initialize"
          ? {
              protocolVersion: "2025-03-26",
              capabilities: { tools: {} },
              serverInfo: { name: "mock", version: "1" },
            }
          : call.method === "tools/list"
            ? {
                tools: [
                  {
                    name: "lookup",
                    description: "Look a thing up",
                    inputSchema: { type: "object", properties: {} },
                  },
                ],
              }
            : {};
      response.writeHead(200, {
        "content-type": "application/json",
        ...(call.method === "initialize" ? { "mcp-session-id": "e2e-session" } : {}),
      });
      response.end(JSON.stringify({ jsonrpc: "2.0", id: call.id ?? null, result }));
    });
  });
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
  const { port } = server.address() as AddressInfo;
  return { url: `http://127.0.0.1:${port}/mcp`, server };
}

test("add a skill pack by pasting a SKILL.md, then add the same name again for v2", async ({
  page,
}) => {
  await signUp(page);
  const name = unique("skill");
  const markdown = (text: string) =>
    `---\nname: ${name}\ndescription: ${text}\n---\n\n# ${name}\n\nName handlers after the route.\n`;

  await page.goto("/customize/skills");
  await expect(page.getByText("No skill packs yet.")).toBeVisible();
  await page.getByRole("link", { name: "Add skill" }).first().click();
  await page.getByLabel("Paste SKILL.md").fill(markdown("Naming rules"));
  await page.getByRole("button", { name: "Add skill" }).click();

  await expect(page.getByRole("heading", { name, level: 1 })).toBeVisible();
  const versions = page.getByRole("list", { name: "Versions", exact: true });
  await expect(versions).toContainText("v1");
  await expect(versions).toContainText("Latest");
  await expect(page.getByRole("button", { name: "Show files v1" })).toBeVisible();
  await expect(page.getByText("SKILL.md").first()).toBeVisible();

  await page.getByRole("link", { name: "Skills", exact: true }).first().click();
  const list = page.getByRole("list", { name: "Skills", exact: true });
  await expect(list).toContainText(name);
  await expect(list).toContainText("v1");

  await page.getByRole("link", { name: "Add skill" }).first().click();
  await page.getByLabel("Paste SKILL.md").fill(markdown("Naming rules, revised"));
  await page.getByRole("button", { name: "Add skill" }).click();
  await expect(page.getByRole("heading", { name, level: 1 })).toBeVisible();
  await expect(versions).toContainText("v2");
  await expect(versions).toContainText("v1");
  await expect(page.getByRole("button", { name: "Show files v2" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});

test("add a connector, set a tool On", async ({ page }, info) => {
  const mock = await mockMcp();
  try {
    await signUp(page);
    const name = unique("Orders");
    const secret = "e2e-secret-token";

    await page.goto("/customize/connectors");
    await expect(page.getByText("No connectors yet.")).toBeVisible();
    await page.getByRole("link", { name: "Add connector" }).first().click();
    await page.getByLabel(/^Name/).fill(name);
    await page.getByLabel("URL").fill(mock.url);
    await page.getByLabel("Credential (optional)").fill(secret);
    await page.getByRole("button", { name: "Add connector" }).click();

    await expect(page.getByRole("heading", { name, level: 1 })).toBeVisible();
    await expect(page.getByText("Connected", { exact: true })).toBeVisible();
    // The credential is sealed: it is never on the page.
    await expect(page.locator("body")).not.toContainText(secret);

    // A new tool starts as Ask first.
    const mode = page.getByRole("combobox", { name: "Mode for lookup" });
    await expect(mode).toContainText("Ask first");
    await mode.click();
    await page.getByRole("option", { name: "On", exact: true }).click();
    await expect(mode).toContainText("On");
    await page.screenshot({ path: `test-results/connector-${info.project.name}.png` });

    // The list says Connected, never Not checked, for a connector that was checked.
    await page.getByRole("link", { name: "Connectors", exact: true }).first().click();
    await expect(page.getByRole("list", { name: "Connectors", exact: true })).toContainText(
      "Connected",
    );

    await page.goto("/customize/connectors");
    await page.getByRole("link", { name: `Configure ${name}` }).click();
    await page.getByRole("button", { name: `Remove ${name}` }).click();
    await page.getByRole("alertdialog").getByRole("button", { name: "Remove connector" }).click();
    await expect(page).toHaveURL(/\/customize\/connectors$/);
    await expect(page.getByText("No connectors yet.")).toBeVisible();
  } finally {
    mock.server.close();
  }
});

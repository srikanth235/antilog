import { expect, test } from "@playwright/test";
import { seedSession, signUp } from "./helpers";

// There is no machine behind the e2e server, so the terminal's two calls are answered here: the
// request for a terminal, and the browser's WebSocket, which plays a shell that answers
// `echo hi`. The relay and the runner are tested in Rust; this checks the panel.
const ticket = { terminal: "11111111-1111-4111-8111-111111111111", token: "tok", expiresAt: 0 };

test.describe("the terminal panel", () => {
  test("opens a shell and shows what the machine writes back", async ({ page }) => {
    await signUp(page);
    const path = await seedSession(page, "Try the terminal");
    const asked: unknown[] = [];
    await page.route("**/api/sessions/*/terminals", async (route) => {
      asked.push(route.request().postDataJSON());
      await route.fulfill({ status: 201, json: ticket });
    });
    let url = "";
    await page.routeWebSocket(/\/api\/terminals\/.*\/connect/, (socket) => {
      url = socket.url();
      // The shell titles its window `user@machine: path`; the panel shows the folder.
      socket.send(
        Buffer.from("\x1b]0;agent@sandbox: ~/acme/checkout\x07agent@sandbox:~/acme/checkout$ "),
      );
      let line = "";
      socket.onMessage((message) => {
        // Keys arrive as bytes, a few at a time; a resize is text.
        if (typeof message === "string") return;
        for (const key of message.toString()) {
          if (key === "\r") {
            socket.send(
              Buffer.from(
                `\r\n${line === "echo hi" ? "hi" : "?"}\r\nagent@sandbox:~/acme/checkout$ `,
              ),
            );
            line = "";
          } else {
            line += key;
            socket.send(Buffer.from(key));
          }
        }
      });
    });

    await page.goto(`${path}?panel=terminal`);
    const panel = page.getByRole("complementary", { name: "Terminal" });
    await expect(panel.getByRole("heading", { name: "Terminal" })).toBeVisible();
    await expect(panel.locator(".xterm-rows")).toContainText("agent@sandbox", { timeout: 15_000 });
    await expect(panel.locator(".term-dir")).toHaveText("checkout");
    expect(url).toContain(`/api/terminals/${ticket.terminal}/connect?token=tok`);
    expect(asked).toHaveLength(1);
    const size = asked[0] as { cols: number; rows: number };
    expect(size.cols).toBeGreaterThan(10);
    expect(size.rows).toBeGreaterThan(3);

    await panel.locator(".xterm").click();
    await page.keyboard.type("echo hi");
    await page.keyboard.press("Enter");
    await expect(panel.locator(".xterm-rows")).toContainText("hi", { timeout: 10_000 });
    await expect(panel.locator(".xterm-rows")).toContainText("echo hi");

    // + opens another tab, and its own terminal.
    await panel.getByRole("button", { name: "New terminal" }).click();
    await expect(panel.getByRole("tab")).toHaveCount(2);
    expect(asked).toHaveLength(2);
    await panel.getByRole("button", { name: "Close Terminal 2" }).click();
    await expect(panel.getByRole("tab")).toHaveCount(0);

    // Maximise takes the chat's room; closing the panel hides it.
    await panel.getByRole("button", { name: "Maximise Terminal panel" }).click();
    await expect(panel).toHaveClass(/max/);
    await panel.getByRole("button", { name: "Close Terminal panel" }).click();
    await expect(panel).toBeHidden();
  });

  test("a machine that is not running offers Resume, then opens the terminal", async ({ page }) => {
    await signUp(page);
    const path = await seedSession(page, "Resume the terminal");
    let calls = 0;
    await page.route("**/api/sessions/*/terminals", async (route) => {
      calls += 1;
      if (calls === 1)
        await route.fulfill({
          status: 409,
          json: {
            code: "not_running",
            message: "The session's machine is not running.",
            fields: [],
          },
        });
      else await route.fulfill({ status: 201, json: ticket });
    });
    await page.routeWebSocket(/\/api\/terminals\/.*\/connect/, (socket) => {
      socket.send(Buffer.from("resumed$ "));
    });

    await page.goto(`${path}?panel=terminal`);
    await expect(page.getByText("Resume to use the terminal")).toBeVisible({ timeout: 15_000 });
    await page.getByRole("button", { name: "Resume", exact: true }).click();
    await expect(page.locator(".xterm-rows")).toContainText("resumed$", { timeout: 15_000 });
  });
});

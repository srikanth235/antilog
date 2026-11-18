import { expect, test } from "@playwright/test";
import { createProject, signUp, unique } from "./helpers";

test("a card appears without a reload, and nothing polls while the stream delivers", async ({
  page,
}) => {
  // `history` is a tool call that opens a session, and a session is an event: keep it out of the count.
  await page.route("**/api/experiments/*/tools/history", (route) =>
    route.fulfill({ json: { answer: [] } }),
  );
  await signUp(page);
  await createProject(page, unique("Checkout"));
  await page.getByRole("link", { name: "New experiment" }).first().click();
  await page.getByLabel("New experiment").fill("Cut checkout p95 latency");
  await page.getByLabel("Budget ($)").fill("10");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(
    page.getByRole("heading", { name: "Cut checkout p95 latency", level: 1 }),
  ).toBeVisible();
  await expect(page.getByText("Started a session on the fake platform")).toBeVisible();
  const id = page.url().split("/experiments/")[1]?.split(/[?#]/)[0];

  // Let the stream's first message refetch what the page shows, then count what is asked.
  await page.waitForTimeout(1500);
  const polled: string[] = [];
  page.on("request", (request) => {
    if (request.url().includes("/api/approvals/pending")) polled.push(request.url());
  });
  // The old poll asks every 4 seconds: a quiet 5 seconds with no ask means the stream is live.
  await page.waitForTimeout(5000);
  expect(polled).toEqual([]);

  // Only the stream can bring this card in time: no poll runs and the page is not reloaded.
  await page.evaluate(() => {
    (window as unknown as { kept: boolean }).kept = true;
  });
  const asked = await page.request.post(`/api/experiments/${id}/tools/request_budget`, {
    data: { input: { amount_cents: 5000, reason: "more trials" } },
  });
  expect(asked.ok()).toBe(true);
  await expect(page.getByRole("region", { name: "Raise budget" })).toBeVisible({ timeout: 3000 });
  expect(await page.evaluate(() => (window as unknown as { kept?: boolean }).kept)).toBe(true);
});

test("when the stream fails the page polls as before", async ({ page }) => {
  await page.route("**/api/events/stream", (route) => route.abort());
  await page.route("**/api/experiments/*/tools/history", (route) =>
    route.fulfill({ json: { answer: [] } }),
  );
  await signUp(page);
  await createProject(page, unique("Checkout"));
  await page.getByRole("link", { name: "New experiment" }).first().click();
  await page.getByLabel("New experiment").fill("Cut checkout p95 latency");
  await page.getByLabel("Budget ($)").fill("10");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText("Started a session on the fake platform")).toBeVisible();
  const id = page.url().split("/experiments/")[1]?.split(/[?#]/)[0];
  const asked = await page.request.post(`/api/experiments/${id}/tools/request_budget`, {
    data: { input: { amount_cents: 5000, reason: "more trials" } },
  });
  expect(asked.ok()).toBe(true);
  await expect(page.getByRole("region", { name: "Raise budget" })).toBeVisible({ timeout: 15_000 });
});

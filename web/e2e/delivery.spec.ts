import { createHmac } from "node:crypto";
import { expect, test } from "@playwright/test";
import { createProject, signUp, unique } from "./helpers";

// The server is started with these (playwright.config.ts). Against a server that is not, the test skips.
const SECRET = "e2e-code-host-secret";

function signed(body: string): Record<string, string> {
  return {
    "content-type": "application/json",
    "x-github-event": "pull_request",
    "x-hub-signature-256": `sha256=${createHmac("sha256", SECRET).update(body).digest("hex")}`,
  };
}

test("a win merged through the code host's webhook is Delivered on the Report and the receipt", async ({
  page,
}) => {
  await signUp(page);
  await createProject(page, unique("Checkout"));
  await page.getByRole("link", { name: "New experiment" }).first().click();
  await page.getByLabel("New experiment").fill("Cut checkout p95 latency");
  await page.getByLabel("Budget ($)").fill("10");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(
    page.getByRole("heading", { name: "Cut checkout p95 latency", level: 1 }),
  ).toBeVisible();
  const id = page.url().split("/experiments/")[1]?.split(/[?#]/)[0];

  const experiment = await (await page.request.get(`/api/experiments/${id}`)).json();
  const model = await page.request.post(`/api/projects/${experiment.projectId}/value-models`, {
    data: {
      metric: "p95",
      unit: "ms",
      centsPerUnit: 100,
      source: { text: "Finance, Q3 2026" },
    },
  });
  expect(model.ok()).toBe(true);

  // A Kept trial (100 ms down to 80 ms, so $20 a year) with a signed receipt and an open pull request.
  const seeded = await page.request.post(`/api/internal/e2e/${id}/win`);
  test.skip(seeded.status() === 404, "this server is not started with ANTILOG_E2E_SEED=1");
  expect(seeded.ok()).toBe(true);
  const win = await seeded.json();
  const pr = win.pullRequests[0];

  // Pending is not Delivered and not in Return.
  await page.goto("/report");
  const figures = page.getByRole("region", { name: "The month in figures" });
  await expect(figures.getByText("1 pull request open")).toBeVisible();
  await expect(figures.getByText("no merged wins")).toBeVisible();

  const merged = JSON.stringify({
    action: "closed",
    number: pr.number,
    pull_request: { merged: true },
    repository: { full_name: pr.repo },
  });
  const delivery = unique("delivery");
  const send = (headers: Record<string, string>, body = merged) =>
    page.request.post("/api/webhooks/code-host", { headers, data: body });
  test.skip((await send({})).status() === 404, "the webhook is not mounted on this server");

  // A missing or wrong signature is refused; a signed one is applied once.
  expect((await send({ "x-github-event": "pull_request" })).status()).toBe(401);
  const wrong = { ...signed(merged), "x-hub-signature-256": `sha256=${"0".repeat(64)}` };
  expect((await send({ ...wrong, "x-github-delivery": delivery })).status()).toBe(401);
  const good = { ...signed(merged), "x-github-delivery": delivery };
  const applied = await send(good);
  expect(applied.status()).toBe(200);
  expect((await applied.json()).status).toBe("merged");
  expect((await (await send(good)).json()).status).toBe("duplicate");

  await page.goto("/report");
  await expect(figures.getByText("1 merged win")).toBeVisible();
  await expect(figures.getByText("$20").first()).toBeVisible();
  await expect(figures.getByText("no pull request open")).toBeVisible();

  await page.goto(`/receipts/${win.trial}`);
  await expect(page.getByText("Delivered", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("$20 a year").first()).toBeVisible();
});

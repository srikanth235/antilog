import { expect, test } from "@playwright/test";
import { createProject, signUp, unique } from "./helpers";

test("the composer's options sit on one row and the agent sentence is in its tooltip", async ({
  page,
}) => {
  await signUp(page);
  await createProject(page, unique("Checkout"));
  await page.getByRole("link", { name: "New experiment" }).first().click();
  const names = ["Project", "Agent", "Environment", "Mode"];
  const tops: number[] = [];
  for (const name of names) {
    const box = await page.getByRole("combobox", { name, exact: true }).boundingBox();
    expect(box, name).not.toBeNull();
    tops.push(Math.round(box?.y ?? 0));
  }
  const budget = await page.getByLabel("Budget ($)").boundingBox();
  tops.push(Math.round(budget?.y ?? 0));
  const spread = Math.max(...tops) - Math.min(...tops);
  expect(spread).toBeLessThan(12);
  // The words fit: the budget field is not cut off.
  await expect(page.getByPlaceholder("Set a budget")).toBeVisible();
  await expect(page.getByRole("status")).toHaveCount(0);
  await expect(page.getByRole("combobox", { name: "Agent", exact: true })).toHaveAttribute(
    "title",
    /Auto would choose .+: .+/,
  );
});

test("the conversation reads in time order and sessions carry the agent's name", async ({
  page,
}) => {
  await signUp(page);
  await createProject(page, unique("Checkout"));
  await page.getByRole("link", { name: "New experiment" }).first().click();
  await page.getByLabel("New experiment").fill("Cut checkout p95 latency");
  await page.getByLabel("Budget ($)").fill("10");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText(/^Started a session on/)).toBeVisible();
  const id = page.url().split("/experiments/")[1]?.split(/[?#]/)[0];
  const asked = await page.request.post(`/api/experiments/${id}/tools/request_budget`, {
    data: { input: { amount_cents: 5000, reason: "more trials" } },
  });
  expect(asked.ok()).toBe(true);
  const card = page.getByRole("region", { name: "Raise budget" });
  await expect(card).toBeVisible({ timeout: 15_000 });

  // The person's opening message is first, then the lines and the card, in the order they happened.
  const opening = await page.locator(".bubble").first().boundingBox();
  const started = await page.getByText(/^Started a session on/).boundingBox();
  const raised = await card.boundingBox();
  expect(opening?.y ?? 0).toBeLessThan(started?.y ?? 0);
  expect(started?.y ?? 0).toBeLessThan(raised?.y ?? 0);

  // The card shows the rows the API supports.
  for (const row of ["Now", "Requested", "Why", "What it may cost"])
    await expect(card.getByText(row, { exact: true })).toBeVisible();
  await expect(card.getByText("$60", { exact: true })).toBeVisible();

  // Names are the agent's display name, never an id turned into words.
  await expect(page.getByText(/Agentcore/)).toHaveCount(0);
  await expect(page.getByText(/\bMcp\b/)).toHaveCount(0);
  await expect(page.getByText(/connected from your machine/).first()).toBeVisible();
});

test("settings opens on its search field and members are added from a dialog", async ({ page }) => {
  await signUp(page);
  await page.goto("/settings/members");
  const dialog = page.getByRole("dialog", { name: "Settings" });
  await expect(dialog).toBeVisible();
  await expect(page.getByLabel("Search settings")).toBeFocused();
  await expect(dialog.getByRole("button", { name: "Close" })).not.toBeFocused();

  // The rail filters.
  await page.getByLabel("Search settings").fill("audit");
  await expect(dialog.getByRole("link", { name: "Audit log" })).toBeVisible();
  await expect(dialog.getByRole("link", { name: "Profile" })).toHaveCount(0);
  await page.getByLabel("Search settings").fill("");

  // Add member opens from a button; the provider is a select, not free text.
  await expect(page.getByLabel("Name", { exact: true })).toHaveCount(0);
  await dialog.getByRole("button", { name: "Add member" }).click();
  const add = page.getByRole("dialog", { name: "Add member" });
  await expect(add.getByLabel("Name", { exact: true })).toBeFocused();
  await expect(add.getByRole("combobox", { name: "Sign-in provider" })).toBeVisible();
  await expect(add.getByRole("textbox", { name: "Sign-in provider" })).toHaveCount(0);
});

import { expect, test } from "@playwright/test";
import { createProject, signUp, unique } from "./helpers";

test("signed out, the app asks for sign-in and offers no password", async ({ page }) => {
  await page.goto("/projects");
  await expect(page).toHaveURL(/\/signin/);
  await expect(page.getByRole("heading", { name: "Sign in to Antilog" })).toBeVisible();
  await expect(page.locator("input[type=password]")).toHaveCount(0);
});

test("a refused sign-in says so in one sentence", async ({ page }) => {
  await page.goto("/signin?error=refused");
  await expect(page.getByRole("alert")).toContainText("Sign-in refused.");
});

test("the theme follows the system", async ({ page }, info) => {
  await signUp(page);
  const dark = info.project.name === "dark";
  if (dark) await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  else await expect(page.locator("html")).not.toHaveAttribute("data-theme", "dark");
});

test("start an experiment, approve a card, read the settings", async ({ page }, info) => {
  await signUp(page);
  const project = unique("Checkout");
  await createProject(page, project);
  await expect(page.getByText("No experiments in this project yet.")).toBeVisible();

  // Home: the composer is the page.
  await page.getByRole("link", { name: "New experiment" }).first().click();
  await page.getByLabel("New experiment").fill("Cut checkout p95 latency");
  // No default budget anywhere: the person must enter one, and sees it.
  await expect(page.getByRole("button", { name: "Send" })).toBeDisabled();
  await expect(page.getByPlaceholder("Set a budget")).toBeVisible();
  await page.getByLabel("Budget ($)").fill("10");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(
    page.getByRole("heading", { name: "Cut checkout p95 latency", level: 1 }),
  ).toBeVisible();
  await expect(page.getByText(/Started a session with/)).toBeVisible();
  // The conversation comes from the server: the prompt is there once, not twice.
  await expect(page.locator(".bubble", { hasText: "Cut checkout p95 latency" })).toHaveCount(1);

  // The agent asks for more budget: a card appears, and a person decides.
  const id = page.url().split("/experiments/")[1]?.split(/[?#]/)[0];
  const asked = await page.request.post(`/api/experiments/${id}/tools/request_budget`, {
    data: { input: { amount_cents: 5000, reason: "more trials" } },
  });
  expect(asked.ok()).toBe(true);
  const card = page.getByRole("region", { name: "Raise budget" });
  await expect(card).toBeVisible({ timeout: 15_000 });
  await expect(card.getByText("Nothing happens until you approve.")).toBeVisible();
  await page.screenshot({ path: `test-results/experiment-${info.project.name}.png` });
  await card.getByRole("button", { name: "Raise budget" }).click();
  await expect(page.getByText(/Raise budget: approved by/)).toBeVisible();

  // Settings: members, tokens, audit log.
  await page.goto("/settings/members");
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
  await expect(page.getByRole("list", { name: "Members" })).toContainText("Owner");
  await page.getByRole("link", { name: "Access tokens" }).click();
  await page.getByLabel("Name").fill("Laptop CLI");
  await page.getByRole("button", { name: "New token" }).click();
  await expect(page.getByTestId("token-secret")).toContainText("alt_");
  await page.getByRole("link", { name: "Audit log" }).click();
  await expect(page.getByText("Card raised").first()).toBeVisible();
  await page.screenshot({ path: `test-results/settings-${info.project.name}.png` });
});

test("customize adds an evaluation and shows it", async ({ page }) => {
  await signUp(page);
  const name = unique("Load test");
  await page.getByRole("link", { name: "Customize" }).click();
  await expect(page.getByRole("heading", { name: "Customize" })).toBeVisible();
  await expect(page.getByText("No evaluations yet.")).toBeVisible();
  await page.getByRole("link", { name: "Add evaluation" }).first().click();
  await page.getByLabel("Name", { exact: true }).fill(name);
  await page.getByLabel(/^Metric/).fill("p95");
  await page.getByLabel("Evaluation command").fill("k6 run bench/load.js");
  await page.getByLabel("Protected paths").fill("app:bench/");
  await page.getByRole("button", { name: "Add evaluation" }).click();
  await expect(page.getByRole("heading", { name, level: 1 })).toBeVisible();
  await expect(page.getByText("k6 run bench/load.js")).toBeVisible();
});

test("a viewer can read but not send", async ({ page, browser }) => {
  await signUp(page);
  await createProject(page, unique("Billing"));
  // The owner adds a viewer who signs in with the test provider.
  const viewer = unique("vic");
  await page.goto("/settings/members");
  await page.getByRole("button", { name: "Add member" }).click();
  const add = page.getByRole("dialog", { name: "Add member" });
  await add.getByLabel("Name", { exact: true }).fill(viewer);
  // The provider is chosen from the ones the server lists; the test server lists only the test one.
  await expect(add.getByRole("combobox", { name: "Sign-in provider" })).toContainText(
    "Test sign-in",
  );
  await add.getByLabel("Account at the provider").fill(viewer);
  await add.getByRole("combobox", { name: "Role", exact: true }).click();
  await page.getByRole("option", { name: "Viewer" }).click();
  await add.getByRole("button", { name: "Add member" }).click();
  await expect(page.getByText(viewer).first()).toBeVisible();

  const context = await browser.newContext({ baseURL: "http://127.0.0.1:4173" });
  const other = await context.newPage();
  await other.goto("/signin");
  const form = other.getByRole("form", { name: "Test sign-in" });
  await form.getByLabel("Account name").fill(viewer);
  await form.getByRole("button", { name: "Continue with a test account" }).click();
  await expect(other.getByRole("heading", { name: "What do you want to improve?" })).toBeVisible();
  await expect(other.getByLabel("New experiment")).toBeDisabled();
  await expect(
    other.getByText("Viewers read experiments. Only editors and owners can send."),
  ).toBeVisible();
  await other.goto("/settings/members");
  await expect(other.getByText("Only owners can change this. You can read it.")).toBeVisible();
  await context.close();
});

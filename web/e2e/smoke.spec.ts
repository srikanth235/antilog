import { expect, test } from "@playwright/test";
import { createProject, seedSession, signUp, unique } from "./helpers";

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

test("start a session from home, then read the settings", async ({ page }, info) => {
  await signUp(page);
  const project = unique("Checkout");
  await createProject(page, project);
  await expect(page.getByText("No sessions in this project yet.")).toBeVisible();

  // Home: the composer is the page; the server needs a model named as provider/model.
  await page.getByRole("link", { name: "New session" }).first().click();
  await page.getByLabel("New session").fill("Cut checkout p95 latency");
  await expect(page.getByRole("button", { name: "Send" })).toBeDisabled();
  await page.getByLabel("Model").fill("opencode/e2e");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText(/Started a session with/)).toBeVisible();
  // The conversation comes from the server: the prompt is there once, not twice.
  await expect(page.locator(".bubble", { hasText: "Cut checkout p95 latency" })).toHaveCount(1);
  await page.screenshot({ path: `test-results/session-${info.project.name}.png` });

  // Settings: members, tokens, audit log.
  await page.goto("/settings/members");
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
  await expect(page.getByRole("list", { name: "Members" })).toContainText("Owner");
  await page.getByRole("link", { name: "Access tokens" }).click();
  await page.getByLabel("Name").fill("Laptop CLI");
  await page.getByRole("button", { name: "New token" }).click();
  await expect(page.getByTestId("token-secret")).toContainText("alt_");
  await page.getByRole("link", { name: "Audit log" }).click();
  await page.screenshot({ path: `test-results/settings-${info.project.name}.png` });
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
  await expect(other.getByRole("heading", { name: "What should the agent do?" })).toBeVisible();
  await expect(other.getByLabel("New session")).toBeDisabled();
  await expect(
    other.getByText("Viewers read sessions. Only editors and owners can send."),
  ).toBeVisible();
  await other.goto("/settings/members");
  await expect(other.getByText("Only owners can change this. You can read it.")).toBeVisible();
  await context.close();
});

test("an owner saves, replaces and removes a model key, and it is never shown again", async ({
  page,
}) => {
  await signUp(page);
  await page.goto("/settings/model-keys");
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
  // Every provider the page lists starts without a key; the test counts them, so a new provider needs no edit here.
  const add = page.getByRole("button", { name: /^Add key / });
  await expect(add.first()).toBeVisible();
  const providers = await add.count();
  expect(providers).toBeGreaterThanOrEqual(3);
  await expect(page.getByText("No key")).toHaveCount(providers);

  await page.getByRole("button", { name: "Add key Anthropic" }).click();
  await page.getByLabel("Anthropic key").fill("sk-ant-e2e-secret-value");
  await page.getByRole("button", { name: "Save key" }).click();
  await expect(page.getByText("Key saved")).toBeVisible();
  await expect(page.getByText("Key set")).toHaveCount(1);
  await expect(page.getByText("No key")).toHaveCount(providers - 1);
  // Nothing on the page carries the key, and a reload still does not.
  await page.reload();
  await expect(page.getByText("Key set")).toHaveCount(1);
  expect(await page.content()).not.toContain("sk-ant-e2e-secret-value");

  await page.getByRole("button", { name: "Replace key Anthropic" }).click();
  await page.getByLabel("Anthropic key").fill("sk-ant-e2e-second-value");
  await page.getByRole("button", { name: "Save key" }).click();
  await expect(page.getByText("Key saved")).toBeVisible();

  await page.getByRole("button", { name: "Remove Anthropic" }).click();
  await page.getByRole("button", { name: "Remove", exact: true }).last().click();
  await expect(page.getByText("Key removed")).toBeVisible();
  await expect(page.getByText("No key")).toHaveCount(providers);

  await page.getByRole("link", { name: "Audit log" }).click();
  await expect(page.getByText("Model key set").first()).toBeVisible();
  await expect(page.getByText("Model key removed").first()).toBeVisible();
});

test("creating a project opens its page, and New project is a normal-sized button", async ({
  page,
}) => {
  await signUp(page);
  await page.goto("/projects");
  const button = page.getByRole("button", { name: "New project" }).first();
  const content = await page.locator(".page").boundingBox();
  const box = await button.boundingBox();
  expect(box && content && box.width < content.width / 2).toBe(true);
  expect(box?.height).toBeLessThan(48);

  const name = unique("Launch");
  await createProject(page, name);
  await expect(page).toHaveURL(/\/projects\/[0-9a-f-]+$/);
  await expect(page.getByText("No sessions in this project yet.")).toBeVisible();
});

test("a session no runner holds reads Queued, and one a person stopped reads Stopped", async ({
  page,
}) => {
  await signUp(page);
  const path = await seedSession(page, "Wait for a machine");
  await page.goto(path);
  await expect(page.getByRole("img", { name: "queued" }).first()).toBeVisible();
  await expect(page.getByText("Queued").first()).toBeVisible();
  await expect(page.getByRole("img", { name: "working" })).toHaveCount(0);

  const id = path.split("/").pop();
  const stopped = await page.request.post(`/api/sessions/${id}/stop`, { data: {} });
  expect(stopped.ok()).toBe(true);
  await page.reload();
  await expect(page.getByRole("img", { name: "stopped" }).first()).toBeVisible();
  await expect(page.getByText("Stopped").first()).toBeVisible();
  await expect(page.getByRole("img", { name: "done" })).toHaveCount(0);
});

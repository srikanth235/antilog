import { expect, test } from "@playwright/test";
import { createProject, signUp, unique } from "./helpers";

test("an owner adds a value model and sees it on the Project page", async ({ page }, info) => {
  await signUp(page);
  const project = unique("Checkout");
  await createProject(page, project);

  // No model yet: the section says so, and never shows it as zero dollars.
  const value = page.getByRole("region", { name: "Value", exact: true });
  await expect(value.getByText("No value set")).toBeVisible();
  const strip = page.getByRole("region", { name: "Value of this project's wins" });
  await expect(strip).toContainText("Delivered");
  await expect(strip).toContainText("measured offline");
  await expect(strip.getByText("No spend yet")).toBeVisible();

  await value.getByRole("button", { name: "Add value model" }).click();
  const dialog = page.getByRole("dialog", { name: "Add value model" });
  await expect(dialog).toBeVisible();

  // The form checks itself before it asks the server.
  await dialog.getByRole("button", { name: "Add value model" }).click();
  await expect(dialog.getByText("Name the metric.")).toBeVisible();
  await expect(dialog.getByText("Say where the figure comes from.")).toBeVisible();
  await dialog.getByLabel("Metric").fill("Checkout p95 latency");
  await dialog.getByLabel("Unit label").fill("ms");
  await dialog.getByLabel("One unit").fill("100");
  await dialog.getByLabel("Is worth, dollars a year").fill("$3,600");
  await expect(
    dialog.getByText(
      "Each 100 ms of Checkout p95 latency is worth $3,600 a year. A win of 400 ms would be worth $14,400 a year.",
    ),
  ).toBeVisible();
  await dialog.getByLabel("Source", { exact: true }).fill("Checkout conversion study, Q3 2026");
  await dialog.getByLabel("Link").fill("ftp://example.com");
  await dialog.getByRole("button", { name: "Add value model" }).click();
  await expect(dialog.getByText("A link starts with http:// or https://.")).toBeVisible();
  await dialog.getByLabel("Link").fill("https://example.com/study");
  await page.screenshot({ path: `test-results/add-value-model-${info.project.name}.png` });
  await dialog.getByRole("button", { name: "Add value model" }).click();
  await expect(dialog).toBeHidden();

  await expect(value.getByText("Each 100 ms is worth $3,600 a year")).toBeVisible();
  await expect(value.getByText("Checkout conversion study, Q3 2026")).toBeVisible();
  await expect(value.getByText("v1", { exact: true })).toBeVisible();
  await expect(value.getByText("No value set")).toHaveCount(0);

  // A change to the same metric is the next version.
  await value
    .getByRole("button", { name: "Change the value model for Checkout p95 latency" })
    .click();
  const change = page.getByRole("dialog", { name: "Change value model" });
  await change.getByLabel("Is worth, dollars a year").fill("4800");
  await change.getByRole("button", { name: "Change value model" }).click();
  await expect(change).toBeHidden();
  await expect(value.getByText("Each 100 ms is worth $4,800 a year")).toBeVisible();
  await expect(value.getByText("v2", { exact: true })).toBeVisible();
  await page.screenshot({ path: `test-results/project-value-${info.project.name}.png` });
});

test("a viewer sees the value models but cannot add one", async ({ page, browser }) => {
  await signUp(page);
  const project = unique("Billing");
  await createProject(page, project);
  const viewer = unique("vic");
  await page.goto("/settings/members");
  await page
    .getByRole("dialog", { name: "Settings" })
    .getByRole("button", { name: "Add member" })
    .click();
  const add = page.getByRole("dialog", { name: "Add member" });
  await add.getByLabel("Name", { exact: true }).fill(viewer);
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
  await other.goto("/projects");
  await other.getByRole("link", { name: new RegExp(project) }).click();
  const button = other.getByRole("button", { name: "Add value model" });
  await expect(button).toBeDisabled();
  await expect(button).toHaveAttribute("title", "Only owners can change this. You can read it.");
  await context.close();
});

test("the Report says plainly that a month had no spend", async ({ page }, info) => {
  await signUp(page);
  await page.goto("/projects");
  await page.getByRole("link", { name: "Report" }).click();
  await expect(page).toHaveURL(/\/report/);
  await expect(page.getByRole("heading", { name: "Report", level: 1 })).toBeVisible();
  await expect(page.getByText("Nothing was spent in this month.")).toBeVisible();

  const tiles = page.getByRole("region", { name: "The month in figures" });
  await expect(tiles.getByText("Spent", { exact: true })).toBeVisible();
  await expect(tiles).toContainText("nothing spent in this month");
  await expect(tiles).toContainText("measured offline");
  await expect(tiles.getByText("No spend yet")).toBeVisible();
  await expect(tiles).not.toContainText("NaN");
  await expect(tiles).not.toContainText("Infinity");
  // No table of zeros-as-data.
  await expect(page.getByRole("heading", { name: "Agent scoreboard" })).toHaveCount(0);
  await page.screenshot({ path: `test-results/report-${info.project.name}.png` });

  // The month can be changed; an older month is just as plain.
  await page.getByRole("combobox", { name: "Month" }).click();
  const older = page.getByRole("option").nth(1);
  const name = (await older.textContent()) ?? "";
  await older.click();
  await expect(page.getByText("Nothing was spent in this month.")).toBeVisible();
  await expect(page.locator(".tag", { hasText: name })).toBeVisible();
});

import { expect, test } from "@playwright/test";
import { signUp, unique } from "./helpers";

test("create an environment, change it into a new version, see it offered to a new session, then archive it", async ({
  page,
}) => {
  await signUp(page);
  const name = unique("Bench");
  await page.goto("/settings/connections");
  const rows = page.locator("[data-environment]");
  // The built-in default is always there and cannot be changed.
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("Default");
  await expect(rows.first()).toContainText("Built in");

  await page.getByRole("button", { name: "Add environment" }).click();
  await expect(page.getByRole("heading", { name: "Add an environment" })).toBeVisible();
  // The variables carry the warning that the agent reads them and a model key never goes here.
  await expect(
    page.getByText("The agent can read these variables.", { exact: false }),
  ).toBeVisible();
  await expect(page.getByText("A model key never goes here", { exact: false })).toBeVisible();
  await page.getByLabel("Name", { exact: true }).fill(name);
  await page.getByLabel("Allowed hosts").fill("API.stripe.com\nghcr.io");
  await page.getByLabel("Variables", { exact: true }).fill("STRIPE_TEST_KEY=sk_test_x");
  await page.getByLabel("Setup script").fill("npm ci");
  await page.getByRole("button", { name: "Add environment" }).click();

  const mine = page.locator(`[data-environment="${name.toLowerCase()}"]`);
  await expect(mine).toContainText(name);
  await expect(mine).toContainText("Limited network, 2 hosts, package registries");
  await expect(mine).toContainText("2 vCPU, 8 GiB");
  await expect(mine).toContainText("Version 1");

  // A host that is not a name is refused with the server's reason, and the form stays.
  await mine.getByRole("button", { name: `Edit ${name}` }).click();
  await expect(page.getByRole("heading", { name: `Change ${name}` })).toBeVisible();
  await page.getByLabel("Allowed hosts").fill("*.stripe.com");
  await page.getByRole("button", { name: "Save as version 2" }).click();
  await expect(page.getByRole("alert")).toContainText("exact host name");
  // A variable line that is not NAME=value is refused before it is sent.
  await page.getByLabel("Allowed hosts").fill("api.stripe.com");
  await page.getByLabel("Variables", { exact: true }).fill("not a variable");
  await page.getByRole("button", { name: "Save as version 2" }).click();
  await expect(page.getByRole("alert")).toContainText("NAME=value");

  // A good change is the next version.
  await page.getByLabel("Variables", { exact: true }).fill("STRIPE_TEST_KEY=sk_test_y");
  await page
    .getByRole("button", { name: "None" })
    .or(page.getByRole("button", { name: "No network" }))
    .click();
  await page.getByRole("button", { name: "Save as version 2" }).click();
  await expect(mine).toContainText("Version 2");
  await expect(mine).toContainText("No network");

  // The new session's Environment chip offers it.
  await page.goto("/");
  await page.getByRole("combobox", { name: "Environment" }).click();
  await expect(page.getByRole("option", { name })).toBeVisible();
  await page.keyboard.press("Escape");

  // Archived, it is no longer offered and cannot be changed.
  await page.goto("/settings/connections");
  await mine.getByRole("button", { name: `Archive ${name}` }).click();
  await page.getByRole("button", { name: "Archive", exact: true }).click();
  await expect(mine).toContainText("Archived");
  await expect(mine.getByRole("button", { name: `Edit ${name}` })).toHaveCount(0);
  await page.goto("/");
  await page.getByRole("combobox", { name: "Environment" }).click();
  await expect(page.getByRole("option", { name })).toHaveCount(0);
});

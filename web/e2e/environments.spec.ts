import { expect, test } from "@playwright/test";
import { createProject, signUp, unique } from "./helpers";

test("create an environment, change it into a new version, see it offered to a new session, then archive it", async ({
  page,
}) => {
  await signUp(page);
  // A new session needs a project; without one the composer, and its Environment chip, is not offered.
  await createProject(page, unique("Bench project"));
  const name = unique("Bench");
  await page.goto("/settings/connections");
  const rows = page.locator("[data-environment]");
  // The built-in default is always there and cannot be changed.
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("Default");
  await expect(rows.first()).toContainText("Built in");

  await page.getByRole("button", { name: "Add environment" }).click();
  const dialog = page.getByRole("dialog", { name: "Add environment" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("Name", { exact: true }).fill(name);
  // Repositories: new sessions in this environment start with them.
  await dialog.getByRole("button", { name: "Add", exact: true }).click();
  await dialog.getByPlaceholder("owner/repo").fill("not a repo");
  await dialog.getByPlaceholder("owner/repo").press("Enter");
  await expect(dialog.getByRole("alert")).toContainText("owner/name");
  await dialog.getByPlaceholder("owner/repo").fill("acme/app");
  await dialog.getByPlaceholder("owner/repo").press("Enter");
  await expect(dialog.getByRole("list", { name: "Repositories" })).toContainText("acme/app");
  // Each row shows a summary and opens its editor.
  await expect(dialog.getByRole("button", { name: /Network secrets/ })).toContainText("None");
  await dialog.getByRole("button", { name: /Network access/ }).click();
  await dialog.getByLabel("Allowed hosts").fill("API.stripe.com\nghcr.io");
  // The variables carry the warning that the agent reads them and a model key never goes here.
  await dialog.getByRole("button", { name: /Environment variables/ }).click();
  await expect(
    dialog.getByText("The agent can read these variables.", { exact: false }),
  ).toBeVisible();
  await expect(dialog.getByText("A model key never goes here", { exact: false })).toBeVisible();
  await dialog.getByLabel("Variables", { exact: true }).fill("STRIPE_TEST_KEY=sk_test_x");
  await expect(dialog.getByRole("button", { name: /Environment variables/ })).toContainText(
    "1 variable",
  );
  await dialog.getByRole("button", { name: /Setup script/ }).click();
  await dialog.getByLabel("Setup script", { exact: true }).fill("npm ci\nnpm run build");
  await expect(dialog.getByRole("button", { name: /Setup script/ })).toContainText("2 lines");
  // A network secret is typed once; its value is never shown again.
  await dialog.getByRole("button", { name: /Network secrets/ }).click();
  await dialog.getByLabel("Host", { exact: true }).fill("api.example.com");
  await dialog.getByLabel("Value", { exact: true }).fill("sk_live_hunter2");
  await dialog.getByRole("button", { name: "Add secret" }).click();
  await expect(dialog.getByRole("button", { name: /Network secrets/ })).toContainText("1 secret");
  await dialog.getByRole("button", { name: "Add environment" }).click();

  const mine = page.locator(`[data-environment="${name.toLowerCase()}"]`);
  await expect(mine).toContainText(name);
  await expect(mine).toContainText("Limited network, 2 hosts, package registries");
  await expect(mine).toContainText("2 vCPU, 8 GiB");
  await expect(mine).toContainText("Version 1");

  // Opened again, the repository and the secret are there; the secret's value is not.
  await mine.getByRole("button", { name: `Edit ${name}` }).click();
  const edit = page.getByRole("dialog", { name: "Edit environment" });
  await expect(edit.getByRole("list", { name: "Repositories" })).toContainText("acme/app");
  await edit.getByRole("button", { name: /Network secrets/ }).click();
  await expect(edit.getByRole("list", { name: "Network secrets" })).toContainText(
    "api.example.com",
  );
  await expect(edit.getByLabel("New value for api.example.com")).toHaveValue("");
  await expect(page.getByText("sk_live_hunter2")).toHaveCount(0);

  // A host that is not a name is refused with the server's reason, and the dialog stays.
  await edit.getByRole("button", { name: /Network access/ }).click();
  await edit.getByLabel("Allowed hosts").fill("*.stripe.com");
  await edit.getByRole("button", { name: "Save changes" }).click();
  await expect(edit.getByRole("alert")).toContainText("exact host name");
  // A variable line that is not NAME=value is refused before it is sent.
  await edit.getByLabel("Allowed hosts").fill("api.stripe.com");
  await edit.getByRole("button", { name: /Environment variables/ }).click();
  await edit.getByLabel("Variables", { exact: true }).fill("not a variable");
  await edit.getByRole("button", { name: "Save changes" }).click();
  await expect(edit.getByRole("alert")).toContainText("NAME=value");

  // A good change is the next version.
  await edit.getByLabel("Variables", { exact: true }).fill("STRIPE_TEST_KEY=sk_test_y");
  await edit.getByRole("button", { name: /Network access/ }).click();
  await edit.getByRole("button", { name: "No network" }).click();
  await edit.getByRole("button", { name: "Save changes" }).click();
  await expect(mine).toContainText("Version 2");
  await expect(mine).toContainText("No network");

  // The new session's Environment chip offers it.
  await page.goto("/");
  await page.getByRole("combobox", { name: "Environment" }).click();
  await expect(page.getByRole("option", { name })).toBeVisible();
  await page.keyboard.press("Escape");

  // Archived from its dialog, it is no longer offered and cannot be changed.
  await page.goto("/settings/connections");
  await mine.getByRole("button", { name: `Edit ${name}` }).click();
  await page
    .getByRole("dialog", { name: "Edit environment" })
    .getByRole("button", { name: "Archive environment" })
    .click();
  await page.getByRole("button", { name: "Archive", exact: true }).click();
  await expect(mine).toContainText("Archived");
  await expect(mine.getByRole("button", { name: `Edit ${name}` })).toHaveCount(0);
  await page.goto("/");
  await page.getByRole("combobox", { name: "Environment" }).click();
  await expect(page.getByRole("option", { name })).toHaveCount(0);
});

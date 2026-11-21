import { expect, test } from "@playwright/test";
import { createProject, signUp, unique } from "./helpers";

test("pick a model and effort at the start, then switch both mid-session", async ({ page }) => {
  await signUp(page);
  await createProject(page, unique("Model project"));
  // Without a key no model is reachable: the control is there, switched off, and says why.
  await page.goto("/");
  const picker = page.getByRole("button", { name: "Model and effort" });
  await expect(picker).toBeDisabled();
  const key = await page.request.put("/api/model-keys/openai", { data: { key: "sk-e2e-key" } });
  expect(key.ok()).toBe(true);
  await page.goto("/");
  await expect(picker).toBeEnabled();

  // The menu: favourites with number keys, Effort, More models.
  await picker.click();
  const menu = page.getByRole("menu", { name: "Model" });
  await expect(menu.getByRole("menuitemradio", { checked: true })).toHaveCount(1);
  await expect(menu.getByRole("menuitem", { name: /Effort/ })).toBeVisible();
  await menu.getByRole("menuitem", { name: /Effort/ }).click();
  const effort = page.getByRole("menu", { name: "Effort" });
  await expect(effort.getByRole("menuitemradio", { name: "Low", exact: true })).toBeVisible();
  await effort.getByRole("menuitemradio", { name: "High", exact: true }).click();
  await expect(picker).toContainText("High");

  // More models: searchable, grouped by provider, with the window and the price.
  await picker.click();
  await page.getByRole("menuitem", { name: "More models" }).click();
  const more = page.getByRole("region", { name: "More models" });
  await more.getByRole("searchbox", { name: "Search models" }).fill("o3");
  const o3 = more.locator('[data-model="openai/o3"]');
  await expect(o3).toContainText("200K");
  await expect(o3).toContainText("$2 / $8");
  await o3.click();
  await expect(picker).toContainText("O3");
  await expect(picker).toContainText("High");

  // Number keys work while the menu is open: the first favourite is the model just used.
  await picker.click();
  await page.keyboard.press("1");
  await expect(page.getByRole("menu", { name: "Model" })).toHaveCount(0);
  await expect(picker).toContainText("O3");

  // Start a session with it; the session composer carries the same choice.
  await page.getByLabel("Task").fill("Cut the p95");
  await page.getByRole("button", { name: "Start session" }).click();
  await expect(page).toHaveURL(/\/sessions\//);
  const inSession = page.getByRole("button", { name: "Model and effort" });
  await expect(inSession).toContainText("O3");
  await expect(inSession).toContainText("High");

  // A change applies from the next message and is noted in the transcript.
  await inSession.click();
  await page.getByRole("menuitem", { name: /Effort/ }).click();
  await page.getByRole("menuitemradio", { name: "Low", exact: true }).click();
  await expect(inSession).toContainText("Low");
  await expect(page.getByText("Switched to O3 · Low")).toBeVisible();
});

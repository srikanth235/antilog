import { expect, test } from "@playwright/test";
import { createProject, signUp, unique } from "./helpers";

test("start a session from the home page with two repos and a branch", async ({ page }) => {
  await signUp(page);
  await createProject(page, unique("Home project"));
  const key = await page.request.put("/api/model-keys/anthropic", { data: { key: "sk-ant-e2e" } });
  expect(key.ok()).toBe(true);
  await page.goto("/");

  await expect(page.getByRole("button", { name: "Repos" })).toContainText("No repository");
  await page.getByRole("button", { name: "Repos" }).click();
  const list = page.getByRole("listbox", { name: "Repos" });
  const search = list.getByRole("searchbox", { name: "Search repos" });
  await search.fill("acme/api");
  await list.getByRole("option", { name: "Use acme/api" }).click();
  await search.fill("acme/web");
  await search.press("Enter");
  await expect(list.getByRole("option", { name: "acme/api", selected: true })).toBeVisible();
  await expect(list.getByRole("option", { name: "acme/web", selected: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Repos" })).toContainText("acme/api +1");

  await page.getByRole("button", { name: "Branch" }).click();
  await page.getByRole("textbox", { name: "Branch name" }).fill("release/1");
  await page.getByRole("textbox", { name: "Branch name" }).press("Enter");
  await expect(page.getByRole("button", { name: "Branch" })).toContainText("release/1");

  await page.getByRole("button", { name: "Fix a failing test" }).click();
  await expect(page.getByLabel("Task")).toHaveValue("Fix a failing test");
  await page.getByLabel("Task").fill("Cut the p95");
  await page.getByRole("button", { name: "Start session" }).click();
  await expect(page).toHaveURL(/\/sessions\//);

  const sessions = await (await page.request.get("/api/sessions")).json();
  expect(sessions[0].repos).toEqual(["acme/api", "acme/web"]);
});

test("the model picker's menus stay inside the viewport on a short window", async ({ page }) => {
  await signUp(page);
  await createProject(page, unique("Short project"));
  await page.request.put("/api/model-keys/openai", { data: { key: "sk-e2e-key" } });
  await page.setViewportSize({ width: 1280, height: 520 });
  await page.goto("/");
  const inside = async (name: string) => {
    const box = await page
      .getByRole(name === "More models" ? "region" : "menu", { name })
      .boundingBox();
    expect(box).not.toBeNull();
    const view = page.viewportSize();
    if (box === null || view === null) return;
    expect(box.y).toBeGreaterThanOrEqual(0);
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.y + box.height).toBeLessThanOrEqual(view.height);
    expect(box.x + box.width).toBeLessThanOrEqual(view.width);
  };
  const picker = page.getByRole("button", { name: "Model and effort" });
  await picker.click();
  await inside("Model");
  await page.getByRole("menuitem", { name: /Effort/ }).click();
  await inside("Effort");
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await picker.click();
  await page.getByRole("menuitem", { name: "More models" }).click();
  await inside("More models");
  const search = await page
    .getByRole("region", { name: "More models" })
    .getByRole("searchbox")
    .boundingBox();
  expect(search?.y).toBeGreaterThanOrEqual(0);
});

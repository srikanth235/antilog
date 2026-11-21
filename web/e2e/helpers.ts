import { expect, type Page } from "@playwright/test";

let counter = 0;
/** A name no other test run used, so the tests can share one database. */
export function unique(prefix: string): string {
  counter += 1;
  return `${prefix}-${Date.now().toString(36)}${counter}`;
}

/** Signs in through the test provider, creating a new organization the person owns. */
export async function signUp(
  page: Page,
  name = unique("ada"),
): Promise<{ name: string; organization: string }> {
  const organization = unique("org");
  await page.goto("/signin");
  const form = page.getByRole("form", { name: "Test sign-in" });
  await form.getByLabel("Account name").fill(name);
  await form.getByLabel("Create a new organization").check();
  await form.getByLabel("Organization name").fill(organization);
  await form.getByRole("button", { name: "Continue with a test account" }).click();
  await expect(page.getByRole("heading", { name: "Good to see you" })).toBeVisible();
  return { name, organization };
}

export async function createProject(page: Page, name: string): Promise<void> {
  await page.goto("/projects");
  await page.getByRole("button", { name: "New project" }).first().click();
  const dialog = page.getByRole("dialog", { name: "New project" });
  await dialog.getByLabel("Name").fill(name);
  await dialog.getByRole("button", { name: "Create project" }).click();
  await expect(page.getByRole("heading", { name, level: 1 })).toBeVisible();
}

/** Creates a project and a session in it through the API, as the signed-in person; returns the session's page path. */
export async function seedSession(page: Page, title: string): Promise<string> {
  const made = await page.request.post("/api/projects", {
    data: { name: unique("Seed"), description: "" },
  });
  expect(made.ok()).toBe(true);
  const project = (await made.json()) as { id: string };
  const started = await page.request.post(`/api/projects/${project.id}/sessions`, {
    data: { title, prompt: title, model: "opencode/e2e", repos: [] },
  });
  expect(started.ok()).toBe(true);
  const session = (await started.json()) as { id: string };
  return `/sessions/${session.id}`;
}

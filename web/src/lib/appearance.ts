import { useStored, writeJson } from "./storage";

const KEY = "antilog.appearance.v1";
export type Appearance = "system" | "light" | "dark";

function parse(raw: unknown): Appearance {
  return raw === "light" || raw === "dark" ? raw : "system";
}

export function useAppearance(): Appearance {
  return useStored(KEY, parse);
}

/** "system" removes the choice; the page then follows the system again. */
export function setAppearance(choice: Appearance): void {
  writeJson(KEY, choice === "system" ? null : choice);
  const dark =
    choice === "dark" ||
    (choice === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  if (dark) document.documentElement.setAttribute("data-theme", "dark");
  else document.documentElement.removeAttribute("data-theme");
}

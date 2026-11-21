import { describe, expect, it } from "vitest";
import type { Schemas } from "../api/client";
import { filterRepos, recentRepos, reposLabel, toggleRepo, typedRepo, validBranch } from "./repos";

const session = (createdAt: number, repos: string[]) =>
  ({ createdAt, repos }) as Schemas["Session"];
const env = (repos: string[], archived = false) => ({ repos, archived }) as Schemas["Environment"];

describe("repos", () => {
  it("lists recent repos newest first, then environment defaults, once each", () => {
    const list = recentRepos(
      [session(1, ["a/old", "a/both"]), session(5, ["a/new", "a/both"])],
      [env(["a/env", "a/new"]), env(["a/gone"], true)],
    );
    expect(list).toEqual(["a/new", "a/both", "a/old", "a/env"]);
  });

  it("searches by every word, any case", () => {
    expect(filterRepos(["acme/web", "acme/api", "x/web"], "WEB acme")).toEqual(["acme/web"]);
    expect(filterRepos(["acme/web"], "")).toEqual(["acme/web"]);
  });

  it("offers a typed owner/repo that is not in the list", () => {
    expect(typedRepo(" acme/new ", ["acme/web"])).toBe("acme/new");
    expect(typedRepo("acme/web", ["acme/web"])).toBeNull();
    expect(typedRepo("acme", [])).toBeNull();
    expect(typedRepo("a/b/c", [])).toBeNull();
  });

  it("toggles a repo in and out of the choice", () => {
    expect(toggleRepo(["a/x"], "a/y")).toEqual(["a/x", "a/y"]);
    expect(toggleRepo(["a/x", "a/y"], "a/x")).toEqual(["a/y"]);
  });

  it("labels the choice", () => {
    const labels = { none: "No repository", more: (f: string, n: number) => `${f} +${n}` };
    expect(reposLabel([], labels)).toBe("No repository");
    expect(reposLabel(["a/x"], labels)).toBe("a/x");
    expect(reposLabel(["a/x", "a/y", "a/z"], labels)).toBe("a/x +2");
  });

  it("accepts ordinary branch names only", () => {
    expect(validBranch("feature/x-1.2")).toBe(true);
    expect(validBranch("a b")).toBe(false);
    expect(validBranch("a..b")).toBe(false);
    expect(validBranch("-x")).toBe(false);
  });
});

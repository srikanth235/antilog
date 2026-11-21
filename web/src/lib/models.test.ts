import { describe, expect, it } from "vitest";
import {
  byShortcut,
  contextLabel,
  defaultEffort,
  effortAfterSwitch,
  favourites,
  levelLabel,
  type ModelInfo,
  priceLabel,
  pushRecent,
  search,
} from "./models";

const model = (id: string, extra: Partial<ModelInfo> = {}): ModelInfo => {
  const [provider = "", name = ""] = id.split("/");
  return {
    id,
    name: name.toUpperCase(),
    provider: provider as ModelInfo["provider"],
    contextWindow: 200_000,
    inputPrice: 3,
    outputPrice: 15,
    reasoningLevels: ["low", "medium", "high"],
    ...extra,
  };
};

const all = [
  model("anthropic/a"),
  model("anthropic/b"),
  model("openai/c"),
  model("opencode-go/d", { reasoningLevels: [] }),
  model("opencode-go/e"),
];

describe("favourites", () => {
  it("lists the recent models that are still reachable, in that order, at most four", () => {
    const recent = ["openai/c", "gone/x", "anthropic/a", "anthropic/b", "opencode-go/e"];
    const shown = favourites(recent, all, "openai/c");
    expect(shown.map((m) => m.id)).toEqual([
      "openai/c",
      "anthropic/a",
      "anthropic/b",
      "opencode-go/e",
    ]);
  });
  it("adds the current model at the front when it was not used recently", () => {
    const shown = favourites(["anthropic/a", "anthropic/b"], all, "opencode-go/d");
    expect(shown.map((m) => m.id)).toEqual(["opencode-go/d", "anthropic/a", "anthropic/b"]);
  });
  it("is empty when nothing is reachable", () => {
    expect(favourites(["x/y"], all, null)).toEqual([]);
  });
});

describe("number shortcuts", () => {
  const shown = all.slice(0, 4);
  it("pick the model in that place", () => {
    expect(byShortcut("1", shown)?.id).toBe("anthropic/a");
    expect(byShortcut("4", shown)?.id).toBe("opencode-go/d");
  });
  it("ignore a key with no model and any other key", () => {
    expect(byShortcut("5", shown)).toBeUndefined();
    expect(byShortcut("a", shown)).toBeUndefined();
    expect(byShortcut("0", shown)).toBeUndefined();
  });
});

describe("recent models", () => {
  it("moves a model to the front and keeps four", () => {
    expect(pushRecent(["a", "b", "c", "d"], "c")).toEqual(["c", "a", "b", "d"]);
    expect(pushRecent(["a", "b", "c", "d"], "e")).toEqual(["e", "a", "b", "c"]);
  });
});

describe("search", () => {
  it("groups by provider and matches every word", () => {
    expect(search(all, "").map((g) => [g.provider, g.models.length])).toEqual([
      ["anthropic", 2],
      ["openai", 1],
      ["opencode-go", 2],
    ]);
    expect(search(all, "opencode-go/e").flatMap((g) => g.models.map((m) => m.id))).toEqual([
      "opencode-go/e",
    ]);
    expect(search(all, "zzz")).toEqual([]);
  });
});

describe("effort", () => {
  it("defaults to medium, else high, else the first level", () => {
    expect(defaultEffort(model("a/b"))).toBe("medium");
    expect(defaultEffort(model("a/b", { reasoningLevels: ["low", "high", "max"] }))).toBe("high");
    expect(defaultEffort(model("a/b", { reasoningLevels: ["max"] }))).toBe("max");
    expect(defaultEffort(model("a/b", { reasoningLevels: [] }))).toBeNull();
  });
  it("is kept across a switch only when the new model offers it", () => {
    const flash = model("a/b", { reasoningLevels: ["low", "high", "max"] });
    expect(effortAfterSwitch(flash, "high")).toBe("high");
    expect(effortAfterSwitch(flash, "medium")).toBe("high");
    expect(effortAfterSwitch(model("a/n", { reasoningLevels: [] }), "high")).toBeNull();
  });
  it("is named as a person would", () => {
    expect(levelLabel("xhigh")).toBe("Extra high");
    expect(levelLabel("medium")).toBe("Medium");
  });
});

describe("labels", () => {
  it("shows the window and the price per million tokens", () => {
    expect(contextLabel(200_000)).toBe("200K");
    expect(contextLabel(1_000_000)).toBe("1M");
    expect(contextLabel(1_050_000)).toBe("1.1M");
    expect(contextLabel(null)).toBe("");
    expect(priceLabel(model("a/b"))).toBe("$3 / $15");
    expect(priceLabel(model("a/b", { inputPrice: 0.3, outputPrice: 1.2 }))).toBe("$0.30 / $1.20");
    expect(priceLabel(model("a/b", { inputPrice: 0, outputPrice: 0 }))).toBe("Free");
    expect(priceLabel(model("a/b", { inputPrice: null, outputPrice: null }))).toBe("");
  });
});

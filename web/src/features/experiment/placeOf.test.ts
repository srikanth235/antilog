import { describe, expect, it } from "vitest";
import { placeOf } from "./ModelPicker";

const view = { width: 1440, height: 900, left: 256 };

describe("placeOf", () => {
  it("opens upward from a button near the bottom, as the home composer's", () => {
    const place = placeOf({ top: 830, bottom: 860, right: 700 }, view);
    expect(place).toEqual({ down: false, left: true, room: 812 });
  });
  it("opens downward when the room below is larger", () => {
    const place = placeOf({ top: 40, bottom: 70, right: 1400 }, view);
    expect(place.down).toBe(true);
    expect(place.room).toBe(900 - 70 - 12 - 6);
  });
  it("opens from the left when both menus would reach into the sidebar", () => {
    expect(placeOf({ top: 500, bottom: 530, right: 600 }, view).left).toBe(true);
    expect(placeOf({ top: 500, bottom: 530, right: 900 }, view).left).toBe(false);
  });
});

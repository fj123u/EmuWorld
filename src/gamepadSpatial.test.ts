import { describe, expect, it } from "vitest";
import { spatialNext, type Box } from "./gamepadSpatial";

const box = (left: number, top: number): Box => ({ left, top, width: 100, height: 130 });

// "Depuis un fichier…" button, then two grids: 3 covers, then 1 cover.
const layout: Box[] = [
  { left: 0, top: 0, width: 160, height: 30 }, // 0 button
  box(0, 60), box(110, 60), box(220, 60), //     1 2 3
  box(0, 240), //                                4
];

describe("gamepad focus in a dialog", () => {
  it("starts on the first item", () => {
    expect(spatialNext(layout, -1, "down")).toBe(0);
    expect(spatialNext([], -1, "down")).toBe(-1);
  });

  it("moves along a row and stops at its end", () => {
    expect(spatialNext(layout, 1, "right")).toBe(2);
    expect(spatialNext(layout, 2, "right")).toBe(3);
    expect(spatialNext(layout, 3, "right")).toBe(3);
    expect(spatialNext(layout, 2, "left")).toBe(1);
  });

  it("goes down to the nearest item of the next row, even a shorter one", () => {
    expect(spatialNext(layout, 0, "down")).toBe(1);
    expect(spatialNext(layout, 3, "down")).toBe(4);
    expect(spatialNext(layout, 4, "up")).toBe(1);
    expect(spatialNext(layout, 2, "up")).toBe(0);
    expect(spatialNext(layout, 4, "down")).toBe(4);
  });
});

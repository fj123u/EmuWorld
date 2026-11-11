import { describe, expect, it } from "vitest";
import { moveFocus, spatialNext, type Box } from "./gamepadSpatial";

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

describe("gamepad focus in a page grid", () => {
  // A back button, then 3 cards per row (the grid of the library / Big Picture).
  const grid: Box[] = [
    { left: 0, top: 0, width: 160, height: 40 }, // 0 back
    box(0, 60), box(110, 60), box(220, 60), //      1 2 3
    box(0, 240), box(110, 240), //                  4 5
  ];

  it("goes up and down by row, not to the next card", () => {
    expect(moveFocus(grid, 2, "down")).toBe(5);
    expect(moveFocus(grid, 5, "up")).toBe(2);
    expect(moveFocus(grid, 1, "up")).toBe(0);
    expect(moveFocus(grid, 3, "down")).toBe(5);
  });

  it("wraps to the next row at the end of a row, stays put at the bottom", () => {
    expect(moveFocus(grid, 3, "right")).toBe(4);
    expect(moveFocus(grid, 4, "left")).toBe(3);
    expect(moveFocus(grid, 4, "down")).toBe(4);
    expect(moveFocus(grid, 5, "right")).toBe(5);
  });

  it("stays in its column on a page in two columns", () => {
    // Home: the resume frame on the left, a narrow side panel on the right,
    // the side panel's button a bit higher than the frame's.
    const home: Box[] = [
      { left: 40, top: 60, width: 120, height: 50 }, //    0 section bar
      { left: 230, top: 300, width: 120, height: 44 }, //  1 Play (left column)
      { left: 1300, top: 240, width: 160, height: 44 }, // 2 Sign in (side panel)
      { left: 30, top: 420, width: 180, height: 300 }, //  3 a card under Play
      { left: 30, top: 780, width: 200, height: 200 }, //  4 a tile far below
    ];
    expect(moveFocus(home, 0, "down")).toBe(1);
    expect(moveFocus(home, 1, "down")).toBe(3);
    // From the right of the page, the next row down, not a row further.
    expect(moveFocus([{ left: 1340, top: 215, width: 190, height: 18 }, home[3], home[4]], 0, "down")).toBe(1);
  });

  it("never lands on a hidden item", () => {
    const hidden: Box = { left: 0, top: 0, width: 0, height: 0 };
    expect(moveFocus([box(0, 0), hidden, box(110, 0)], 0, "right")).toBe(2);
    expect(moveFocus([box(0, 0), hidden, box(0, 200)], 2, "up")).toBe(0);
  });
});

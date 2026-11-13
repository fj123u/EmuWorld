// The pointer is hidden while the controller drives the app, and comes back
// when the mouse is really used.
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { PAD_INPUT_CLASS, hideCursorForPad, padIsUsed, showCursorOnMouseUse } from "./padCursor";

const hidden = () => document.documentElement.classList.contains(PAD_INPUT_CLASS);
const move = (screenX: number, screenY: number) =>
  window.dispatchEvent(new MouseEvent("mousemove", { screenX, screenY }));

let stop: () => void;
beforeEach(() => { stop = showCursorOnMouseUse(); });
afterEach(() => stop());

describe("padIsUsed", () => {
  it("is true for a pressed button or a pushed stick, not for a resting pad", () => {
    expect(padIsUsed([false, true], [0, 0])).toBe(true);
    expect(padIsUsed([false], [0, -0.9])).toBe(true);
    expect(padIsUsed([false, false], [0.1, -0.2])).toBe(false);
    expect(padIsUsed([], [])).toBe(false);
    expect(padIsUsed(undefined, undefined)).toBe(false);
  });
});

describe("pointer while using the pad", () => {
  it("stays hidden when the page scrolls under a still mouse", () => {
    hideCursorForPad();
    move(300, 300);
    move(300, 300);
    move(302, 301);
    expect(hidden()).toBe(true);
  });

  it("comes back when the mouse really moves", () => {
    hideCursorForPad();
    move(300, 300);
    move(330, 300);
    expect(hidden()).toBe(false);
  });

  it("comes back on a click or the wheel", () => {
    hideCursorForPad();
    window.dispatchEvent(new MouseEvent("mousedown"));
    expect(hidden()).toBe(false);
    hideCursorForPad();
    window.dispatchEvent(new WheelEvent("wheel"));
    expect(hidden()).toBe(false);
  });

  it("is shown again when the watcher stops", () => {
    hideCursorForPad();
    stop();
    expect(hidden()).toBe(false);
  });
});

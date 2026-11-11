// The toasts container must be positioned: without a `.toasts` rule the
// toasts sat in the page flow, below the app, and could not be seen.
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

describe("toasts", () => {
  it("are pinned to the screen corner, above Big Picture", () => {
    const css = readFileSync(join(__dirname, "App.css"), "utf8");
    const rule = css.match(/(^|\n)\.toasts\b[^{]*\{([^}]*)\}/)?.[2] ?? "";
    expect(rule).toMatch(/position:\s*fixed/);
    const z = Number(rule.match(/z-index:\s*(\d+)/)?.[1]);
    expect(z).toBeGreaterThan(99999); // .big-picture is 99999
  });
});

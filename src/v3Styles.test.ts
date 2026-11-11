// The v3 look of pages restyled in CSS only (jsdom does not apply styles):
// checks the rule that wins (the last one for the selector) in App.css.
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const css = readFileSync(join(__dirname, "App.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

/** Every declaration block whose selector list contains `selector`, in file order. */
function blocks(selector: string): string[] {
  const out: string[] = [];
  for (const m of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const selectors = m[1].split(",").map((s) => s.trim());
    if (selectors.includes(selector)) out.push(m[2]);
  }
  return out;
}

/** The value a property finally gets for `selector` (last declaration wins). */
function finalValue(selector: string, property: string): string | undefined {
  let value: string | undefined;
  for (const block of blocks(selector)) {
    for (const decl of block.split(";")) {
      const [p, ...v] = decl.split(":");
      if (p?.trim() === property) value = v.join(":").trim();
    }
  }
  return value;
}

describe("v3 styles of the Achievements and Settings sections", () => {
  it("turns settings groups into plain cards with Outfit titles", () => {
    expect(finalValue(".settings__group", "background")).toBe("var(--panel)");
    expect(finalValue(".settings__group", "border-radius")).toBe("var(--radius-lg)");
    expect(finalValue(".settings__group-title", "font-family")).toBe("var(--font-title)");
    expect(finalValue(".settings__group-title", "text-transform")).toBe("none");
  });

  it("draws stats bars with the gradient on a soft pill track", () => {
    expect(finalValue(".stats-bar-row__fill", "background")).toBe("var(--grad)");
    expect(finalValue(".stats-bar-row__bar", "background")).toBe("var(--soft)");
    expect(finalValue(".stats-card__value", "font-family")).toBe("var(--font-title)");
  });

  it("frames the monthly wrap's hero card with the gradient (one frame per screen)", () => {
    expect(finalValue(".wrap-card--hero", "background")).toContain("var(--grad) border-box");
    expect(finalValue(".wrap-card--hero", "border")).toBe("4px solid transparent");
  });

  it("gives the theme picker pills and section colors to the pages", () => {
    expect(finalValue(".theme-picker__item", "border-radius")).toBe("var(--radius-pill)");
    expect(css).toMatch(/\.main-content\[data-page="stats"\][^{]*\{\s*--k: var\(--sec-trophy\)/);
    expect(css).toMatch(/\.main-content\[data-page="settings"\][^{]*\{\s*--k: var\(--sec-settings\)/);
  });
});

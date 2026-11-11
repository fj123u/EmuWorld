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

describe("v3 Big Picture (TV sizes)", () => {
  it("frames the selected game with the gradient and keeps its console tint", () => {
    const bg = finalValue(".bp-card--focused .bp-card__cover", "background") ?? "";
    expect(bg).toContain("var(--grad) border-box");
    expect(bg).toContain("var(--k)");
  });

  it("uses TV sizes: 20 px text, 64 px buttons, a 4 px focus ring", () => {
    expect(finalValue(".big-picture", "font-size")).toBe("20px");
    expect(finalValue(".big-picture__exit", "min-height")).toBe("64px");
    expect(finalValue(".big-picture button:focus-visible", "outline")).toBe("4px solid var(--accent)");
    expect(finalValue(".bp-hint", "font-size")).toBe("20px");
  });
});

describe("v3 fixes after the first test", () => {
  it("keeps games and consoles in their own colors under the mouse", () => {
    expect(finalValue(".game-card:hover .game-card__overlay", "display")).toBe("none");
    expect(finalValue(".game-card:hover .game-card__cover", "box-shadow")).toBe(finalValue(".game-card__cover", "box-shadow"));
    expect(finalValue(".home-game:hover .v3-cover", "box-shadow")).toBe(finalValue(".home-cover", "box-shadow"));
    expect(finalValue(".library-tile:hover", "box-shadow")).toBe(finalValue(".v3-ring", "box-shadow"));
    expect(finalValue(".rgs-console-card:hover", "box-shadow")).toBe(finalValue(".rgs-console-card", "box-shadow"));
    expect(finalValue(".big-picture__console-card:hover", "background")).toBe(finalValue(".big-picture__console-card", "background"));
    expect(finalValue(".big-picture__console-card:hover", "box-shadow")).toBe(finalValue(".big-picture__console-card", "box-shadow"));
  });

  it("drops the old pad outline around a focused game (offset in Big Picture)", () => {
    expect(finalValue(".bp-card.gamepad-focused", "outline")).toBe("none !important");
    expect(finalValue(".game-card.gamepad-focused", "outline")).toBe("none !important");
  });

  it("puts the Store search bar and its buttons on one row", () => {
    expect(finalValue(".rgs-search-header", "display")).toBe("flex");
    expect(finalValue(".rgs-search-header", "gap")).toBe("12px");
  });

  it("writes the library menus in the app's font", () => {
    expect(finalValue(".library-sort-select", "font-family")).toBe("inherit");
  });
});

describe("covers without a colored strip", () => {
  it("shows the cover alone: no console ring, and the card no longer clips it", () => {
    expect(finalValue(".game-card", "overflow")).toBe("visible");
    for (const cover of [".game-card__cover", ".bp-card__cover", ".home-cover"]) {
      expect(finalValue(cover, "box-shadow"), cover).toBe("var(--shadow-soft)");
      expect(finalValue(cover, "border"), cover).toBe("none");
    }
    // The console color stays on a game that has no cover yet.
    expect(finalValue(".game-card__cover:not(:has(img))", "background-color")).toContain("var(--k)");
  });
});

describe("search bars", () => {
  it("draw no rectangle inside the pill when focused", () => {
    expect(finalValue(".search-bar__input:focus-visible", "outline")).toBe("none !important");
    expect(finalValue(".search-bar__input.gamepad-focused", "outline")).toBe("none !important");
    expect(finalValue(".search-bar:has(.search-bar__input.gamepad-focused)", "box-shadow")).toBe("0 0 0 3px var(--accent)");
  });
});

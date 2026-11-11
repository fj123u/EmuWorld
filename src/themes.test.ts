import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { DARK_VARIANTS, DEFAULT_THEME, isDarkTheme, themeNameKey, themeSwatch } from "./themes";
import fr from "./i18n/fr";

const css = readFileSync(join(__dirname, "App.css"), "utf8");

describe("themes", () => {
  it("light by default, every other theme is a dark one", () => {
    expect(isDarkTheme("default")).toBe(false);
    for (const id of ["dark", "midnight", "oled", "retro", "sakura", "sunset"]) expect(isDarkTheme(id)).toBe(true);
  });

  it("starts dark when nothing is saved", () => {
    expect(DEFAULT_THEME).toBe("dark");
    expect(isDarkTheme(DEFAULT_THEME)).toBe(true);
  });

  it("keeps the themes saved before v3 (marketplace base_theme) working", () => {
    for (const v of DARK_VARIANTS) {
      expect(css, `[data-theme="${v.id}"] in App.css`).toContain(`[data-theme="${v.id}"]`);
      expect(themeNameKey(v.id)).toBe(`themes.${v.id}`);
    }
    // Every data-theme gets the dark tokens first.
    expect(css).toMatch(/\n\[data-theme\] \{\s+--bg: #0D0E14;/); // \s: CRLF or LF
  });

  it("names every theme, even an unknown one", () => {
    const names = fr.themes as Record<string, string>;
    for (const id of ["default", "dark", ...DARK_VARIANTS.map((v) => v.id), "from-the-future"]) {
      const key = themeNameKey(id).split(".")[1];
      expect(names[key], id).toBeTruthy();
    }
    expect(themeSwatch("sakura")).toBe("#f472b6");
    expect(themeSwatch("from-the-future")).toBe("#8E9BF5");
  });
});

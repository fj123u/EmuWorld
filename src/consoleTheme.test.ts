import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { applyConsoleAccent, consoleHue } from "./consoleTheme";

describe("console colours", () => {
  it("gives every catalog console a colour", () => {
    const catalog = readFileSync(join(__dirname, "..", "src-tauri", "src", "emulators.rs"), "utf8");
    const consoles = [...new Set([...catalog.matchAll(/console: "([^"]+)"/g)].map((m) => m[1]))];
    expect(consoles.length).toBeGreaterThan(20);
    // A new console in emulators.rs needs its hue in src/consoleTheme.ts.
    const missing = consoles.filter((c) => c !== "Multi-System" && consoleHue(c) === null);
    expect(missing).toEqual([]);
  });

  it("has no colour outside a console", () => {
    expect(consoleHue(null)).toBeNull();
    expect(consoleHue("Multi-System")).toBeNull();
    expect(consoleHue("Unknown console")).toBeNull();
  });

  it("sets and clears the accent on the root element", () => {
    const root = document.createElement("html");
    applyConsoleAccent(root, 355);
    expect(root.hasAttribute("data-console-accent")).toBe(true);
    expect(root.style.getPropertyValue("--console-hue")).toBe("355");
    applyConsoleAccent(root, null);
    expect(root.hasAttribute("data-console-accent")).toBe(false);
    expect(root.style.getPropertyValue("--console-hue")).toBe("");
  });
});

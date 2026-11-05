import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { accountUrl, authCallbackUrl, profileUrl, profileUrlLabel, WEB_PANEL_URL } from "./webPanel";

describe("web panel links", () => {
  it("point to GitHub Pages", () => {
    expect(WEB_PANEL_URL).toBe("https://fj123u.github.io/EmuWorld/");
    expect(profileUrl("Léo & co")).toBe("https://fj123u.github.io/EmuWorld/#/u/L%C3%A9o%20%26%20co");
    expect(profileUrlLabel("fj123u")).toBe("fj123u.github.io/EmuWorld/#/u/fj123u");
    expect(accountUrl()).toBe("https://fj123u.github.io/EmuWorld/#/account");
    expect(authCallbackUrl()).toBe("https://fj123u.github.io/EmuWorld/auth-callback.html");
  });

  it("no longer lead to the deleted alwaysdata site", () => {
    // Old changelog entries may mention it; links may not.
    const files = readdirSync(__dirname, { recursive: true })
      .map(String)
      .filter((f) => /\.(ts|tsx)$/.test(f) && !/changelog\.ts$|webPanel(\.test)?\.ts$/.test(f));
    const offenders = files.filter((f) => readFileSync(join(__dirname, f), "utf8").includes("alwaysdata.net"));
    expect(offenders).toEqual([]);
    expect(readFileSync(join(__dirname, "..", "web-panel", "index.html"), "utf8")).not.toContain("alwaysdata.net");
  });
});

import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { CHANGELOG } from "./changelog";

const appVersion: string = JSON.parse(
  readFileSync(join(__dirname, "..", "src-tauri", "tauri.conf.json"), "utf8"),
).version;

const parse = (v: string) => v.split(".").map(Number);
const compare = (a: string, b: string) => {
  const [x, y] = [parse(a), parse(b)];
  for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] - y[i];
  return 0;
};

describe("changelog", () => {
  it("has an entry for the app version (bumping the version needs a changelog)", () => {
    expect(CHANGELOG.map((e) => e.version)).toContain(appVersion);
    // The newest entry may describe the next, unreleased version — never an older one.
    expect(compare(CHANGELOG[0].version, appVersion)).toBeGreaterThanOrEqual(0);
  });

  it("lists versions newest first, once each, with a date and changes", () => {
    for (const entry of CHANGELOG) {
      expect(entry.version).toMatch(/^\d+\.\d+\.\d+$/);
      expect(entry.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      expect(entry.changes.length).toBeGreaterThan(0);
    }
    for (let i = 1; i < CHANGELOG.length; i++) {
      expect(compare(CHANGELOG[i - 1].version, CHANGELOG[i].version), `${CHANGELOG[i - 1].version} before ${CHANGELOG[i].version}`).toBeGreaterThan(0);
    }
  });
});

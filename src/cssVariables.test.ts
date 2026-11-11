// Every var(--x) used by the UI must be defined somewhere: before v3, five
// variables (--accent, --border-subtle, --bg-tertiary, --card-bg,
// --neon-primary) were used in 50 places but never defined, so those styles
// silently fell back to nothing.
import { describe, expect, it } from "vitest";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const SRC = __dirname;
const files = readdirSync(SRC).filter((f) => /\.(css|tsx?)$/.test(f) && !/\.test\.tsx?$/.test(f));
const sources = Object.fromEntries(files.map((f) => [f, readFileSync(join(SRC, f), "utf8")]));

/** Variables defined in CSS (`--x:`) or set from code (style `"--x":`, setProperty("--x")). */
function definedVariables(): Set<string> {
  const defined = new Set<string>();
  for (const text of Object.values(sources)) {
    for (const m of text.matchAll(/(?:^|[\s;{"'])(--[\w-]+)\s*["']?\s*:/g)) defined.add(m[1]);
    for (const m of text.matchAll(/setProperty\(\s*["'](--[\w-]+)["']/g)) defined.add(m[1]);
  }
  return defined;
}

describe("CSS variables", () => {
  it("are all defined", () => {
    const defined = definedVariables();
    const missing: string[] = [];
    for (const [file, text] of Object.entries(sources)) {
      for (const m of text.matchAll(/var\(\s*(--[\w-]+)\s*(,)?/g)) {
        // var(--x, fallback) is fine: it has a value either way.
        if (!m[2] && !defined.has(m[1])) missing.push(`${file}: ${m[1]}`);
      }
    }
    expect([...new Set(missing)]).toEqual([]);
  });

  it("give the five variables that were missing a value", () => {
    const defined = definedVariables();
    for (const v of ["--accent", "--border-subtle", "--bg-tertiary", "--card-bg", "--neon-primary"]) expect(defined.has(v), v).toBe(true);
  });
});

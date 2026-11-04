import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import fr from "./fr";
import en from "./en";

type Tree = { [k: string]: string | Tree };

function flatten(tree: Tree, prefix = ""): Map<string, string> {
  const out = new Map<string, string>();
  for (const [k, v] of Object.entries(tree)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (typeof v === "string") out.set(key, v);
    else for (const [kk, vv] of flatten(v, key)) out.set(kk, vv);
  }
  return out;
}

const frKeys = flatten(fr as unknown as Tree);
const enKeys = flatten(en as unknown as Tree);
const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("translations", () => {
  it("FR and EN define exactly the same keys", () => {
    const missingInEn = [...frKeys.keys()].filter((k) => !enKeys.has(k));
    const missingInFr = [...enKeys.keys()].filter((k) => !frKeys.has(k));
    expect({ missingInEn, missingInFr }).toEqual({ missingInEn: [], missingInFr: [] });
  });

  it("no translation is empty", () => {
    const empty = [...frKeys, ...enKeys].filter(([, v]) => v.trim() === "").map(([k]) => k);
    expect(empty).toEqual([]);
  });

  it("FR and EN use the same {placeholders}", () => {
    const mismatched = [...frKeys.keys()]
      .filter((k) => enKeys.has(k))
      .filter((k) => placeholders(frKeys.get(k)!).join() !== placeholders(enKeys.get(k)!).join());
    expect(mismatched).toEqual([]);
  });

  it("every t()/tl() key used in the code exists", () => {
    const srcDir = join(__dirname, "..");
    const files = readdirSync(srcDir, { recursive: true, encoding: "utf8" })
      .filter((f) => /\.tsx?$/.test(f) && !/\.test\.tsx?$/.test(f) && !f.includes("i18n"));
    const missing: string[] = [];
    for (const file of files) {
      const code = readFileSync(join(srcDir, file), "utf8");
      // Only literal keys: t("section.key") / tl('section.key'). Dynamic keys
      // (template strings, variables) can't be checked statically.
      for (const m of code.matchAll(/\bt[l]?\(\s*["']([a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)+)["']/g)) {
        if (!frKeys.has(m[1])) missing.push(`${file}: ${m[1]}`);
      }
    }
    expect([...new Set(missing)]).toEqual([]);
  });
});

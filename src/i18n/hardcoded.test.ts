// Every visible text must go through t() (CLAUDE.md, i18n rule): this test
// reads the UI files and fails on text written straight in the JSX — between
// tags, on its own line, or in placeholder / title / aria-label / alt.
// Names that are the same in every language are allowed below.
import { describe, expect, it } from "vitest";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const SRC = join(__dirname, "..");

/** Brand names, keys and symbols: identical in French and English. */
const ALLOWED = new Set([
  "EmuWorld", "🎮 EmuWorld", "RetroArch", "RetroAchievements", "Wikipedia", "Vimm's Lair",
  "Ctrl+F12", "ESC", "HC", "iiDB", "Discord", "Google",
]);

const LETTERS = /[A-Za-zÀ-ÿ]{2,}/;

/** Hardcoded texts of one file, as "line: text". */
export function hardcodedTexts(source: string): string[] {
  const found: string[] = [];
  const lines = source.split("\n");
  lines.forEach((line, i) => {
    const trimmed = line.trim();
    if (trimmed.startsWith("//") || trimmed.startsWith("*") || trimmed.startsWith("/*")) return;
    const report = (text: string) => {
      const value = text.trim();
      if (value && LETTERS.test(value) && !ALLOWED.has(value)) found.push(`${i + 1}: ${value}`);
    };
    // Text between two tags: <b>Text</b>, <Icon /> Text</button>. Not `=>` (arrow types).
    for (const m of line.matchAll(/(?<![=-])>([^<>{}]*)</g)) {
      if (!/[;()=&|]/.test(m[1])) report(m[1]); // `a > 0 && b < 3` is code, not text
    }
    // Text after a tag at the end of the line: <span>Text
    const tail = line.match(/(?<![=-])>([^<>{}]+)$/);
    if (tail && !/[;()=,&|]/.test(tail[1]) && !/^\s*(void|string|number|boolean|null)\b/.test(tail[1])) report(tail[1]);
    // {cond && "Text"} in JSX.
    for (const m of line.matchAll(/&& "([A-ZÀ-Ý][^"]*)"\}/g)) report(m[1]);
    // Attributes shown to the user.
    for (const m of line.matchAll(/\b(?:placeholder|title|aria-label|alt)="([^"]*)"/g)) report(m[1]);
    // A line of plain text inside JSX (multi-line text), after a line ending with ">".
    const prev = lines.slice(0, i).reverse().find((l) => l.trim() !== "")?.trim() ?? "";
    if (prev.endsWith(">") && !prev.endsWith("=>") && /^[A-Za-zÀ-ÿ'«][^=;(){}<>`"]*$/.test(trimmed) && / /.test(trimmed)) report(trimmed);
  });
  return found;
}

const uiFiles = readdirSync(SRC).filter((f) => f.endsWith(".tsx") && !f.endsWith(".test.tsx"));

describe("no hardcoded UI text", () => {
  it("finds the kinds of hardcoded text it guards against", () => {
    const sample = [
      "<button>Supprimer</button>",
      "<Play size={14} /> Jouer",
      '<input placeholder="Search..." />',
      "<p>",
      "  Où sont tes ROMs ? EmuWorld va les scanner.",
      "</p>",
      '<span>{t("ok.key")}</span>',
      "<b>EmuWorld</b>",
      "onLaunch: (rom: RomFile) => void,",
      '{page === "discover" && "Discover"}',
    ].join("\n");
    expect(hardcodedTexts(sample)).toEqual([
      "1: Supprimer",
      "2: Jouer",
      "3: Search...",
      "5: Où sont tes ROMs ? EmuWorld va les scanner.",
      "10: Discover",
    ]);
  });

  it.each(uiFiles)("%s only shows translated text", (file) => {
    const source = readFileSync(join(SRC, file), "utf8");
    expect(hardcodedTexts(source), `${file}: use t("section.key") and add the key to fr.ts and en.ts`).toEqual([]);
  });
});

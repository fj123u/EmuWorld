// The v3 app icon (chosen on 2026-11-11) has one source, scratch/icon-v3.svg:
// the app (src/assets), the web panel and the generated Tauri icons
// (`npx tauri icon scratch/icon-v3.svg`) must all come from it.
import { describe, expect, it } from "vitest";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(__dirname, "..");
const read = (p: string) => readFileSync(join(root, p), "utf8").replace(/\r\n/g, "\n");

describe("app icon", () => {
  it("is the same SVG in the app and in the web panel", () => {
    const source = read("scratch/icon-v3.svg");
    expect(read("src/assets/app-icon.svg")).toBe(source);
    expect(read("web-panel/icon.svg")).toBe(source);
    expect(read("web-panel/index.html")).toContain('href="icon.svg"');
    expect(read("index.html")).toContain('href="/src/assets/app-icon.svg"');
  });

  it("has every icon file the installer needs", () => {
    const conf = JSON.parse(read("src-tauri/tauri.conf.json"));
    for (const icon of conf.bundle.icon as string[]) expect(existsSync(join(root, "src-tauri", icon)), icon).toBe(true);
    // A 512 px PNG generated from the SVG (PNG header: width at byte 16).
    const png = readFileSync(join(root, "src-tauri/icons/icon.png"));
    expect(png.readUInt32BE(16)).toBe(512);
  });
});

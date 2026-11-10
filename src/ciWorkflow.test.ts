// The 2.7.1 release failed only at `tauri build` (mismatched Tauri packages),
// a step the push CI did not run. CI must build the real app on every push so
// such errors show up before a version is tagged.
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ci = readFileSync(join(__dirname, "..", ".github", "workflows", "ci.yml"), "utf8");

describe("CI workflow", () => {
  it("builds the Tauri app on every push", () => {
    expect(ci).toMatch(/^\s+push:/m);
    expect(ci).toMatch(/run: npx tauri build --debug --no-bundle/);
  });

  it("still type-checks and runs both test suites", () => {
    expect(ci).toMatch(/run: npx tsc --noEmit/);
    expect(ci).toMatch(/run: npm test/);
    expect(ci).toMatch(/run: cargo test --manifest-path src-tauri\/Cargo\.toml/);
  });
});

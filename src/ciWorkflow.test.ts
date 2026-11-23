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

  it("builds and tests the backend on Linux too (port for 3.2.0)", () => {
    expect(ci).toMatch(/runs-on: ubuntu-22\.04/);
    expect(ci).toMatch(/libwebkit2gtk-4\.1-dev/);
    const linux = ci.slice(ci.indexOf("runs-on: ubuntu-22.04"));
    expect(linux).toMatch(/run: cargo test --manifest-path src-tauri\/Cargo\.toml/);
  });

  it("still type-checks and runs both test suites", () => {
    expect(ci).toMatch(/run: npx tsc --noEmit/);
    expect(ci).toMatch(/run: npm test/);
    expect(ci).toMatch(/run: cargo test --manifest-path src-tauri\/Cargo\.toml/);
  });
});

// 3.0.0 shipped with a Supabase key the project refused: sign-up answered
// "Invalid API key". The release must ask Supabase before building.
const release = readFileSync(join(__dirname, "..", ".github", "workflows", "release.yml"), "utf8");

describe("Release workflow", () => {
  it("checks the Supabase key with Supabase before building the app", () => {
    const check = release.indexOf("name: Check the Supabase key");
    const build = release.indexOf("name: Build + bundle + sign");
    expect(check).toBeGreaterThan(0);
    expect(check).toBeLessThan(build);
    const step = release.slice(check, build);
    expect(step).toContain("/auth/v1/settings");
    expect(step).toContain('apikey: $VITE_SUPABASE_ANON_KEY');
    expect(step).toMatch(/exit 1/);
  });
});

// The release build refuses Tauri npm packages and Rust crates on different
// major.minor versions ("Found version mismatched Tauri packages"): a
// `cargo update` once moved the crates alone and the 2.7.1 release failed.
// This test catches it on every push, before tagging.
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(__dirname, "..");

/** Rust crate versions from Cargo.lock. */
function crateVersions(): Map<string, string> {
  const lock = readFileSync(join(root, "src-tauri", "Cargo.lock"), "utf8");
  const out = new Map<string, string>();
  for (const block of lock.split("[[package]]")) {
    const name = block.match(/^name = "([^"]+)"/m)?.[1];
    const version = block.match(/^version = "([^"]+)"/m)?.[1];
    if (name && version) out.set(name, version);
  }
  return out;
}

/** Installed npm versions from package-lock.json. */
function npmVersion(pkg: string): string | undefined {
  const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
  return lock.packages?.[`node_modules/${pkg}`]?.version;
}

const majorMinor = (v: string) => v.split(".").slice(0, 2).join(".");

describe("Tauri versions", () => {
  it("npm packages and Rust crates share major.minor", () => {
    const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
    const npmNames = Object.keys({ ...pkg.dependencies, ...pkg.devDependencies }).filter((n) => n.startsWith("@tauri-apps/") && n !== "@tauri-apps/cli");
    expect(npmNames.length).toBeGreaterThan(0);
    const crates = crateVersions();
    const mismatches: string[] = [];
    for (const name of npmNames) {
      const crate = name === "@tauri-apps/api" ? "tauri" : name.replace("@tauri-apps/", "tauri-");
      const rust = crates.get(crate);
      const js = npmVersion(name);
      if (!rust || !js) continue; // JS-only helper or crate not used
      if (majorMinor(rust) !== majorMinor(js)) mismatches.push(`${crate} ${rust} ≠ ${name} ${js}`);
    }
    expect(mismatches, "run npm install for the matching @tauri-apps versions").toEqual([]);
  });
});

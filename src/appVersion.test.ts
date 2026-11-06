import { describe, expect, it } from "vitest";
import { compareVersions, versionInRange } from "./appVersion";

describe("app version", () => {
  it("compares versions as numbers, not text", () => {
    expect(compareVersions("2.10.0", "2.9.0")).toBeGreaterThan(0);
    expect(compareVersions("2.6.1", "2.6.1")).toBe(0);
    expect(compareVersions("2.6", "2.6.0")).toBe(0);
    expect(compareVersions("2.5.3", "2.6.0")).toBeLessThan(0);
  });

  it("targets announcements by version range", () => {
    expect(versionInRange("2.6.1", "2.6.0", null)).toBe(true);
    expect(versionInRange("2.5.3", "2.6.0", null)).toBe(false);
    expect(versionInRange("2.10.0", null, "2.9.9")).toBe(false);
    expect(versionInRange("2.6.1")).toBe(true);
  });
});

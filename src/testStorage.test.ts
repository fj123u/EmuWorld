// Node 25+ defines empty localStorage / sessionStorage globals that hid
// jsdom's: on a recent Node every test touching storage failed. The setup
// (src/test/setup.ts) puts jsdom's back.
import { describe, expect, it } from "vitest";

describe("test environment", () => {
  it("has a working localStorage and sessionStorage, whatever the Node version", () => {
    localStorage.setItem("emuworld_test", "1");
    sessionStorage.setItem("emuworld_test", "2");
    expect(localStorage.getItem("emuworld_test")).toBe("1");
    expect(sessionStorage.getItem("emuworld_test")).toBe("2");
    localStorage.clear();
    expect(localStorage.getItem("emuworld_test")).toBeNull();
  });
});

import { describe, expect, it, vi } from "vitest";
import { describeError, installErrorLogging } from "./errorLogging";

describe("interface errors in the log file", () => {
  it("sends uncaught errors and rejected promises, with where they happened", () => {
    const send = vi.fn();
    const stop = installErrorLogging(send);
    const error = new TypeError("roms is undefined");
    window.dispatchEvent(new ErrorEvent("error", { error, message: error.message, filename: "http://localhost:1420/src/App.tsx", lineno: 120, colno: 7 }));
    const rejection = new Event("unhandledrejection") as PromiseRejectionEvent;
    Object.defineProperty(rejection, "reason", { value: "network down" });
    window.dispatchEvent(rejection);
    expect(send).toHaveBeenCalledTimes(2);
    expect(send.mock.calls[0][0]).toBe("ERROR");
    expect(send.mock.calls[0][1]).toMatch(/^Erreur JS: TypeError: roms is undefined @ App\.tsx:120:7/);
    expect(send.mock.calls[1][1]).toBe("Promesse rejetée: network down");
    stop();
    window.dispatchEvent(new ErrorEvent("error", { message: "later" }));
    expect(send).toHaveBeenCalledTimes(2);
  });

  it("describes any thrown value", () => {
    expect(describeError({ code: 42 })).toBe('{"code":42}');
    expect(describeError(new Error("boom"))).toMatch(/^Error: boom( \| .*)?$/);
  });
});

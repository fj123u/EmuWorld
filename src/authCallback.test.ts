import { describe, expect, it } from "vitest";
import { oauthRedirectUrl, parseAuthCallback } from "./authCallback";

describe("OAuth return", () => {
  it("sends the browser back to the app's loopback server, not to a web page", () => {
    expect(oauthRedirectUrl(17643)).toBe("http://127.0.0.1:17643/callback");
  });

  it("reads the PKCE code from the loopback server or a deep link", () => {
    expect(parseAuthCallback("emuworld://auth-callback?code=3f2a-91bc")).toEqual({ kind: "code", code: "3f2a-91bc" });
    expect(parseAuthCallback("emuworld://auth-callback#code=abc")).toEqual({ kind: "code", code: "abc" });
  });

  it("explains a refused sign-in instead of waiting forever", () => {
    expect(parseAuthCallback("emuworld://auth-callback?error=access_denied&error_description=User+cancelled")).toEqual({
      kind: "error",
      message: "User cancelled",
    });
    expect(parseAuthCallback("emuworld://auth-callback?error=server_error")).toEqual({ kind: "error", message: "server_error" });
  });

  it("ignores raw tokens, which any web page could forge to switch accounts", () => {
    expect(parseAuthCallback("emuworld://auth-callback#access_token=evil&refresh_token=evil")).toBeNull();
    expect(parseAuthCallback("emuworld://auth-callback")).toBeNull();
    expect(parseAuthCallback("not a url ::")).toBeNull();
  });
});

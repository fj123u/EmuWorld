// OAuth (Google / Discord) return to the app. The browser comes back to the
// app's loopback server (http://127.0.0.1:<port>/callback) or, as a fallback,
// through an `emuworld://auth-callback` deep link; both reach the UI as an
// `oauth-callback` event carrying an `emuworld://auth-callback?...` URL.

/** Redirect URL given to Supabase; it must be allowed in Authentication → URL Configuration. */
export function oauthRedirectUrl(port: number): string {
  return `http://127.0.0.1:${port}/callback`;
}

export type AuthCallback =
  | { kind: "code"; code: string }
  | { kind: "error"; message: string };

/**
 * Reads the callback URL. Only the PKCE `code` is accepted: it can only be
 * exchanged with the verifier this app stored when the sign-in started, so a
 * web page that fires an `emuworld://` link can't sign the app into another
 * account (raw access tokens in the URL could).
 */
export function parseAuthCallback(url: string): AuthCallback | null {
  let parsed: URL;
  try {
    parsed = new URL(url.replace(/^emuworld:\/\//, "http://localhost/"));
  } catch {
    return null;
  }
  const params = new URLSearchParams(parsed.search || parsed.hash.replace(/^#/, ""));
  const code = params.get("code");
  if (code) return { kind: "code", code };
  const error = params.get("error_description") || params.get("error");
  if (error) return { kind: "error", message: error };
  return null;
}

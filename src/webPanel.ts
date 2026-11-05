// The public web panel (profiles, leaderboard, account), hosted on GitHub
// Pages from `web-panel/` by .github/workflows/pages.yml. It moved off
// alwaysdata (account deleted in November 2026); every link goes through here.

export const WEB_PANEL_URL = "https://fj123u.github.io/EmuWorld/";

/** Public profile page of a player (hash routing: works on static hosting). */
export function profileUrl(username: string): string {
  return `${WEB_PANEL_URL}#/u/${encodeURIComponent(username)}`;
}

/** Short form shown next to the share buttons. */
export function profileUrlLabel(username: string): string {
  return profileUrl(username).replace(/^https:\/\//, "");
}

/** Account page of the signed-in player. */
export function accountUrl(): string {
  return `${WEB_PANEL_URL}#/account`;
}

/** Where Supabase sends email confirmations (signup, password reset). */
export function authCallbackUrl(): string {
  return `${WEB_PANEL_URL}auth-callback.html`;
}

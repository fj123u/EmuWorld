// Smart playlists: library filters computed from the local playtime data, so
// they stay up to date without the user maintaining anything.

export const SMART_PLAYLISTS = ["recent", "stale", "started", "short", "long"] as const;
export type SmartPlaylist = (typeof SMART_PLAYLISTS)[number];

/** The playtime fields the playlists look at (subset of `GameEntry`). */
export interface SmartPlaylistEntry {
  seconds: number;
  launches: number;
  last_played: string | null;
}

const DAY_MS = 86_400_000;
const HOUR = 3_600;

export function isSmartPlaylist(value: string): value is SmartPlaylist {
  return (SMART_PLAYLISTS as readonly string[]).includes(value);
}

/** Days since the last session, or `null` when unknown (never played, corrupt date). */
function daysSinceLastPlayed(entry: SmartPlaylistEntry, now: number): number | null {
  if (!entry.last_played) return null;
  const at = Date.parse(entry.last_played);
  if (Number.isNaN(at)) return null;
  return Math.max(0, (now - at) / DAY_MS);
}

/**
 * Whether a game belongs to a smart playlist. Games never launched only match
 * nothing here: "Unplayed" is already its own filter.
 *
 * - recent  — played in the last 7 days
 * - stale   — not played for more than 30 days ("À reprendre")
 * - started — barely started: 1–2 launches and under 1 h in total
 * - short   — average session under 30 min (at least 2 launches)
 * - long    — 10 h or more in total
 */
export function matchesSmartPlaylist(
  playlist: SmartPlaylist,
  entry: SmartPlaylistEntry | undefined,
  now: number = Date.now(),
): boolean {
  if (!entry || entry.launches <= 0) return false;
  const days = daysSinceLastPlayed(entry, now);
  switch (playlist) {
    case "recent":
      return days !== null && days <= 7;
    case "stale":
      return days !== null && days > 30;
    case "started":
      return entry.launches <= 2 && entry.seconds < HOUR;
    case "short":
      return entry.launches >= 2 && entry.seconds / entry.launches < 30 * 60;
    case "long":
      return entry.seconds >= 10 * HOUR;
  }
}

// Data of the v3 home page (HomePage.tsx), computed from what the app already
// has in memory: the library, playtime.json, achievements, friends' presence.

export interface HomeRom {
  name: string;
  console: string;
  path: string;
}

export interface HomeGameStats {
  seconds: number;
  launches: number;
  last_played: string | null;
  monthly?: Record<string, { seconds: number }>;
  active_days?: string[];
}

const key = (r: { console: string; name: string }) => `${r.console}::${r.name}`;

/** The library's games by last session, most recent first (games no longer on disk are skipped). */
export function recentGames<R extends HomeRom>(roms: R[], games: Record<string, HomeGameStats>, limit: number): { rom: R; seconds: number; lastPlayed: string }[] {
  return roms
    .map((rom) => ({ rom, g: games[key(rom)] }))
    .filter((x) => x.g?.last_played && !Number.isNaN(Date.parse(x.g.last_played)))
    .sort((a, b) => Date.parse(b.g!.last_played!) - Date.parse(a.g!.last_played!))
    .slice(0, limit)
    .map((x) => ({ rom: x.rom, seconds: x.g!.seconds, lastPlayed: x.g!.last_played! }));
}

/** One tile per console of the library, most games first. */
export function consoleTiles(roms: HomeRom[]): { name: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const r of roms) counts.set(r.console, (counts.get(r.console) ?? 0) + 1);
  return [...counts.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
}

/** Local month key `YYYY-MM`, as playtime.json writes it. */
export function monthKey(now: Date): string {
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
}

/** Play time, days played and achievements unlocked this month. */
export function monthSummary(
  games: Record<string, HomeGameStats>,
  achievements: { unlocked: boolean; unlocked_at: string | null }[],
  now: Date,
): { seconds: number; activeDays: number; achievements: number } {
  const month = monthKey(now);
  let seconds = 0;
  const days = new Set<string>();
  for (const g of Object.values(games)) {
    seconds += g.monthly?.[month]?.seconds ?? 0;
    for (const d of g.active_days ?? []) if (d.startsWith(month)) days.add(d);
  }
  const unlocked = achievements.filter((a) => {
    if (!a.unlocked || !a.unlocked_at) return false;
    const d = new Date(a.unlocked_at);
    return !Number.isNaN(d.getTime()) && monthKey(d) === month;
  }).length;
  return { seconds, activeDays: days.size, achievements: unlocked };
}

/** "3 h 06", "48 min", "0 min". */
export function formatPlaytime(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  if (s >= 3600) return `${Math.floor(s / 3600)} h ${String(Math.floor((s % 3600) / 60)).padStart(2, "0")}`;
  return `${Math.floor(s / 60)} min`;
}

export interface FriendPresence {
  user_id: string;
  status: "online" | "playing" | "offline";
  current_game: string | null;
  updated_at: string;
}

/** A presence older than this is a closed app that could not say goodbye. */
export const PRESENCE_TIMEOUT_MS = 120_000;

/** Friends whose app is open, playing ones first. */
export function friendsOnline<F extends { id: string }>(friends: F[], presences: FriendPresence[], now: number): { friend: F; playing: string | null }[] {
  return friends
    .map((friend) => ({ friend, p: presences.find((x) => x.user_id === friend.id) }))
    .filter((x) => x.p && x.p.status !== "offline" && now - Date.parse(x.p.updated_at) < PRESENCE_TIMEOUT_MS)
    .map((x) => ({ friend: x.friend, playing: x.p!.status === "playing" ? x.p!.current_game : null }))
    .sort((a, b) => Number(b.playing !== null) - Number(a.playing !== null));
}

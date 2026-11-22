// Where the challenges keep what must not change once seen: the challenges
// drawn for a period (a new ROM must not swap this week's personal one) and
// the challenges done (points, streak, badges). Per-viewer data in
// localStorage, read and written defensively (private mode, quota).
import type { Challenge, History, Period } from "./challenges";

const DRAWN_KEY = "emuworld_challenges_drawn";
const HISTORY_KEY = "emuworld_challenge_history";
/** Periods kept in the drawn cache: the current day / week / month and a few before. */
const MAX_DRAWN = 12;

function read<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function write(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* storage unavailable: the challenges still work, just not remembered */
  }
}

/** Challenges a full draw has: 1 a day, 3 a week, 1 a month. */
export const FULL_DRAW: Record<Period["rhythm"], number> = { day: 1, week: 3, month: 1 };

/**
 * The challenges of `period`, drawn with `draw` the first time and kept after.
 * Only a full draw is kept: one made before the library was loaded (no
 * personal challenge) is drawn again later.
 */
export function drawnChallenges(period: Period, draw: () => Challenge[]): Challenge[] {
  const drawn = read<Record<string, Challenge[]>>(DRAWN_KEY, {});
  const kept = drawn[period.key];
  if (Array.isArray(kept) && kept.length >= FULL_DRAW[period.rhythm]) return kept;
  const fresh = draw();
  if (fresh.length >= FULL_DRAW[period.rhythm]) {
    const keys = Object.keys(drawn).filter((k) => k !== period.key).slice(-(MAX_DRAWN - 1));
    write(DRAWN_KEY, { ...Object.fromEntries(keys.map((k) => [k, drawn[k]])), [period.key]: fresh });
  }
  return fresh;
}

export function readChallengeHistory(): History {
  const h = read<History>(HISTORY_KEY, {});
  return h && typeof h === "object" && !Array.isArray(h) ? h : {};
}

export function saveChallengeHistory(history: History) {
  write(HISTORY_KEY, history);
}

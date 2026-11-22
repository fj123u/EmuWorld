// Challenges ("défis") v2, computed from the session journal (sessions.rs).
//
// Nothing is joined or counted on the side: a challenge's progress is
// recomputed from the sessions of its period, so what was played before
// opening the page counts, and the same function serves every rhythm.
// Plan: scratch/defis-refonte.md.
//
// - Day (local midnight → midnight): 1 personal challenge.
// - Week (ISO, Monday → Sunday): 2 common + 1 personal.
// - Month: 1 common.
// Common challenges are the same for everyone; personal ones are drawn from
// the player's library and are always doable with it.

export interface Session {
  console: string;
  name: string;
  /** Start, RFC 3339. */
  start: string;
  seconds: number;
  emulator: string;
}

export type Rhythm = "day" | "week" | "month";
export type Kind = "time" | "distinct" | "days" | "sessions" | "discovery" | "comeback";
export type Family = "common" | "personal";

export interface Period {
  rhythm: Rhythm;
  /** `J-2026-11-12`, `S-2026-W46`, `M-2026-11`. */
  key: string;
  start: Date;
  /** Exclusive. */
  end: Date;
}

export interface Challenge {
  /** Template id, e.g. `week_time_3h`. */
  id: string;
  /** `period key:id`, unique for this period. */
  key: string;
  rhythm: Rhythm;
  kind: Kind;
  family: Family;
  /** Seconds (time, discovery, comeback) or a count. */
  goal: number;
  /** Least seconds for a day / a session to count. */
  minSeconds?: number;
  console?: string;
  series?: string;
  /** One game, by name (versus on a game). */
  game?: string;
  points: number;
}

/** What the draw needs to know about the library. */
export interface Library {
  games: { name: string; console: string }[];
  /** playtime.json entries by `console::name`. */
  playtime: Record<string, { seconds?: number; first_played?: string | null; favorite?: boolean }>;
}

export const POINTS: Record<Rhythm, number> = { day: 10, week: 30, month: 100 };
/** Bonus when the 3 challenges of a week are done. */
export const WEEK_BONUS = 20;
/** Badge tiers: completions of one kind. */
export const BADGE_TIERS = [
  { tier: "bronze", count: 1 },
  { tier: "silver", count: 5 },
  { tier: "gold", count: 20 },
] as const;
/** A favorite counts as "left aside" after this long without a session. */
const COMEBACK_DAYS = 30;

const pad = (n: number) => String(n).padStart(2, "0");
const gameKey = (g: { console: string; name: string }) => `${g.console}::${g.name}`;

// ---- periods ------------------------------------------------------------------

/** The ISO week of a local date: its year and number. */
function isoWeek(d: Date): { year: number; week: number } {
  const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
  const day = t.getUTCDay() || 7; // Sunday 7
  t.setUTCDate(t.getUTCDate() + 4 - day); // the Thursday of this week
  const yearStart = new Date(Date.UTC(t.getUTCFullYear(), 0, 1));
  return { year: t.getUTCFullYear(), week: Math.ceil(((t.getTime() - yearStart.getTime()) / 86400000 + 1) / 7) };
}

/** The period of `rhythm` holding `date` (local time). */
export function periodOf(rhythm: Rhythm, date: Date): Period {
  if (rhythm === "day") {
    const start = new Date(date.getFullYear(), date.getMonth(), date.getDate());
    const end = new Date(date.getFullYear(), date.getMonth(), date.getDate() + 1);
    return { rhythm, key: `J-${start.getFullYear()}-${pad(start.getMonth() + 1)}-${pad(start.getDate())}`, start, end };
  }
  if (rhythm === "week") {
    const monday = new Date(date.getFullYear(), date.getMonth(), date.getDate() - ((date.getDay() + 6) % 7));
    const end = new Date(monday.getFullYear(), monday.getMonth(), monday.getDate() + 7);
    const { year, week } = isoWeek(monday);
    return { rhythm, key: `S-${year}-W${pad(week)}`, start: monday, end };
  }
  const start = new Date(date.getFullYear(), date.getMonth(), 1);
  const end = new Date(date.getFullYear(), date.getMonth() + 1, 1);
  return { rhythm, key: `M-${start.getFullYear()}-${pad(start.getMonth() + 1)}`, start, end };
}

export function previousPeriod(p: Period): Period {
  return periodOf(p.rhythm, new Date(p.start.getTime() - 1));
}

// ---- seeded draw ----------------------------------------------------------------

/** FNV-1a of a string, as a 32-bit seed. */
function hash(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/** mulberry32: the same numbers on every machine for one seed. */
function rng(seed: string): () => number {
  let a = hash(seed);
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function shuffle<T>(items: T[], seed: string): T[] {
  const out = [...items];
  const r = rng(seed);
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(r() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

// ---- catalog ------------------------------------------------------------------

type Template = Omit<Challenge, "key" | "rhythm" | "family" | "points">;

const H = 3600;
const M = 60;

const COMMON_WEEK: Template[] = [
  { id: "week_time_3h", kind: "time", goal: 3 * H },
  { id: "week_time_6h", kind: "time", goal: 6 * H },
  { id: "week_distinct_4", kind: "distinct", goal: 4 },
  { id: "week_days_4", kind: "days", goal: 4, minSeconds: 15 * M },
  { id: "week_sessions_6", kind: "sessions", goal: 6, minSeconds: 10 * M },
];

const COMMON_MONTH: Template[] = [
  { id: "month_days_15", kind: "days", goal: 15, minSeconds: 10 * M },
  { id: "month_time_15h", kind: "time", goal: 15 * H },
  { id: "month_distinct_10", kind: "distinct", goal: 10 },
];

/** Series a challenge can be about: whole words of the title. */
export const SERIES: { id: string; label: string; pattern: RegExp }[] = [
  { id: "zelda", label: "Zelda", pattern: /\bzelda\b/i },
  { id: "mario", label: "Mario", pattern: /\bmario\b/i },
  { id: "pokemon", label: "Pokémon", pattern: /\bpok[eé]mon\b/i },
  { id: "sonic", label: "Sonic", pattern: /\bsonic\b/i },
  { id: "metroid", label: "Metroid", pattern: /\bmetroid\b/i },
  { id: "final-fantasy", label: "Final Fantasy", pattern: /\bfinal fantasy\b/i },
  { id: "kirby", label: "Kirby", pattern: /\bkirby\b/i },
  { id: "castlevania", label: "Castlevania", pattern: /\bcastlevania\b/i },
  { id: "resident-evil", label: "Resident Evil", pattern: /\b(resident evil|biohazard)\b/i },
  { id: "mega-man", label: "Mega Man", pattern: /\bmega ?man\b/i },
  { id: "donkey-kong", label: "Donkey Kong", pattern: /\bdonkey kong\b/i },
  { id: "street-fighter", label: "Street Fighter", pattern: /\bstreet fighter\b/i },
  { id: "crash", label: "Crash Bandicoot", pattern: /\bcrash bandicoot\b/i },
  { id: "spyro", label: "Spyro", pattern: /\bspyro\b/i },
  { id: "metal-gear", label: "Metal Gear", pattern: /\bmetal gear\b/i },
  { id: "fire-emblem", label: "Fire Emblem", pattern: /\bfire emblem\b/i },
  { id: "gta", label: "GTA", pattern: /\b(grand theft auto|gta)\b/i },
  { id: "tekken", label: "Tekken", pattern: /\btekken\b/i },
];

const seriesById = (id: string) => SERIES.find((s) => s.id === id);

/** Personal templates the library makes possible, for one rhythm. */
function personalTemplates(rhythm: "day" | "week", lib: Library, seed: string): Template[] {
  const r = rng(`${seed}|pick`);
  const pick = <T,>(items: T[]): T | undefined => (items.length ? items[Math.floor(r() * items.length)] : undefined);
  const consoles = [...new Set(lib.games.map((g) => g.console))].sort();
  const series = SERIES.filter((s) => lib.games.some((g) => s.pattern.test(g.name))).map((s) => s.id);
  const neverPlayed = lib.games.some((g) => !(lib.playtime[gameKey(g)]?.seconds));
  const favorites = lib.games.some((g) => lib.playtime[gameKey(g)]?.favorite);
  const consoleGames = (c: string) => new Set(lib.games.filter((g) => g.console === c).map((g) => g.name)).size;

  const out: Template[] = [];
  const day = rhythm === "day";
  const console_ = pick(consoles);
  if (console_) out.push({ id: `${rhythm}_console`, kind: "time", goal: day ? 30 * M : 2 * H, console: console_ });
  const serie = pick(series);
  if (serie) out.push({ id: `${rhythm}_series`, kind: "time", goal: day ? 30 * M : 2 * H, series: serie });
  if (neverPlayed) out.push({ id: `${rhythm}_discovery`, kind: "discovery", goal: day ? 20 * M : H });
  if (favorites) out.push({ id: `${rhythm}_comeback`, kind: "comeback", goal: day ? 20 * M : H });
  if (day) out.push({ id: "day_time", kind: "time", goal: 45 * M });
  else {
    const rich = consoles.filter((c) => consoleGames(c) >= 3);
    const c = pick(rich);
    if (c) out.push({ id: "week_console_distinct", kind: "distinct", goal: 3, console: c });
  }
  return out;
}

function make(t: Template, period: Period, family: Family): Challenge {
  return { ...t, key: `${period.key}:${t.id}`, rhythm: period.rhythm, family, points: POINTS[period.rhythm] };
}

/** Two common challenges of a week, of different kinds, none of `exclude`. */
function pickCommons(period: Period, exclude: string[]): Template[] {
  const out: Template[] = [];
  for (const t of shuffle(COMMON_WEEK, period.key)) {
    if (out.length === 2) break;
    if (exclude.includes(t.id) || out.some((o) => o.kind === t.kind)) continue;
    out.push(t);
  }
  return out;
}

/** First week of the rotation: each later week excludes the one before. */
const ROTATION_START = new Date(2025, 11, 29); // Monday of S-2026-W01
const commonsCache = new Map<string, Template[]>();

/** The common challenges of a week: never the two of the week before. */
function weekCommons(period: Period): Template[] {
  const cached = commonsCache.get(period.key);
  if (cached) return cached;
  if (period.start < ROTATION_START) return pickCommons(period, []);
  // Walk from the last known week (or the start) up to this one.
  let p = periodOf("week", ROTATION_START);
  let last: Template[] = [];
  for (let back = previousPeriod(period); back.start >= ROTATION_START; back = previousPeriod(back)) {
    const known = commonsCache.get(back.key);
    if (known) {
      p = periodOf("week", back.end);
      last = known;
      break;
    }
  }
  for (; p.start <= period.start; p = periodOf("week", p.end)) {
    last = pickCommons(p, last.map((t) => t.id));
    commonsCache.set(p.key, last);
  }
  return commonsCache.get(period.key) ?? pickCommons(period, []);
}

/**
 * The challenges of a period. `player` seeds the personal ones (each player
 * gets their own); the common ones only depend on the period.
 */
export function drawChallenges(period: Period, lib: Library, player: string): Challenge[] {
  const personalSeed = `${period.key}|${player}`;
  if (period.rhythm === "month") {
    return [make(shuffle(COMMON_MONTH, period.key)[0], period, "common")];
  }
  if (period.rhythm === "day") {
    const t = shuffle(personalTemplates("day", lib, personalSeed), personalSeed)[0];
    return t ? [make(t, period, "personal")] : [];
  }
  // Not the same common challenges two weeks in a row.
  const commons = weekCommons(period);
  const kinds = commons.map((t) => t.kind);
  const personal = shuffle(personalTemplates("week", lib, personalSeed), personalSeed).find((t) => !kinds.includes(t.kind) || t.console || t.series);
  return [...commons.map((t) => make(t, period, "common")), ...(personal ? [make(personal, period, "personal")] : [])];
}

// ---- progress -------------------------------------------------------------------

/** Seconds of `s` inside [from, to). */
function overlap(s: Session, from: Date, to: Date): number {
  const start = Date.parse(s.start);
  if (Number.isNaN(start)) return 0;
  const end = start + s.seconds * 1000;
  return Math.max(0, Math.min(end, to.getTime()) - Math.max(start, from.getTime())) / 1000;
}

function matches(c: Challenge, s: Session): boolean {
  if (c.console && s.console !== c.console) return false;
  if (c.series && !seriesById(c.series)?.pattern.test(s.name)) return false;
  if (c.game && s.name.toLowerCase() !== c.game.toLowerCase()) return false;
  return true;
}

/** Progress of a challenge over its period: seconds or a count, capped at the goal. */
export function progress(c: Challenge, period: Period, sessions: Session[], lib: Library): number {
  const inPeriod = sessions
    .map((s) => ({ s, secs: overlap(s, period.start, period.end) }))
    .filter((x) => x.secs > 0 && matches(c, x.s));
  let value = 0;
  switch (c.kind) {
    case "time":
      value = inPeriod.reduce((acc, x) => acc + x.secs, 0);
      break;
    case "distinct":
      value = new Set(inPeriod.map((x) => gameKey(x.s))).size;
      break;
    case "sessions":
      value = inPeriod.filter((x) => x.secs >= (c.minSeconds ?? 0)).length;
      break;
    case "days": {
      const perDay = new Map<number, number>();
      for (let d = new Date(period.start); d < period.end; d = new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1)) {
        const next = new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1);
        const secs = inPeriod.reduce((acc, x) => acc + overlap(x.s, d, next), 0);
        if (secs > 0) perDay.set(d.getTime(), secs);
      }
      value = [...perDay.values()].filter((secs) => secs >= (c.minSeconds ?? 0)).length;
      break;
    }
    case "discovery": {
      // A game first played during the period (or never recorded before it).
      const isNew = (s: Session) => {
        const first = lib.playtime[gameKey(s)]?.first_played;
        const before = sessions.some((o) => gameKey(o) === gameKey(s) && Date.parse(o.start) < period.start.getTime());
        return !before && (!first || Date.parse(first) >= period.start.getTime() - 1000);
      };
      value = inPeriod.filter((x) => isNew(x.s)).reduce((acc, x) => acc + x.secs, 0);
      break;
    }
    case "comeback": {
      const from = period.start.getTime() - COMEBACK_DAYS * 86400000;
      const leftAside = (s: Session) =>
        !!lib.playtime[gameKey(s)]?.favorite &&
        !sessions.some((o) => gameKey(o) === gameKey(s) && Date.parse(o.start) >= from && Date.parse(o.start) < period.start.getTime());
      value = inPeriod.filter((x) => leftAside(x.s)).reduce((acc, x) => acc + x.secs, 0);
      break;
    }
  }
  return Math.min(c.goal, Math.floor(value));
}

/** Kinds whose goal and progress are durations. */
export const isDuration = (c: Pick<Challenge, "kind">) => c.kind === "time" || c.kind === "discovery" || c.kind === "comeback";

// ---- rewards ----------------------------------------------------------------------

/** A challenge done, kept in the history (localStorage). */
export interface Completion {
  key: string;
  id: string;
  kind: Kind | "bonus";
  rhythm: Rhythm;
  period: string;
  points: number;
  at: string;
}

export type History = Record<string, Completion>;

/** New completions among `challenges`, with the week bonus when the 3 of a week are done. */
export function newCompletions(history: History, items: { challenge: Challenge; period: Period; value: number }[], now: Date): Completion[] {
  const out: Completion[] = [];
  for (const { challenge: c, period, value } of items) {
    if (value >= c.goal && !history[c.key]) {
      out.push({ key: c.key, id: c.id, kind: c.kind, rhythm: c.rhythm, period: period.key, points: c.points, at: now.toISOString() });
    }
  }
  const weeks = items.filter((i) => i.challenge.rhythm === "week");
  if (weeks.length >= 3) {
    const period = weeks[0].period.key;
    const allDone = weeks.every((i) => i.value >= i.challenge.goal);
    const key = `${period}:bonus`;
    if (allDone && !history[key]) out.push({ key, id: "bonus", kind: "bonus", rhythm: "week", period, points: WEEK_BONUS, at: now.toISOString() });
  }
  return out;
}

export interface Rewards {
  points: number;
  /** Weeks in a row with a weekly challenge done, up to this week (or the last one). */
  streak: number;
  badges: { kind: Kind; count: number; tier: (typeof BADGE_TIERS)[number]["tier"] | null }[];
  recent: Completion[];
}

export function rewards(history: History, now: Date): Rewards {
  const all = Object.values(history);
  const points = all.reduce((acc, c) => acc + c.points, 0);
  const weeksDone = new Set(all.filter((c) => c.rhythm === "week" && c.kind !== "bonus").map((c) => c.period));
  let week = periodOf("week", now);
  if (!weeksDone.has(week.key)) week = previousPeriod(week); // this week is not lost yet
  let streak = 0;
  while (weeksDone.has(week.key)) {
    streak++;
    week = previousPeriod(week);
  }
  const kinds: Kind[] = ["time", "distinct", "days", "sessions", "discovery", "comeback"];
  const badges = kinds.map((kind) => {
    const count = all.filter((c) => c.kind === kind).length;
    const tier = [...BADGE_TIERS].reverse().find((t) => count >= t.count)?.tier ?? null;
    return { kind, count, tier };
  });
  const recent = all.filter((c) => c.kind !== "bonus").sort((a, b) => b.at.localeCompare(a.at)).slice(0, 10);
  return { points, streak, badges, recent };
}

// ---- texts -------------------------------------------------------------------------

type T = (key: string, params?: Record<string, string | number>) => string;

/** Title and description of a challenge, in the app's language. */
export function describe(c: Challenge, t: T, formatTime: (s: number) => string): { title: string; description: string } {
  const series = c.series ? seriesById(c.series)?.label ?? c.series : undefined;
  const time = formatTime(c.goal);
  const min = Math.round((c.minSeconds ?? 0) / 60);
  switch (c.kind) {
    case "time":
      if (c.console) return { title: t("challenges2.title.console", { console: c.console }), description: t("challenges2.desc.timeConsole", { time, console: c.console }) };
      if (series) return { title: t("challenges2.title.series", { series }), description: t("challenges2.desc.timeSeries", { time, series }) };
      return { title: t(c.rhythm === "day" ? "challenges2.title.timeDay" : "challenges2.title.time"), description: t("challenges2.desc.time", { time }) };
    case "distinct":
      if (c.console) return { title: t("challenges2.title.console", { console: c.console }), description: t("challenges2.desc.distinctConsole", { n: c.goal, console: c.console }) };
      return { title: t("challenges2.title.distinct"), description: t("challenges2.desc.distinct", { n: c.goal }) };
    case "days":
      return { title: t("challenges2.title.days"), description: t("challenges2.desc.days", { n: c.goal, min }) };
    case "sessions":
      return { title: t("challenges2.title.sessions"), description: t("challenges2.desc.sessions", { n: c.goal, min }) };
    case "discovery":
      return { title: t("challenges2.title.discovery"), description: t("challenges2.desc.discovery", { time }) };
    case "comeback":
      return { title: t("challenges2.title.comeback"), description: t("challenges2.desc.comeback", { time, days: COMEBACK_DAYS }) };
  }
}

/** "1 h 20 / 3 h 00" or "2 / 4". */
export function progressText(c: Challenge, value: number, formatTime: (s: number) => string): string {
  return isDuration(c) ? `${formatTime(value)} / ${formatTime(c.goal)}` : `${value} / ${c.goal}`;
}

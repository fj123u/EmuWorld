import { describe, expect, it } from "vitest";
import {
  describe as describeChallenge, drawChallenges, newCompletions, periodOf, previousPeriod, progress, rewards,
  type Challenge, type History, type Library, type Session,
} from "./challenges";
import { formatPlaytime } from "./home";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

const local = (y: number, m: number, d: number, h = 0, min = 0) => new Date(y, m - 1, d, h, min);
const session = (name: string, console: string, start: Date, minutes: number): Session =>
  ({ name, console, start: start.toISOString(), seconds: minutes * 60, emulator: "x" });

const lib: Library = {
  games: [
    { name: "The Legend of Zelda", console: "NES" },
    { name: "Super Mario Bros. 3", console: "NES" },
    { name: "Metroid", console: "NES" },
    { name: "Super Mario 64", console: "Nintendo 64" },
    { name: "Unplayed Game", console: "Nintendo 64" },
  ],
  playtime: {
    "NES::The Legend of Zelda": { seconds: 7200, first_played: "2025-01-01T10:00:00Z", favorite: true },
    "NES::Super Mario Bros. 3": { seconds: 3600, first_played: "2025-02-01T10:00:00Z" },
    "NES::Metroid": { seconds: 60, first_played: "2025-03-01T10:00:00Z" },
    "Nintendo 64::Super Mario 64": { seconds: 600, first_played: "2025-04-01T10:00:00Z" },
  },
};

describe("periods", () => {
  it("weeks run Monday to Sunday (ISO), whatever the day of January 1st", () => {
    const sunday = periodOf("week", local(2026, 11, 15, 23, 59));
    const monday = periodOf("week", local(2026, 11, 16, 0, 0));
    expect(sunday.key).toBe("S-2026-W46");
    expect(monday.key).toBe("S-2026-W47");
    expect(sunday.start).toEqual(local(2026, 11, 9));
    expect(sunday.end).toEqual(monday.start);
    expect(periodOf("week", local(2027, 1, 1)).key).toBe("S-2026-W53"); // a Friday in 2026's last ISO week
    expect(periodOf("week", local(2025, 12, 29)).key).toBe("S-2026-W01");
  });

  it("days and months follow local midnight and the calendar", () => {
    expect(periodOf("day", local(2026, 11, 22, 23, 59)).key).toBe("J-2026-11-22");
    expect(periodOf("day", local(2026, 11, 23, 0, 0)).key).toBe("J-2026-11-23");
    const nov = periodOf("month", local(2026, 11, 22));
    expect(nov.key).toBe("M-2026-11");
    expect(nov.end).toEqual(local(2026, 12, 1));
    expect(previousPeriod(nov).key).toBe("M-2026-10");
  });
});

describe("draw", () => {
  const week = periodOf("week", local(2026, 11, 18));

  it("is the same every time, with 2 common challenges of different kinds and 1 personal one", () => {
    const a = drawChallenges(week, lib, "player-1");
    const b = drawChallenges(week, lib, "player-1");
    expect(a).toEqual(b);
    expect(a.filter((c) => c.family === "common")).toHaveLength(2);
    expect(new Set(a.filter((c) => c.family === "common").map((c) => c.kind)).size).toBe(2);
    expect(a.filter((c) => c.family === "personal")).toHaveLength(1);
    expect(a.every((c) => c.key.startsWith("S-2026-W47:"))).toBe(true);
  });

  it("gives everyone the same common challenges, each their own personal one", () => {
    const commons = (player: string) => drawChallenges(week, lib, player).filter((c) => c.family === "common").map((c) => c.id);
    expect(commons("player-1")).toEqual(commons("player-2"));
  });

  it("never repeats last week's common challenges", () => {
    for (let w = 0; w < 30; w++) {
      const p = periodOf("week", local(2026, 1, 5 + w * 7));
      const now = drawChallenges(p, lib, "x").filter((c) => c.family === "common").map((c) => c.id);
      const before = drawChallenges(previousPeriod(p), lib, "x").filter((c) => c.family === "common").map((c) => c.id);
      expect(now.some((id) => before.includes(id)), p.key).toBe(false);
    }
  });

  it("only draws personal challenges the library makes possible", () => {
    const small: Library = { games: [{ name: "Tetris", console: "Game Boy" }], playtime: { "Game Boy::Tetris": { seconds: 100, first_played: "2025-01-01T00:00:00Z" } } };
    for (let d = 0; d < 60; d++) {
      for (const p of [periodOf("day", local(2026, 3, 1 + d)), periodOf("week", local(2026, 3, 1 + d))]) {
        for (const c of drawChallenges(p, small, "x").filter((c) => c.family === "personal")) {
          expect(c.series, c.id).toBeUndefined(); // no series in this library
          expect(c.kind, c.id).not.toBe("discovery"); // every game was played
          expect(c.kind, c.id).not.toBe("comeback"); // no favorite
          if (c.console) expect(c.console).toBe("Game Boy");
        }
      }
    }
  });

  it("has one challenge a day and one a month", () => {
    expect(drawChallenges(periodOf("day", local(2026, 11, 22)), lib, "x")).toHaveLength(1);
    const month = drawChallenges(periodOf("month", local(2026, 11, 22)), lib, "x");
    expect(month).toHaveLength(1);
    expect(month[0].family).toBe("common");
  });
});

describe("progress", () => {
  const week = periodOf("week", local(2026, 11, 18)); // Mon 16 → Sun 22
  const c = (over: Partial<Challenge>): Challenge => ({ id: "x", key: "k", rhythm: "week", kind: "time", family: "common", goal: 3 * 3600, points: 30, ...over });

  it("counts play time inside the period only, even a session across Sunday midnight", () => {
    const sessions = [
      session("Super Mario Bros. 3", "NES", local(2026, 11, 22, 23, 30), 60), // 30 min this week, 30 next
      session("Metroid", "NES", local(2026, 11, 15, 23, 50), 20), // 10 min this week
      session("Super Mario 64", "Nintendo 64", local(2026, 11, 18, 20, 0), 90),
    ];
    expect(progress(c({}), week, sessions, lib)).toBe((30 + 10 + 90) * 60);
    expect(progress(c({ console: "NES" }), week, sessions, lib)).toBe(40 * 60);
    expect(progress(c({ series: "mario" }), week, sessions, lib)).toBe((30 + 90) * 60);
    expect(progress(c({ goal: 60 * 60 }), week, sessions, lib)).toBe(3600); // capped at the goal
  });

  it("counts different games, days with enough play and long enough sessions", () => {
    const sessions = [
      session("Super Mario Bros. 3", "NES", local(2026, 11, 16, 10), 20),
      session("Super Mario Bros. 3", "NES", local(2026, 11, 17, 10), 5),
      session("Metroid", "NES", local(2026, 11, 17, 12), 12),
      session("Super Mario 64", "Nintendo 64", local(2026, 11, 19, 10), 30),
    ];
    expect(progress(c({ kind: "distinct", goal: 4 }), week, sessions, lib)).toBe(3);
    expect(progress(c({ kind: "days", goal: 4, minSeconds: 15 * 60 }), week, sessions, lib)).toBe(3); // 16th: 20, 17th: 5+12, 19th: 30
    expect(progress(c({ kind: "sessions", goal: 6, minSeconds: 10 * 60 }), week, sessions, lib)).toBe(3);
  });

  it("discovery counts games never played before the period, comeback favorites left 30 days", () => {
    const sessions = [
      session("Unplayed Game", "Nintendo 64", local(2026, 11, 18, 10), 25),
      session("Super Mario 64", "Nintendo 64", local(2026, 11, 18, 11), 25), // played long ago
      session("The Legend of Zelda", "NES", local(2026, 11, 19, 10), 40), // favorite, nothing in the 30 days before
    ];
    const withFirst: Library = { ...lib, playtime: { ...lib.playtime, "Nintendo 64::Unplayed Game": { seconds: 1500, first_played: local(2026, 11, 18, 10, 25).toISOString() } } };
    expect(progress(c({ kind: "discovery", goal: 3600 }), week, sessions, withFirst)).toBe(25 * 60);
    expect(progress(c({ kind: "comeback", goal: 3600 }), week, sessions, lib)).toBe(40 * 60);
    const playedLastWeek = [...sessions, session("The Legend of Zelda", "NES", local(2026, 11, 10, 10), 10)];
    expect(progress(c({ kind: "comeback", goal: 3600 }), week, playedLastWeek, lib)).toBe(0);
  });
});

describe("rewards", () => {
  const now = local(2026, 11, 20);
  const week = periodOf("week", now);
  const ch = (id: string, kind: Challenge["kind"], goal: number): Challenge => ({ id, key: `${week.key}:${id}`, rhythm: "week", kind, family: "common", goal, points: 30 });

  it("records each challenge done once, and the bonus when the 3 of the week are done", () => {
    const items = [
      { challenge: ch("a", "time", 100), period: week, value: 100 },
      { challenge: ch("b", "distinct", 4), period: week, value: 4 },
      { challenge: ch("c", "days", 4), period: week, value: 2 },
    ];
    const first = newCompletions({}, items, now);
    expect(first.map((c) => c.key)).toEqual([`${week.key}:a`, `${week.key}:b`]);
    const history: History = Object.fromEntries(first.map((c) => [c.key, c]));
    expect(newCompletions(history, items, now)).toEqual([]);
    items[2].value = 4;
    const second = newCompletions(history, items, now);
    expect(second.map((c) => c.key)).toEqual([`${week.key}:c`, `${week.key}:bonus`]);
    expect(second[1].points).toBe(20);
  });

  it("adds points, counts the weeks in a row and the badges", () => {
    const done = (period: string, kind: Challenge["kind"], at: string): [string, History[string]] =>
      [`${period}:${kind}`, { key: `${period}:${kind}`, id: kind, kind, rhythm: "week", period, points: 30, at }];
    const history: History = Object.fromEntries([
      done("S-2026-W45", "time", "2026-11-03T10:00:00Z"),
      done("S-2026-W46", "time", "2026-11-10T10:00:00Z"),
      done("S-2026-W46", "days", "2026-11-11T10:00:00Z"),
      done("S-2026-W43", "distinct", "2026-10-20T10:00:00Z"),
    ]);
    const r = rewards(history, now); // week 47 has nothing yet: the streak is 45-46
    expect(r.points).toBe(120);
    expect(r.streak).toBe(2);
    expect(r.badges.find((b) => b.kind === "time")).toEqual({ kind: "time", count: 2, tier: "bronze" });
    expect(r.badges.find((b) => b.kind === "comeback")?.tier).toBeNull();
    expect(r.recent[0].period).toBe("S-2026-W46");
  });
});

describe("texts", () => {
  it("describes every kind in French, with its numbers", () => {
    const base = { key: "k", rhythm: "week" as const, family: "common" as const, points: 30 };
    const texts = [
      describeChallenge({ ...base, id: "a", kind: "time", goal: 3 * 3600 }, t, formatPlaytime),
      describeChallenge({ ...base, id: "b", kind: "time", goal: 1800, console: "NES" }, t, formatPlaytime),
      describeChallenge({ ...base, id: "c", kind: "time", goal: 7200, series: "zelda" }, t, formatPlaytime),
      describeChallenge({ ...base, id: "d", kind: "days", goal: 4, minSeconds: 900 }, t, formatPlaytime),
      describeChallenge({ ...base, id: "e", kind: "discovery", goal: 1200 }, t, formatPlaytime),
    ];
    expect(texts[0].description).toBe("Joue 3 h 00 au total");
    expect(texts[1].description).toBe("Joue 30 min sur NES");
    expect(texts[2].description).toBe("Joue 2 h 00 à un jeu Zelda");
    expect(texts[3].description).toBe("Joue au moins 15 min sur 4 jours différents");
    expect(texts[4].description).toBe("Joue 20 min à un jeu que tu n'as jamais lancé");
    for (const x of texts) expect(x.title).not.toMatch(/^challenges2\./);
  });
});

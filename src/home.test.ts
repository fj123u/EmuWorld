import { describe, expect, it } from "vitest";
import { consoleTiles, formatPlaytime, friendsOnline, monthSummary, recentGames } from "./home";

const rom = (name: string, console = "NES") => ({ name, console, path: `D:\\${name}` });

describe("home data", () => {
  it("lists the library's games by last session, skipping unplayed and missing ones", () => {
    const roms = [rom("A"), rom("B"), rom("C"), rom("D")];
    const games = {
      "NES::A": { seconds: 60, launches: 1, last_played: "2026-11-01T10:00:00Z" },
      "NES::B": { seconds: 120, launches: 2, last_played: "2026-11-10T10:00:00Z" },
      "NES::C": { seconds: 0, launches: 0, last_played: null },
      "NES::D": { seconds: 5, launches: 1, last_played: "not-a-date" },
      "NES::Gone": { seconds: 999, launches: 9, last_played: "2026-11-11T10:00:00Z" },
    };
    expect(recentGames(roms, games, 5).map((g) => g.rom.name)).toEqual(["B", "A"]);
    expect(recentGames(roms, games, 1)).toEqual([{ rom: roms[1], seconds: 120, lastPlayed: "2026-11-10T10:00:00Z" }]);
  });

  it("makes one tile per console, most games first", () => {
    expect(consoleTiles([rom("a", "NES"), rom("b", "GBA"), rom("c", "GBA"), rom("d", "Atari 2600")])).toEqual([
      { name: "GBA", count: 2 }, { name: "Atari 2600", count: 1 }, { name: "NES", count: 1 },
    ]);
  });

  it("sums this month only: play time, days played, achievements", () => {
    const now = new Date(2026, 10, 11);
    const games = {
      a: { seconds: 0, launches: 0, last_played: null, monthly: { "2026-11": { seconds: 3600 }, "2026-10": { seconds: 9999 } }, active_days: ["2026-10-31", "2026-11-02", "2026-11-05"] },
      b: { seconds: 0, launches: 0, last_played: null, monthly: { "2026-11": { seconds: 1800 } }, active_days: ["2026-11-05"] },
    };
    const achievements = [
      { unlocked: true, unlocked_at: new Date(2026, 10, 3).toISOString() },
      { unlocked: true, unlocked_at: new Date(2026, 9, 3).toISOString() },
      { unlocked: false, unlocked_at: null },
    ];
    expect(monthSummary(games, achievements, now)).toEqual({ seconds: 5400, activeDays: 2, achievements: 1 });
  });

  it("formats play time", () => {
    expect(formatPlaytime(11160)).toBe("3 h 06");
    expect(formatPlaytime(2880)).toBe("48 min");
    expect(formatPlaytime(-5)).toBe("0 min");
  });

  it("keeps friends whose app is open, playing ones first", () => {
    const now = Date.parse("2026-11-11T12:00:00Z");
    const friends = [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }];
    const presences = [
      { user_id: "a", status: "online" as const, current_game: null, updated_at: "2026-11-11T11:59:30Z" },
      { user_id: "b", status: "playing" as const, current_game: "Zelda", updated_at: "2026-11-11T11:59:00Z" },
      { user_id: "c", status: "online" as const, current_game: null, updated_at: "2026-11-11T11:00:00Z" }, // app closed long ago
      { user_id: "d", status: "offline" as const, current_game: null, updated_at: "2026-11-11T11:59:59Z" },
    ];
    expect(friendsOnline(friends, presences, now)).toEqual([{ friend: { id: "b" }, playing: "Zelda" }, { friend: { id: "a" }, playing: null }]);
  });
});

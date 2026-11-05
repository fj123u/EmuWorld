import { describe, expect, it } from "vitest";
import { matchesSmartPlaylist, isSmartPlaylist, SMART_PLAYLISTS } from "./smartPlaylists";

const NOW = Date.parse("2026-11-05T12:00:00Z");
const daysAgo = (d: number) => new Date(NOW - d * 86_400_000).toISOString();
const game = (seconds: number, launches: number, lastPlayedDaysAgo: number | null) => ({
  seconds,
  launches,
  last_played: lastPlayedDaysAgo === null ? null : daysAgo(lastPlayedDaysAgo),
});

describe("smart playlists", () => {
  it("never lists a game that was never launched", () => {
    for (const p of SMART_PLAYLISTS) {
      expect(matchesSmartPlaylist(p, undefined, NOW)).toBe(false);
      expect(matchesSmartPlaylist(p, game(0, 0, null), NOW)).toBe(false);
    }
  });

  it("splits games by how long ago they were played", () => {
    expect(matchesSmartPlaylist("recent", game(600, 1, 2), NOW)).toBe(true);
    expect(matchesSmartPlaylist("recent", game(600, 1, 10), NOW)).toBe(false);
    expect(matchesSmartPlaylist("stale", game(600, 1, 45), NOW)).toBe(true);
    expect(matchesSmartPlaylist("stale", game(600, 1, 20), NOW)).toBe(false);
  });

  it("ignores an unreadable date instead of guessing", () => {
    const corrupt = { seconds: 600, launches: 1, last_played: "not-a-date" };
    expect(matchesSmartPlaylist("recent", corrupt, NOW)).toBe(false);
    expect(matchesSmartPlaylist("stale", corrupt, NOW)).toBe(false);
  });

  it("finds barely started, short-session and long games", () => {
    expect(matchesSmartPlaylist("started", game(20 * 60, 1, 3), NOW)).toBe(true);
    expect(matchesSmartPlaylist("started", game(2 * 3600, 2, 3), NOW)).toBe(false);
    expect(matchesSmartPlaylist("started", game(30 * 60, 5, 3), NOW)).toBe(false);

    expect(matchesSmartPlaylist("short", game(4 * 15 * 60, 4, 3), NOW)).toBe(true);
    expect(matchesSmartPlaylist("short", game(15 * 60, 1, 3), NOW)).toBe(false); // one session says little
    expect(matchesSmartPlaylist("short", game(4 * 3600, 4, 3), NOW)).toBe(false);

    expect(matchesSmartPlaylist("long", game(10 * 3600, 3, 3), NOW)).toBe(true);
    expect(matchesSmartPlaylist("long", game(9 * 3600, 3, 3), NOW)).toBe(false);
  });

  it("recognises playlist ids", () => {
    expect(isSmartPlaylist("stale")).toBe(true);
    expect(isSmartPlaylist("favorites")).toBe(false);
  });
});

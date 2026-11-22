import { beforeEach, describe, expect, it } from "vitest";
import { drawnChallenges } from "./challengeStore";
import { periodOf, type Challenge } from "./challenges";

const week = periodOf("week", new Date(2026, 10, 18));
const ch = (id: string): Challenge => ({ id, key: `${week.key}:${id}`, rhythm: "week", kind: "time", family: "common", goal: 60, points: 30 });

describe("drawn challenges", () => {
  beforeEach(() => localStorage.clear());

  it("keeps a full draw for the whole period", () => {
    expect(drawnChallenges(week, () => [ch("a"), ch("b"), ch("c")]).map((c) => c.id)).toEqual(["a", "b", "c"]);
    // A new ROM changes what would be drawn: the week keeps its challenges.
    expect(drawnChallenges(week, () => [ch("x"), ch("y"), ch("z")]).map((c) => c.id)).toEqual(["a", "b", "c"]);
  });

  it("draws again when the first draw was incomplete (library not loaded yet)", () => {
    expect(drawnChallenges(week, () => [ch("a"), ch("b")])).toHaveLength(2);
    expect(drawnChallenges(week, () => [ch("a"), ch("b"), ch("p")]).map((c) => c.id)).toEqual(["a", "b", "p"]);
  });
});

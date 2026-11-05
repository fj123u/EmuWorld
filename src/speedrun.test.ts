import { describe, expect, it } from "vitest";
import {
  deltaVsPb,
  elapsed,
  emptyData,
  exportSummary,
  formatDelta,
  formatTime,
  isGoldSegment,
  newRun,
  recordRun,
  skip,
  split,
  sumOfBest,
  togglePause,
  undo,
  type SpeedrunData,
} from "./speedrun";

const data3 = (over: Partial<SpeedrunData> = {}): SpeedrunData => ({
  ...emptyData(), splits: ["Monde 1", "Monde 2", "Bowser"], best_segments: [null, null, null], ...over,
});

/** Runs splits at the given clock times (first value starts the run at t). */
function play(times: number[], count = 3) {
  let run = newRun();
  for (const t of times) run = split(run, t, count);
  return run;
}

describe("speedrun timer", () => {
  it("starts on the first split and finishes on the last segment", () => {
    let run = split(newRun(), 1000, 3);
    expect(run.status).toBe("running");
    expect(elapsed(run, 61_000)).toBe(60_000);
    run = split(run, 61_000, 3);
    run = split(run, 151_000, 3);
    expect(run.status).toBe("running");
    run = split(run, 301_000, 3);
    expect(run.status).toBe("finished");
    expect(run.splits).toEqual([60_000, 150_000, 300_000]);
    expect(elapsed(run, 999_999), "the clock stops on the last split").toBe(300_000);
  });

  it("excludes paused time", () => {
    let run = split(newRun(), 0, 2);
    run = togglePause(run, 10_000);
    expect(elapsed(run, 50_000)).toBe(10_000);
    run = togglePause(run, 50_000);
    expect(elapsed(run, 55_000)).toBe(15_000);
    run = split(run, 60_000, 2);
    expect(run.splits).toEqual([20_000]);
  });

  it("skips a segment (never the last) and undoes a split", () => {
    let run = split(newRun(), 0, 3);
    run = skip(run, 3);
    run = skip(run, 3);
    expect(run.splits).toEqual([null, null]);
    run = skip(run, 3);
    expect(run.splits, "the last segment can't be skipped").toEqual([null, null]);
    run = split(run, 90_000, 3);
    expect(run.status).toBe("finished");
    run = undo(run);
    expect(run.status).toBe("running");
    expect(run.splits).toEqual([null, null]);
  });

  it("saves a faster finished run as the new PB and keeps best segments", () => {
    const first = recordRun(data3({ attempts: 1 }), play([0, 60_000, 150_000, 300_000]));
    expect(first.newPb).toBe(true);
    expect(first.data.pb).toEqual([60_000, 150_000, 300_000]);
    expect(first.data.best_segments).toEqual([60_000, 90_000, 150_000]);
    expect(first.data.completed).toBe(1);

    // Slower overall, but a faster Monde 2: PB kept, that segment improves.
    const second = recordRun({ ...first.data, attempts: 2 }, play([0, 70_000, 140_000, 320_000]));
    expect(second.newPb).toBe(false);
    expect(second.data.pb).toEqual([60_000, 150_000, 300_000]);
    expect(second.data.best_segments).toEqual([60_000, 70_000, 150_000]);
    expect(sumOfBest(second.data)).toBe(280_000);
  });

  it("learns from a reset run without touching the PB", () => {
    const base = data3({ pb: [60_000, 150_000, 300_000], best_segments: [60_000, 90_000, 150_000], attempts: 3, completed: 1 });
    const reset = recordRun(base, play([0, 55_000]));
    expect(reset.newPb).toBe(false);
    expect(reset.data.pb).toEqual(base.pb);
    expect(reset.data.best_segments[0]).toBe(55_000);
    expect(reset.data.completed).toBe(1);
  });

  it("never makes a PB out of a run with skipped segments", () => {
    let run = split(newRun(), 0, 3);
    run = skip(run, 3);
    run = split(run, 100_000, 3);
    run = split(run, 200_000, 3);
    expect(recordRun(data3(), run).newPb).toBe(false);
  });

  it("compares with the PB and spots gold segments", () => {
    const data = data3({ pb: [60_000, 150_000, 300_000], best_segments: [60_000, 90_000, 150_000] });
    const run = play([0, 58_000, 160_000]);
    expect(deltaVsPb(data, run, 0)).toBe(-2_000);
    expect(deltaVsPb(data, run, 1)).toBe(10_000);
    expect(isGoldSegment(data, run, 0)).toBe(true);
    expect(isGoldSegment(data, run, 1)).toBe(false);
    expect(deltaVsPb(data3(), run, 0), "no PB yet").toBeNull();
  });

  it("formats times and deltas", () => {
    expect(formatTime(3_723_450)).toBe("1:02:03.45");
    expect(formatTime(123_450)).toBe("2:03.45");
    expect(formatTime(3_450)).toBe("3.45");
    expect(formatTime(3_450, 0)).toBe("3");
    expect(formatDelta(1_234)).toBe("+1.2");
    expect(formatDelta(-800)).toBe("-0.8");
    expect(formatDelta(0)).toBe("±0.0");
  });

  it("exports a shareable summary", () => {
    const data = data3({ pb: [60_000, 150_000, 300_000], best_segments: [60_000, 90_000, 150_000], attempts: 4, completed: 2 });
    const text = exportSummary("Super Mario Bros. 3", data, { pb: "Record", sob: "Meilleur possible", attempts: "Runs finies" });
    expect(text).toContain("🏁 Super Mario Bros. 3");
    expect(text).toContain("Record : 5:00.00");
    expect(text).toContain("  Monde 2 — 2:30.00 (1:30.00)");
    expect(text).toContain("Meilleur possible : 5:00.00");
    expect(text).toContain("Runs finies : 2/4");
  });
});

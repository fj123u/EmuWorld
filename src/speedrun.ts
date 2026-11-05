// Speedrun timer logic (pure, no UI): the run state machine, records, deltas
// and formatting. Storage: src-tauri/src/speedrun.rs (speedruns.json).

export interface SpeedrunData {
  /** Segment names, in order; the last one ends the run. */
  splits: string[];
  /** Personal best: time since the start at each split, in ms. */
  pb: number[] | null;
  /** Best duration ever of each segment, in ms. */
  best_segments: (number | null)[];
  attempts: number;
  completed: number;
}

export type RunStatus = "idle" | "running" | "paused" | "finished";

export interface RunState {
  status: RunStatus;
  /** performance-style clock (ms) when the run started. */
  startedAt: number | null;
  pausedAt: number | null;
  pausedTotal: number;
  /** Time since the start at each split done so far; `null` = skipped. */
  splits: (number | null)[];
}

export type HotkeyAction = "split" | "skip" | "reset" | "pause" | "undo";

/** One segment called "Fin" when the player hasn't set any. */
export const DEFAULT_SPLITS = ["Fin"];

export function emptyData(): SpeedrunData {
  return { splits: [], pb: null, best_segments: [], attempts: 0, completed: 0 };
}

export function segmentNames(data: SpeedrunData): string[] {
  return data.splits.length > 0 ? data.splits : DEFAULT_SPLITS;
}

export function newRun(): RunState {
  return { status: "idle", startedAt: null, pausedAt: null, pausedTotal: 0, splits: [] };
}

/** Time on the clock, paused time excluded. */
export function elapsed(run: RunState, now: number): number {
  if (run.startedAt === null) return 0;
  const end = run.status === "paused" && run.pausedAt !== null ? run.pausedAt : now;
  if (run.status === "finished") {
    const last = [...run.splits].reverse().find((t) => t !== null);
    return last ?? 0;
  }
  return Math.max(0, end - run.startedAt - run.pausedTotal);
}

/** Split: starts an idle run, records the current segment, ends the run on the last one. */
export function split(run: RunState, now: number, segmentCount: number): RunState {
  if (run.status === "idle") return { ...newRun(), status: "running", startedAt: now };
  if (run.status !== "running") return run;
  const splits = [...run.splits, elapsed(run, now)];
  return { ...run, splits, status: splits.length >= segmentCount ? "finished" : "running" };
}

/** Skips a segment (no time for it). The last segment can't be skipped. */
export function skip(run: RunState, segmentCount: number): RunState {
  if (run.status !== "running" || run.splits.length >= segmentCount - 1) return run;
  return { ...run, splits: [...run.splits, null] };
}

export function undo(run: RunState): RunState {
  if ((run.status !== "running" && run.status !== "finished") || run.splits.length === 0) return run;
  return { ...run, status: "running", splits: run.splits.slice(0, -1) };
}

export function togglePause(run: RunState, now: number): RunState {
  if (run.status === "running") return { ...run, status: "paused", pausedAt: now };
  if (run.status === "paused" && run.pausedAt !== null) {
    return { ...run, status: "running", pausedTotal: run.pausedTotal + (now - run.pausedAt), pausedAt: null };
  }
  return run;
}

/** Duration of each segment of a run (`null` when it or the previous split was skipped). */
export function segmentDurations(splits: (number | null)[]): (number | null)[] {
  return splits.map((t, i) => {
    if (t === null) return null;
    if (i === 0) return t;
    const prev = splits[i - 1];
    return prev === null ? null : t - prev;
  });
}

/**
 * Records a run that ended (finished or reset midway): best segments are
 * kept from any run, the PB only changes on a finished, faster run.
 */
export function recordRun(data: SpeedrunData, run: RunState): { data: SpeedrunData; newPb: boolean } {
  const count = segmentNames(data).length;
  const splitsNames = data.splits.length > 0 ? data.splits : [...DEFAULT_SPLITS];
  const bests = Array.from({ length: count }, (_, i) => data.best_segments[i] ?? null);
  segmentDurations(run.splits).forEach((d, i) => {
    if (d !== null && d > 0 && i < count && (bests[i] === null || d < (bests[i] as number))) bests[i] = d;
  });
  const finished = run.status === "finished" && run.splits.length === count && run.splits[count - 1] !== null;
  const final = finished ? (run.splits[count - 1] as number) : null;
  const pbFinal = data.pb ? data.pb[data.pb.length - 1] : null;
  const newPb = final !== null && run.splits.every((t) => t !== null) && (pbFinal === null || final < pbFinal);
  return {
    newPb,
    data: {
      ...data,
      splits: splitsNames,
      best_segments: bests,
      pb: newPb ? (run.splits as number[]) : data.pb,
      completed: data.completed + (finished ? 1 : 0),
    },
  };
}

/** Ahead (< 0) or behind (> 0) the PB at split `i`, in ms. */
export function deltaVsPb(data: SpeedrunData, run: RunState, i: number): number | null {
  const t = run.splits[i];
  const pb = data.pb?.[i];
  return t === null || t === undefined || pb === undefined ? null : t - pb;
}

/** A segment faster than any before it (shown in gold). */
export function isGoldSegment(data: SpeedrunData, run: RunState, i: number): boolean {
  const d = segmentDurations(run.splits)[i];
  const best = data.best_segments[i];
  return d !== null && d !== undefined && d > 0 && (best === null || best === undefined || d < best);
}

/** Sum of best segments: the best time possible with today's skills. */
export function sumOfBest(data: SpeedrunData): number | null {
  const n = segmentNames(data).length;
  if (data.best_segments.length !== n || data.best_segments.some((b) => b === null)) return null;
  return (data.best_segments as number[]).reduce((a, b) => a + b, 0);
}

/** 1:02:03.45, 2:03.45 or 3.45 */
export function formatTime(ms: number, decimals: 0 | 1 | 2 = 2): string {
  const negative = ms < 0;
  const total = Math.abs(ms);
  const h = Math.floor(total / 3_600_000);
  const m = Math.floor((total % 3_600_000) / 60_000);
  const s = Math.floor((total % 60_000) / 1000);
  const frac = decimals === 0 ? "" : "." + String(Math.floor((total % 1000) / (decimals === 1 ? 100 : 10))).padStart(decimals, "0");
  const body = h > 0 ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : m > 0 ? `${m}:${String(s).padStart(2, "0")}` : `${s}`;
  return (negative ? "-" : "") + body + frac;
}

/** +1.2 / -0.8 (deltas against the PB). */
export function formatDelta(ms: number): string {
  return (ms > 0 ? "+" : ms < 0 ? "-" : "±") + formatTime(Math.abs(ms), 1);
}

/** Text summary of a game's records, to share. */
export function exportSummary(gameName: string, data: SpeedrunData, labels: { pb: string; sob: string; attempts: string }): string {
  const names = segmentNames(data);
  const lines = [`🏁 ${gameName}`];
  if (data.pb) {
    lines.push(`${labels.pb} : ${formatTime(data.pb[data.pb.length - 1])}`);
    const durations = segmentDurations(data.pb);
    names.forEach((n, i) => lines.push(`  ${n} — ${formatTime(data.pb![i])} (${formatTime(durations[i] ?? 0)})`));
  }
  const sob = sumOfBest(data);
  if (sob !== null) lines.push(`${labels.sob} : ${formatTime(sob)}`);
  lines.push(`${labels.attempts} : ${data.completed}/${data.attempts}`);
  return lines.join("\n");
}

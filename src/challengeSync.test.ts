import { describe, expect, it } from "vitest";
import {
  commonProgressRows, insertVersus, loadFriendsProgress, missingSchema, pushProgress, ranking, syncVersus, versusProgress, versusRules,
  type Client, type FriendProgress, type Versus,
} from "./challengeSync";
import { periodOf, type Challenge, type Library, type Session } from "./challenges";

/** A fake Supabase client: records calls, answers with `answer(table, op)`. */
function fakeClient(answer: (table: string, op: string, args: unknown) => { data?: unknown; error?: { code?: string; message?: string } | null } = () => ({ data: [], error: null })) {
  const calls: { table: string; op: string; args: unknown; filters: unknown[] }[] = [];
  const client: Client = {
    from(table: string) {
      const call = { table, op: "", args: undefined as unknown, filters: [] as unknown[] };
      const chain: Record<string, unknown> = {};
      for (const op of ["select", "insert", "upsert", "update"]) {
        chain[op] = (args: unknown) => { call.op = op; call.args = args; calls.push(call); return chain; };
      }
      for (const f of ["eq", "in"]) chain[f] = (...args: unknown[]) => { call.filters.push([f, ...args]); return chain; };
      chain.then = (resolve: (v: unknown) => void) => resolve({ data: [], error: null, ...answer(table, call.op, call.args) });
      return chain;
    },
    rpc(fn: string, args: Record<string, unknown>) {
      calls.push({ table: fn, op: "rpc", args, filters: [] });
      return Promise.resolve({ data: null, error: null, ...answer(fn, "rpc", args) });
    },
  };
  return { client, calls };
}

const week = periodOf("week", new Date(2026, 10, 18));
const month = periodOf("month", new Date(2026, 10, 18));
const ch = (over: Partial<Challenge> & Pick<Challenge, "id" | "rhythm" | "family">): Challenge =>
  ({ key: `${over.rhythm}:${over.id}`, kind: "time", goal: 3600, points: 30, ...over });
const missing = { code: "42P01", message: 'relation "public.challenge_progress" does not exist' };

describe("common challenges with friends", () => {
  const items = [
    { challenge: ch({ id: "day_time", rhythm: "day", family: "personal" }), period: periodOf("day", new Date(2026, 10, 18)), value: 100 },
    { challenge: ch({ id: "week_time_3h", rhythm: "week", family: "common", goal: 10800 }), period: week, value: 4000 },
    { challenge: ch({ id: "week_console", rhythm: "week", family: "personal", console: "NES" }), period: week, value: 50 },
    { challenge: ch({ id: "month_days_15", rhythm: "month", family: "common", kind: "days", goal: 15 }), period: month, value: 20 },
  ];

  it("sends only the common challenges of the week and the month, capped at their goal", () => {
    expect(commonProgressRows("me", items)).toEqual([
      { user_id: "me", period_key: week.key, challenge_id: "week_time_3h", progress: 4000, goal: 10800 },
      { user_id: "me", period_key: month.key, challenge_id: "month_days_15", progress: 15, goal: 15 },
    ]);
  });

  it("upserts them, and says when the table is not there yet", async () => {
    const ok = fakeClient();
    expect(await pushProgress(ok.client, commonProgressRows("me", items))).toBe("ok");
    expect(ok.calls[0]).toMatchObject({ table: "challenge_progress", op: "upsert" });
    const off = fakeClient(() => ({ error: missing }));
    expect(await pushProgress(off.client, commonProgressRows("me", items))).toBe("missing");
    expect(await loadFriendsProgress(off.client, [week.key], ["me"])).toBe("missing");
    expect(missingSchema({ code: "PGRST204", message: "Could not find the 'rules' column" })).toBe(true);
    expect(missingSchema({ code: "23505", message: "duplicate key" })).toBe(false);
  });

  it("ranks friends by progress, the first to finish first on a tie", () => {
    const rows: FriendProgress[] = [
      { user_id: "a", period_key: week.key, challenge_id: "week_time_3h", progress: 10800, goal: 10800, completed_at: "2026-11-18T10:00:00Z" },
      { user_id: "me", period_key: week.key, challenge_id: "week_time_3h", progress: 4000, goal: 10800, completed_at: null },
      { user_id: "b", period_key: week.key, challenge_id: "week_time_3h", progress: 10800, goal: 10800, completed_at: "2026-11-17T10:00:00Z" },
      { user_id: "c", period_key: "S-2026-W46", challenge_id: "week_time_3h", progress: 9999, goal: 10800, completed_at: null },
    ];
    const profiles = new Map([["a", { username: "Alice", avatar_url: null }], ["b", { username: "Bob", avatar_url: null }], ["me", { username: "Moi", avatar_url: null }]]);
    const list = ranking(rows, { id: "week_time_3h" }, week.key, profiles, "me");
    expect(list.map((e) => e.name)).toEqual(["Bob", "Alice", "Moi"]);
    expect(list[2]).toMatchObject({ me: true, done: false });
  });
});

describe("versus on the challenges engine", () => {
  const lib: Library = { games: [], playtime: {} };
  const s = (name: string, console: string, start: string, minutes: number): Session => ({ name, console, start, seconds: minutes * 60, emulator: "x" });
  const versus = (over: Partial<Versus> = {}): Versus => ({
    id: "v1", challenger_id: "me", opponent_id: "friend", challenge_type: "playtime", game_name: null, game_console: null,
    status: "active", created_at: "2026-11-16T10:00:00Z", started_at: "2026-11-16T12:00:00Z", ends_at: "2026-11-23T12:00:00Z",
    duration_days: 7, challenger_progress: 0, opponent_progress: 0, winner_id: null, ...over,
  });
  const sessions = [
    s("Zelda", "NES", "2026-11-16T11:00:00Z", 120), // 60 min before the start
    s("Zelda", "NES", "2026-11-17T20:00:00Z", 30),
    s("Mario", "SNES", "2026-11-18T20:00:00Z", 45),
    s("Zelda", "NES", "2026-11-24T20:00:00Z", 90), // after the end
  ];
  const now = new Date("2026-11-25T00:00:00Z");

  it("reads the rules, or the old type of a versus made before them", () => {
    expect(versusRules({ rules: null, challenge_type: "launches", game_name: null, game_console: null }).kind).toBe("sessions");
    expect(versusRules({ rules: null, challenge_type: "streak", game_name: null, game_console: null }).kind).toBe("days");
    expect(versusRules({ rules: { kind: "distinct", console: "NES" }, challenge_type: "playtime", game_name: null, game_console: null })).toEqual({ kind: "distinct", console: "NES", game: undefined });
  });

  it("counts only what was played between the start and the end", () => {
    expect(versusProgress(versus(), sessions, lib, now)).toBe((60 + 30 + 45) * 60);
    expect(versusProgress(versus({ game_name: "Zelda" }), sessions, lib, now)).toBe((60 + 30) * 60);
    expect(versusProgress(versus({ challenge_type: "launches" }), sessions, lib, now)).toBe(3);
    expect(versusProgress(versus({ challenge_type: "streak" }), sessions, lib, now)).toBe(3);
    expect(versusProgress(versus({ status: "pending", started_at: null, created_at: "2026-11-30T00:00:00Z" }), sessions, lib, now)).toBe(0);
  });

  it("sends my own score only when it went up, and closes a versus whose time is up", async () => {
    const { client, calls } = fakeClient();
    const list = [versus(), versus({ id: "v2", challenger_id: "friend", opponent_id: "me", opponent_progress: 999_999 }), versus({ id: "v3", status: "pending" })];
    const result = await syncVersus(client, "me", list, sessions, lib, now);
    expect(result).toEqual({ updated: 1, closed: 2 });
    const update = calls.find((c) => c.op === "update")!;
    expect(update.args).toEqual({ challenger_progress: (60 + 30 + 45) * 60 });
    expect(update.filters).toEqual([["eq", "id", "v1"]]);
    expect(calls.filter((c) => c.op === "rpc").map((c) => c.args)).toEqual([{ versus_id: "v1" }, { versus_id: "v2" }]);
  });

  it("creates a versus with its rules, or without them before the SQL is applied", async () => {
    const ok = fakeClient();
    expect(await insertVersus(ok.client, { challenge_type: "playtime", rules: { kind: "time" } })).toBeNull();
    expect(ok.calls).toHaveLength(1);
    const old = fakeClient((_t, _op, args) => ((args as Record<string, unknown>).rules ? { error: { code: "PGRST204", message: "Could not find the 'rules' column" } } : {}));
    expect(await insertVersus(old.client, { challenge_type: "playtime", rules: { kind: "time" } })).toBeNull();
    expect(old.calls.map((c) => c.args)).toEqual([{ challenge_type: "playtime", rules: { kind: "time" } }, { challenge_type: "playtime" }]);
  });
});

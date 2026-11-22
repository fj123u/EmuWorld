// Challenges v2 with friends (steps 4 and 6 of scratch/defis-refonte.md):
// the common challenges' progress sent to Supabase for the friends ranking,
// and versus computed with the same engine as the challenges.
//
// Tables: scratch/supabase_challenges_v2.sql. Until it is applied, every call
// here sees "table / column missing" and turns the feature off quietly: the
// local challenges keep working.
import { progress, type Challenge, type Kind, type Library, type Period, type Session } from "./challenges";

/** The few Supabase calls used here (the real client, or a fake in tests). */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Client = { from: (table: string) => any; rpc: (fn: string, args: Record<string, unknown>) => any };

interface PgError { code?: string; message?: string }

/** The table, column or function of supabase_challenges_v2.sql is not there yet. */
export function missingSchema(error: PgError | null | undefined): boolean {
  if (!error) return false;
  return ["42P01", "42703", "42883", "PGRST202", "PGRST204", "PGRST205"].includes(error.code ?? "")
    || /does not exist|schema cache|could not find/i.test(error.message ?? "");
}

// ---- common challenges: progress and ranking ---------------------------------------

export interface ProgressRow {
  user_id: string;
  period_key: string;
  challenge_id: string;
  progress: number;
  goal: number;
}

/** Rows to send: the common challenges of the week and the month (personal ones are not ranked). */
export function commonProgressRows(userId: string, items: { challenge: Challenge; period: Period; value: number }[]): ProgressRow[] {
  return items
    .filter((i) => i.challenge.family === "common" && i.challenge.rhythm !== "day")
    .map((i) => ({ user_id: userId, period_key: i.period.key, challenge_id: i.challenge.id, progress: Math.min(i.value, i.challenge.goal), goal: i.challenge.goal }));
}

export type SyncResult = "ok" | "missing" | "error";

export async function pushProgress(client: Client, rows: ProgressRow[]): Promise<SyncResult> {
  if (rows.length === 0) return "ok";
  const { error } = await client.from("challenge_progress").upsert(rows, { onConflict: "user_id,period_key,challenge_id" });
  if (!error) return "ok";
  return missingSchema(error) ? "missing" : "error";
}

export interface FriendProgress {
  user_id: string;
  period_key: string;
  challenge_id: string;
  progress: number;
  goal: number;
  completed_at: string | null;
}

/** Progress of the player and their friends (the table only shows those) on some periods. */
export async function loadFriendsProgress(client: Client, periodKeys: string[], userIds: string[]): Promise<FriendProgress[] | "missing"> {
  if (userIds.length === 0) return [];
  const { data, error } = await client.from("challenge_progress").select("user_id, period_key, challenge_id, progress, goal, completed_at")
    .in("period_key", periodKeys).in("user_id", userIds);
  if (error) return missingSchema(error) ? "missing" : [];
  return Array.isArray(data) ? data : [];
}

export interface RankEntry {
  userId: string;
  name: string;
  avatar: string | null;
  progress: number;
  goal: number;
  done: boolean;
  me: boolean;
}

/** Ranking of one challenge: most progress first, the first to finish ahead on a tie. */
export function ranking(
  rows: FriendProgress[], challenge: Pick<Challenge, "id">, periodKey: string,
  profiles: Map<string, { username: string | null; avatar_url: string | null }>, me: string,
): RankEntry[] {
  return rows
    .filter((r) => r.period_key === periodKey && r.challenge_id === challenge.id)
    .sort((a, b) => b.progress - a.progress || (a.completed_at ?? "~").localeCompare(b.completed_at ?? "~"))
    .map((r) => ({
      userId: r.user_id,
      name: profiles.get(r.user_id)?.username ?? "?",
      avatar: profiles.get(r.user_id)?.avatar_url ?? null,
      progress: r.progress,
      goal: r.goal,
      done: r.progress >= r.goal,
      me: r.user_id === me,
    }));
}

// ---- versus -----------------------------------------------------------------------------

export interface Versus {
  id: string;
  challenger_id: string;
  opponent_id: string;
  challenge_type: string;
  game_name: string | null;
  game_console: string | null;
  status: string;
  created_at: string;
  started_at?: string | null;
  ends_at: string | null;
  duration_days: number;
  challenger_progress: number;
  opponent_progress: number;
  winner_id: string | null;
  rules?: { kind?: Kind; console?: string; game?: string } | null;
}

export type VersusKind = "time" | "distinct" | "days" | "sessions";

/** What a versus measures: its rules, or the type of a versus made before them. */
export function versusRules(v: Pick<Versus, "rules" | "challenge_type" | "game_name" | "game_console">): { kind: VersusKind; console?: string; game?: string } {
  const old: Record<string, VersusKind> = { playtime: "time", launches: "sessions", streak: "days" };
  const kind = (v.rules?.kind as VersusKind | undefined) ?? old[v.challenge_type] ?? "time";
  return {
    kind,
    console: v.rules?.console ?? v.game_console ?? undefined,
    game: v.rules?.game ?? v.game_name ?? undefined,
  };
}

/** The type column a new versus needs (the table only accepts the old names). */
export function versusType(kind: VersusKind): string {
  return kind === "sessions" ? "launches" : kind === "days" ? "streak" : "playtime";
}

/** My score in a versus, from my sessions between its start and its end. */
export function versusProgress(v: Versus, sessions: Session[], lib: Library, now: Date): number {
  const rules = versusRules(v);
  const start = new Date(v.started_at ?? v.created_at);
  const end = v.ends_at ? new Date(Math.min(Date.parse(v.ends_at), now.getTime())) : now;
  if (!(end > start)) return 0;
  const period: Period = { rhythm: "week", key: `V-${v.id}`, start, end };
  const challenge: Challenge = {
    id: "versus", key: `V-${v.id}`, rhythm: "week", family: "personal", points: 0,
    kind: rules.kind, goal: 1_000_000_000, console: rules.console, game: rules.game,
    // A day counts with any play; every session counts.
    minSeconds: rules.kind === "days" ? 1 : 0,
  };
  return progress(challenge, period, sessions, lib);
}

/** Sends my score of each running versus, and closes the ones whose time is up. */
export async function syncVersus(client: Client, me: string, list: Versus[], sessions: Session[], lib: Library, now: Date): Promise<{ updated: number; closed: number }> {
  let updated = 0;
  let closed = 0;
  for (const v of list) {
    if (v.status !== "active") continue;
    const mine = v.challenger_id === me ? "challenger_progress" : v.opponent_id === me ? "opponent_progress" : null;
    if (!mine) continue;
    const score = versusProgress(v, sessions, lib, now);
    if (score > v[mine]) {
      const { error } = await client.from("versus_challenges").update({ [mine]: score }).eq("id", v.id);
      if (!error) updated++;
    }
    if (v.ends_at && now.getTime() >= Date.parse(v.ends_at)) {
      const { error } = await client.rpc("close_versus", { versus_id: v.id });
      if (!error) closed++;
    }
  }
  return { updated, closed };
}

/** Inserts a versus with its rules; without the `rules` column yet, without them. */
export async function insertVersus(client: Client, row: Record<string, unknown>): Promise<PgError | null> {
  const first = await client.from("versus_challenges").insert(row);
  if (!first.error || !("rules" in row) || !missingSchema(first.error)) return first.error ?? null;
  const rest = { ...row };
  delete rest.rules;
  const second = await client.from("versus_challenges").insert(rest);
  return second.error ?? null;
}

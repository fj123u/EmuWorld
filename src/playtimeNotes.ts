// Per-game notes in the cloud live in `playtime_notes`, readable only by
// their owner (scratch/supabase_playtime_notes_private.sql). They used to be
// a column of `playtime_games`, which public profiles expose to everyone.
import { supabase } from "./supabase";

type Db = typeof supabase;

export interface NotedGame {
  console: string;
  name: string;
  notes?: string | null;
}

export const noteKey = (console: string, name: string) => `${console}::${name}`;

const hasText = (notes: string | null | undefined): notes is string => !!notes && notes.trim() !== "";

/** The signed-in player's notes, by `console::name`. Empty when the table is unreachable. */
export async function fetchCloudNotes(userId: string, db: Db = supabase): Promise<Record<string, string>> {
  const { data, error } = await db.from("playtime_notes").select("console, name, notes").eq("user_id", userId);
  if (error) {
    console.warn("[EmuWorld] playtime_notes unavailable:", error.message);
    return {};
  }
  const out: Record<string, string> = {};
  for (const row of (data ?? []) as { console: string; name: string; notes: string }[]) {
    out[noteKey(row.console, row.name)] = row.notes;
  }
  return out;
}

/**
 * Makes the cloud notes match this machine: new or edited notes are saved,
 * a note cleared here is deleted there. Games this machine doesn't know
 * (played on another PC) are left alone.
 */
export async function syncCloudNotes(
  userId: string,
  games: Record<string, NotedGame>,
  db: Db = supabase,
): Promise<{ saved: number; deleted: number }> {
  const cloud = await fetchCloudNotes(userId, db);
  const toSave = Object.values(games)
    .filter((g) => hasText(g.notes) && cloud[noteKey(g.console, g.name)] !== g.notes)
    .map((g) => ({ user_id: userId, console: g.console, name: g.name, notes: (g.notes as string).slice(0, 5000), updated_at: new Date().toISOString() }));
  const toDelete = Object.values(games).filter((g) => !hasText(g.notes) && noteKey(g.console, g.name) in cloud);

  if (toSave.length > 0) {
    const { error } = await db.from("playtime_notes").upsert(toSave, { onConflict: "user_id,console,name" });
    if (error) throw new Error(error.message);
  }
  for (const g of toDelete) {
    const { error } = await db.from("playtime_notes").delete().eq("user_id", userId).eq("console", g.console).eq("name", g.name);
    if (error) throw new Error(error.message);
  }
  return { saved: toSave.length, deleted: toDelete.length };
}

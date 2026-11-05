// Sharing RetroArch save states with friends through Supabase: the file goes
// to the private `savestates` bucket (<sender id>/<uuid>.state) and a row in
// `shared_savestates` tells the friend about it. Schema and access rules:
// scratch/supabase_savestates_sharing.sql.
import { invoke } from "@tauri-apps/api/core";
import { supabase } from "./supabase";
import type { SaveState } from "./SaveStatesModal";

export const SHARED_STATES_BUCKET = "savestates";

/** The Supabase client; tests pass a fake one. */
type Db = typeof supabase;

export interface SharedSaveState {
  id: string;
  sender_id: string;
  recipient_id: string;
  console: string;
  game_name: string;
  core: string | null;
  slot: number | null;
  label: string | null;
  storage_path: string;
  size_bytes: number;
  thumbnail: string | null;
  created_at: string;
  /** Sender's username, filled in by `listReceivedStates`. */
  sender_name?: string | null;
}

/** Title without region/revision tags, for matching the same game across two libraries. */
export function normalizeGameTitle(name: string): string {
  return name
    .replace(/\([^)]*\)|\[[^\]]*\]/g, " ")
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

/** Whether a shared state is for this game of the library. */
export function isForGame(shared: Pick<SharedSaveState, "console" | "game_name">, rom: { console: string; name: string }): boolean {
  return shared.console === rom.console && normalizeGameTitle(shared.game_name) === normalizeGameTitle(rom.name);
}

/** Small JPEG of a state thumbnail, light enough to store in the row. */
export function makeShareThumbnail(dataUrl: string | null, width = 192): Promise<string | null> {
  if (!dataUrl) return Promise.resolve(null);
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      try {
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = Math.max(1, Math.round((img.height / img.width) * width));
        const ctx = canvas.getContext("2d");
        if (!ctx) return resolve(null);
        ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
        const jpeg = canvas.toDataURL("image/jpeg", 0.7);
        resolve(jpeg.startsWith("data:image/jpeg;base64,") && jpeg.length <= 60_000 ? jpeg : null);
      } catch {
        resolve(null);
      }
    };
    img.onerror = () => resolve(null);
    img.src = dataUrl;
  });
}

export async function shareSaveState(params: {
  senderId: string;
  recipientId: string;
  state: SaveState;
  rom: { name: string; console: string };
}, db: Db = supabase): Promise<void> {
  const { senderId, recipientId, state, rom } = params;
  const bytes = await invoke<ArrayBuffer>("read_save_state", { path: state.path });
  const storagePath = `${senderId}/${crypto.randomUUID()}.state`;
  const upload = await db.storage
    .from(SHARED_STATES_BUCKET)
    .upload(storagePath, new Blob([bytes]), { contentType: "application/octet-stream", upsert: false });
  if (upload.error) throw new Error(upload.error.message);

  const { error } = await db.from("shared_savestates").insert({
    sender_id: senderId,
    recipient_id: recipientId,
    console: rom.console,
    game_name: rom.name,
    core: state.core,
    slot: state.slot,
    label: state.label,
    storage_path: storagePath,
    size_bytes: bytes.byteLength,
    thumbnail: await makeShareThumbnail(state.thumbnail),
  });
  if (error) {
    // Don't leave an orphan file nobody can see.
    await db.storage.from(SHARED_STATES_BUCKET).remove([storagePath]);
    throw new Error(error.message);
  }
}

export async function listReceivedStates(userId: string, db: Db = supabase): Promise<SharedSaveState[]> {
  const { data, error } = await db
    .from("shared_savestates")
    .select("*")
    .eq("recipient_id", userId)
    .order("created_at", { ascending: false });
  if (error) throw new Error(error.message);
  const rows = (data ?? []) as SharedSaveState[];
  const senderIds = [...new Set(rows.map((r) => r.sender_id))];
  if (senderIds.length === 0) return rows;
  const { data: profiles } = await db.from("profiles").select("id, username").in("id", senderIds);
  const names = new Map(((profiles ?? []) as { id: string; username: string | null }[]).map((p) => [p.id, p.username]));
  return rows.map((r) => ({ ...r, sender_name: names.get(r.sender_id) ?? null }));
}

/** Removes a received state from the cloud (after import, or when declined). */
export async function dismissReceivedState(shared: SharedSaveState, db: Db = supabase): Promise<void> {
  // The file first: the storage rule lets the recipient delete it only while the row exists.
  await db.storage.from(SHARED_STATES_BUCKET).remove([shared.storage_path]);
  const { error } = await db.from("shared_savestates").delete().eq("id", shared.id);
  if (error) throw new Error(error.message);
}

/** Downloads a received state into a new slot of `romPath`, then clears it from the cloud. */
export async function importReceivedState(shared: SharedSaveState, romPath: string, label: string, db: Db = supabase): Promise<SaveState> {
  const { data, error } = await db.storage.from(SHARED_STATES_BUCKET).createSignedUrl(shared.storage_path, 300);
  if (error || !data?.signedUrl) throw new Error(error?.message ?? "signed URL");
  const imported = await invoke<SaveState>("import_save_state", { romPath, core: shared.core, url: data.signedUrl });
  await invoke("rename_save_state", { path: imported.path, label }).catch(() => {});
  await dismissReceivedState(shared, db);
  return imported;
}

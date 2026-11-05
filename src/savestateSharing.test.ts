import { describe, expect, it } from "vitest";
import { mockBackend } from "./test/tauriMock";
import {
  dismissReceivedState,
  importReceivedState,
  isForGame,
  listReceivedStates,
  normalizeGameTitle,
  shareSaveState,
  type SharedSaveState,
} from "./savestateSharing";
import type { SaveState } from "./SaveStatesModal";

/** Minimal Supabase double that records what the sharing code does, in order. */
function fakeDb(opts: { insertError?: string; rows?: SharedSaveState[] } = {}) {
  const log: string[] = [];
  let inserted: Record<string, unknown> | null = null;
  const db = {
    storage: {
      from: (bucket: string) => ({
        upload: async (path: string, body: Blob) => {
          log.push(`upload ${bucket}/${path} (${body.size} B)`);
          return { error: null };
        },
        remove: async (paths: string[]) => {
          log.push(`remove ${bucket}/${paths.join(",")}`);
          return { error: null };
        },
        createSignedUrl: async (path: string) => ({
          data: { signedUrl: `https://proj.supabase.co/storage/v1/object/sign/${bucket}/${path}?token=t` },
          error: null,
        }),
      }),
    },
    from: (table: string) => ({
      insert: async (row: Record<string, unknown>) => {
        log.push(`insert ${table}`);
        inserted = row;
        return { error: opts.insertError ? { message: opts.insertError } : null };
      },
      delete: () => ({
        eq: async (_col: string, id: string) => {
          log.push(`delete ${table} ${id}`);
          return { error: null };
        },
      }),
      select: () => ({
        eq: () => ({ order: async () => ({ data: opts.rows ?? [], error: null }) }),
        in: async () => ({ data: [{ id: "friend-1", username: "Léo" }], error: null }),
      }),
    }),
  };
  return { db: db as never, log, inserted: () => inserted };
}

const state: SaveState = {
  path: "C:\\RA\\states\\Snes9x\\Zelda.state2", file_name: "Zelda.state2", slot: 2, auto: false,
  folder: "Snes9x", core: "Snes9x", modified: null, size: 3, thumbnail: null, label: "Avant Ganon",
};
const shared: SharedSaveState = {
  id: "share-1", sender_id: "friend-1", recipient_id: "me", console: "Super Nintendo",
  game_name: "Zelda (Europe)", core: "Snes9x", slot: 2, label: "Avant Ganon",
  storage_path: "friend-1/abc.state", size_bytes: 3, thumbnail: null, created_at: "2026-11-05T10:00:00Z",
};

describe("matching a shared state to the library", () => {
  it("ignores region tags, case and accents but not the console", () => {
    expect(normalizeGameTitle("Pokémon Émeraude (France) [!]")).toBe("pokemon emeraude");
    expect(isForGame(shared, { console: "Super Nintendo", name: "Zelda (USA)" })).toBe(true);
    expect(isForGame(shared, { console: "NES", name: "Zelda (Europe)" })).toBe(false);
    expect(isForGame(shared, { console: "Super Nintendo", name: "Zelda II" })).toBe(false);
  });
});

describe("sharing a save state", () => {
  it("uploads the file to the sender's folder, then records the share", async () => {
    const backend = mockBackend({ read_save_state: () => new Uint8Array([1, 2, 3]).buffer });
    const { db, log, inserted } = fakeDb();
    await shareSaveState({ senderId: "me", recipientId: "friend-1", state, rom: { name: "Zelda (Europe)", console: "Super Nintendo" } }, db);

    expect(backend.calls.find((c) => c.cmd === "read_save_state")?.args).toEqual({ path: state.path });
    expect(log[0]).toMatch(/^upload savestates\/me\/[0-9a-f-]{36}\.state \(3 B\)$/);
    expect(log[1]).toBe("insert shared_savestates");
    expect(inserted()).toMatchObject({
      sender_id: "me", recipient_id: "friend-1", console: "Super Nintendo", game_name: "Zelda (Europe)",
      core: "Snes9x", slot: 2, label: "Avant Ganon", size_bytes: 3, thumbnail: null,
    });
    expect(String(inserted()!.storage_path)).toBe(log[0].split(" ")[1].replace("savestates/", ""));
  });

  it("removes the uploaded file when the share is refused (not friends, table missing…)", async () => {
    mockBackend({ read_save_state: () => new Uint8Array([1]).buffer });
    const { db, log } = fakeDb({ insertError: "new row violates row-level security policy" });
    await expect(
      shareSaveState({ senderId: "me", recipientId: "stranger", state, rom: { name: "Zelda", console: "Super Nintendo" } }, db),
    ).rejects.toThrow(/row-level security/);
    expect(log.map((l) => l.split(" ")[0])).toEqual(["upload", "insert", "remove"]);
  });
});

describe("receiving a save state", () => {
  it("lists received states with the sender's name", async () => {
    const { db } = fakeDb({ rows: [shared] });
    const rows = await listReceivedStates("me", db);
    expect(rows).toEqual([{ ...shared, sender_name: "Léo" }]);
  });

  it("imports through a signed link, names it, then clears it from the cloud", async () => {
    const imported = { ...state, path: "C:\\RA\\states\\Snes9x\\Zelda (USA).state1", slot: 1, label: null };
    const backend = mockBackend({ import_save_state: () => imported });
    const { db, log } = fakeDb();
    await importReceivedState(shared, "D:\\Roms\\SNES\\Zelda (USA).sfc", "Avant Ganon · Léo", db);

    expect(backend.calls.find((c) => c.cmd === "import_save_state")?.args).toEqual({
      romPath: "D:\\Roms\\SNES\\Zelda (USA).sfc",
      core: "Snes9x",
      url: "https://proj.supabase.co/storage/v1/object/sign/savestates/friend-1/abc.state?token=t",
    });
    expect(backend.calls.find((c) => c.cmd === "rename_save_state")?.args).toEqual({ path: imported.path, label: "Avant Ganon · Léo" });
    expect(log).toEqual(["remove savestates/friend-1/abc.state", "delete shared_savestates share-1"]);
  });

  it("keeps the shared state when the import fails, so it can be retried", async () => {
    mockBackend({ import_save_state: () => { throw "RetroArch n'est pas installé"; } });
    const { db, log } = fakeDb();
    await expect(importReceivedState(shared, "D:\\Roms\\SNES\\Zelda.sfc", "x", db)).rejects.toBe("RetroArch n'est pas installé");
    expect(log).toEqual([]);
  });

  it("deletes the file before the row (the storage rule needs the row)", async () => {
    const { db, log } = fakeDb();
    await dismissReceivedState(shared, db);
    expect(log).toEqual(["remove savestates/friend-1/abc.state", "delete shared_savestates share-1"]);
  });
});

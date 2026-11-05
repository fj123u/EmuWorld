import { describe, expect, it } from "vitest";
import { fetchCloudNotes, syncCloudNotes } from "./playtimeNotes";
import { playtimeGameRow } from "./App";

/** playtime_notes double: `rows` is the cloud table, every write is recorded. */
function fakeDb(rows: { console: string; name: string; notes: string }[]) {
  const writes: string[] = [];
  const db = {
    from: () => ({
      select: () => ({ eq: async () => ({ data: rows, error: null }) }),
      upsert: async (batch: { name: string; notes: string }[]) => {
        writes.push(...batch.map((r) => `save ${r.name}=${r.notes}`));
        return { error: null };
      },
      delete: () => {
        const filters: string[] = [];
        const chain = {
          eq: (col: string, v: string) => {
            filters.push(`${col}=${v}`);
            if (filters.length === 3) {
              writes.push(`delete ${filters[2].replace("name=", "")}`);
              return Promise.resolve({ error: null });
            }
            return chain;
          },
        };
        return chain;
      },
    }),
  };
  return { db: db as never, writes };
}

const game = (name: string, notes?: string) => ({ console: "NES", name, notes });

describe("private game notes", () => {
  it("never puts notes in the public playtime row", () => {
    const row = playtimeGameRow("me", {
      console: "NES", name: "Zelda", seconds: 1, launches: 1, last_played: null, first_played: null,
      favorite: false, last_emulator_id: null, notes: "code du coffre : 4-2-1",
    });
    expect(row.notes).toBeNull();
  });

  it("reads the player's notes by game", async () => {
    const { db } = fakeDb([{ console: "NES", name: "Zelda", notes: "donjon 3" }]);
    expect(await fetchCloudNotes("me", db)).toEqual({ "NES::Zelda": "donjon 3" });
  });

  it("saves new and edited notes, deletes cleared ones, leaves other PCs' games alone", async () => {
    const { db, writes } = fakeDb([
      { console: "NES", name: "Zelda", notes: "donjon 3" },
      { console: "NES", name: "Metroid", notes: "same" },
      { console: "NES", name: "Mario", notes: "to clear" },
      { console: "NES", name: "OtherPC", notes: "kept" },
    ]);
    const result = await syncCloudNotes("me", {
      "NES::Zelda": game("Zelda", "donjon 4"),
      "NES::Metroid": game("Metroid", "same"),
      "NES::Mario": game("Mario", ""),
      "NES::Kirby": game("Kirby", "nouveau"),
      "NES::Tetris": game("Tetris"),
    }, db);
    expect(writes.sort()).toEqual(["delete Mario", "save Kirby=nouveau", "save Zelda=donjon 4"].sort());
    expect(result).toEqual({ saved: 2, deleted: 1 });
  });

  it("does nothing when the notes are already in sync", async () => {
    const { db, writes } = fakeDb([{ console: "NES", name: "Zelda", notes: "donjon 3" }]);
    await syncCloudNotes("me", { "NES::Zelda": game("Zelda", "donjon 3") }, db);
    expect(writes).toEqual([]);
  });
});

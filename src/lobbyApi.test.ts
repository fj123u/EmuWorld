import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { acceptLobbyInvite, addLobbyMessage, cleanLobbyMessage, LOBBY_CHAT_HISTORY, LOBBY_MESSAGE_MAX, LobbyInviteGone } from "./lobbyApi";

/** Supabase double: answers reads from `rows`, records any write. */
function fakeDb(rows: { lobby_members?: unknown; lobbies?: unknown }) {
  const writes: string[] = [];
  const query = (table: "lobby_members" | "lobbies") => {
    const chain = {
      select: () => chain,
      eq: () => chain,
      maybeSingle: async () => ({ data: rows[table] ?? null, error: null }),
    };
    return chain;
  };
  const db = {
    from: (table: "lobby_members" | "lobbies") => ({
      ...query(table),
      insert: () => { writes.push(`insert ${table}`); return query(table); },
      upsert: () => { writes.push(`upsert ${table}`); return query(table); },
    }),
  };
  return { db: db as never, writes };
}

describe("accepting a lobby invitation", () => {
  it("reads the invitation the host created, without writing anything", async () => {
    const lobby = { id: "l1", host_id: "host", game_name: "Zelda" };
    const { db, writes } = fakeDb({ lobby_members: { lobby_id: "l1" }, lobbies: lobby });
    await expect(acceptLobbyInvite("l1", "friend", db)).resolves.toEqual(lobby);
    // The access rules refuse a player adding themselves (the old upsert).
    expect(writes).toEqual([]);
  });

  it("says when the invitation or the lobby is gone", async () => {
    await expect(acceptLobbyInvite("l1", "friend", fakeDb({}).db)).rejects.toBeInstanceOf(LobbyInviteGone);
    await expect(acceptLobbyInvite("l1", "friend", fakeDb({ lobby_members: { lobby_id: "l1" } }).db)).rejects.toBeInstanceOf(LobbyInviteGone);
  });
});

describe("lobby access rules (scratch/supabase_lobbies_rls_v2.sql)", () => {
  const sql = readFileSync(join(__dirname, "..", "scratch", "supabase_lobbies_rls_v2.sql"), "utf8");
  const policy = (name: string) => {
    const start = sql.indexOf(`CREATE POLICY "${name}"`);
    expect(start, `policy "${name}"`).toBeGreaterThanOrEqual(0);
    return sql.slice(start, sql.indexOf(";", start));
  };

  it("lets only the host add members, and only friends as guests", () => {
    const insert = policy("Host adds itself or invites a friend");
    expect(insert).toContain("FOR INSERT");
    expect(insert).toContain("private.is_lobby_host(lobby_id, auth.uid())");
    expect(insert).toContain("private.are_friends(auth.uid(), user_id)");
  });

  it("hides lobbies from people who are not in them", () => {
    expect(policy("Host and members view the lobby")).toContain("host_id = auth.uid() OR private.is_lobby_member(id, auth.uid())");
  });

  it("lets members change nothing but their ready flag", () => {
    expect(sql).toContain("REVOKE UPDATE ON public.lobby_members FROM authenticated, anon;");
    expect(sql).toContain("GRANT UPDATE (is_ready) ON public.lobby_members TO authenticated;");
  });

  it("keeps its helper functions out of the public API", () => {
    // In "public", Supabase lets anyone call them through /rest/v1/rpc.
    expect(sql).not.toMatch(/CREATE OR REPLACE FUNCTION public\./);
    expect(sql).toContain("REVOKE ALL ON ALL FUNCTIONS IN SCHEMA private FROM PUBLIC, anon;");
    expect(sql).toContain("DROP FUNCTION IF EXISTS public.are_friends(uuid, uuid);");
  });

  it("applies only to signed-in accounts (anonymous reads get an empty list)", () => {
    const policies = sql.match(/CREATE POLICY "[^"]+" ON public\.\w+\s+FOR \w+[^\r\n]*/g) ?? [];
    expect(policies).toHaveLength(8);
    for (const p of policies) expect(p).toContain("TO authenticated");
  });

  it("drops every older policy and keeps the host from changing hands", () => {
    expect(sql).toMatch(/FOR pol IN[\s\S]*tablename IN \('lobbies', 'lobby_members'\)[\s\S]*DROP POLICY/);
    expect(policy("Host updates the lobby")).toContain("WITH CHECK (host_id = auth.uid())");
  });
});

describe("lobby chat messages", () => {
  const m = (id: string, minute: number) => ({ id, lobby_id: "l1", user_id: "u", content: id, created_at: `2026-11-11T20:${String(minute).padStart(2, "0")}:00Z` });

  it("accepts 1 to 500 characters once trimmed", () => {
    expect(cleanLobbyMessage("  gg  ")).toBe("gg");
    expect(cleanLobbyMessage("   ")).toBeNull();
    expect(cleanLobbyMessage("é".repeat(LOBBY_MESSAGE_MAX))).toBe("é".repeat(LOBBY_MESSAGE_MAX));
    expect(cleanLobbyMessage("a".repeat(LOBBY_MESSAGE_MAX + 1))).toBeNull();
    // Counted in characters like Postgres, not UTF-16 units: 500 emoji fit.
    expect(cleanLobbyMessage("🎮".repeat(LOBBY_MESSAGE_MAX))).not.toBeNull();
  });

  it("adds each message once, in time order, keeping the latest ones", () => {
    let list = addLobbyMessage([], m("b", 2));
    list = addLobbyMessage(list, m("a", 1));
    list = addLobbyMessage(list, m("b", 2));
    expect(list.map((x) => x.id)).toEqual(["a", "b"]);
    let many: ReturnType<typeof m>[] = [];
    for (let i = 0; i < LOBBY_CHAT_HISTORY + 5; i++) many = addLobbyMessage(many, { ...m(`x${i}`, 0), created_at: new Date(Date.UTC(2026, 10, 11, 0, 0, i)).toISOString() });
    expect(many).toHaveLength(LOBBY_CHAT_HISTORY);
    expect(many[0].id).toBe("x5");
  });

  it("keeps the database rules of the chat in the repository", () => {
    const sql = readFileSync(join(__dirname, "..", "scratch", "supabase_lobby_chat.sql"), "utf8");
    expect(sql).toMatch(/ENABLE ROW LEVEL SECURITY/);
    expect(sql).toMatch(/BETWEEN 1 AND 500/);
    expect(sql).toMatch(/REVOKE ALL ON public\.lobby_messages FROM anon, authenticated/);
  });
});

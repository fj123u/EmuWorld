import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { acceptLobbyInvite, LobbyInviteGone } from "./lobbyApi";

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

  it("drops every older policy and keeps the host from changing hands", () => {
    expect(sql).toMatch(/FOR pol IN[\s\S]*tablename IN \('lobbies', 'lobby_members'\)[\s\S]*DROP POLICY/);
    expect(policy("Host updates the lobby")).toContain("WITH CHECK (host_id = auth.uid())");
  });
});

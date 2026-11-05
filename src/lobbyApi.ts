// Supabase calls of the lobby that depend on its access rules
// (scratch/supabase_lobbies_rls_v2.sql): only the host adds members (itself,
// or friends as invitations), members only change their own `is_ready`.
import { supabase } from "./supabase";

type Db = typeof supabase;

export class LobbyInviteGone extends Error {
  constructor() {
    super("invitation no longer exists");
  }
}

/**
 * Accepting an invitation: the host already created this player's row, so
 * there is nothing to insert (the rules refuse a player adding themselves to
 * a lobby). Throws `LobbyInviteGone` when the invitation or the lobby is gone.
 */
export async function acceptLobbyInvite<L>(lobbyId: string, userId: string, db: Db = supabase): Promise<L> {
  const { data: membership } = await db
    .from("lobby_members")
    .select("lobby_id")
    .eq("lobby_id", lobbyId)
    .eq("user_id", userId)
    .maybeSingle();
  if (!membership) throw new LobbyInviteGone();
  const { data: lobby } = await db.from("lobbies").select("*").eq("id", lobbyId).maybeSingle();
  if (!lobby) throw new LobbyInviteGone();
  return lobby as L;
}

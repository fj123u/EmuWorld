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

// ─── Lobby chat (scratch/supabase_lobby_chat.sql) ───
// Only the host and the members of a lobby read and write its messages;
// they are deleted with the lobby.

export interface LobbyMessage {
  id: string;
  lobby_id: string;
  user_id: string;
  content: string;
  created_at: string;
}

/** Same limit as the table's CHECK (characters, not UTF-16 units). */
export const LOBBY_MESSAGE_MAX = 500;
/** Messages kept on screen (and loaded when opening the lobby). */
export const LOBBY_CHAT_HISTORY = 100;

/** The message to send, or null when empty or too long. */
export function cleanLobbyMessage(raw: string): string | null {
  const value = raw.trim();
  const length = [...value].length;
  return length >= 1 && length <= LOBBY_MESSAGE_MAX ? value : null;
}

/** Adds a message once (own messages also come back through realtime), oldest first. */
export function addLobbyMessage(list: LobbyMessage[], msg: LobbyMessage): LobbyMessage[] {
  if (list.some((m) => m.id === msg.id)) return list;
  const next = [...list, msg].sort((a, b) => a.created_at.localeCompare(b.created_at));
  return next.slice(-LOBBY_CHAT_HISTORY);
}

/** What the chat needs from the server (a double in tests). */
export interface LobbyChatApi {
  load: (lobbyId: string) => Promise<LobbyMessage[]>;
  send: (lobbyId: string, userId: string, content: string) => Promise<LobbyMessage>;
  /** Calls `onMessage` for each new message; returns the unsubscribe function. */
  subscribe: (lobbyId: string, onMessage: (msg: LobbyMessage) => void) => () => void;
}

export function supabaseLobbyChat(db: Db = supabase): LobbyChatApi {
  return {
    async load(lobbyId) {
      const { data, error } = await db
        .from("lobby_messages")
        .select("*")
        .eq("lobby_id", lobbyId)
        .order("created_at", { ascending: false })
        .limit(LOBBY_CHAT_HISTORY);
      if (error) throw new Error(error.message);
      return ((data || []) as LobbyMessage[]).reverse();
    },
    async send(lobbyId, userId, content) {
      const { data, error } = await db
        .from("lobby_messages")
        .insert({ lobby_id: lobbyId, user_id: userId, content })
        .select()
        .single();
      if (error || !data) throw new Error(error?.message ?? "no row");
      return data as LobbyMessage;
    },
    subscribe(lobbyId, onMessage) {
      const channel = db
        .channel(`lobby-chat-${lobbyId}`)
        .on("postgres_changes", { event: "INSERT", schema: "public", table: "lobby_messages", filter: `lobby_id=eq.${lobbyId}` }, (payload) => {
          onMessage(payload.new as LobbyMessage);
        })
        .subscribe();
      return () => { db.removeChannel(channel); };
    },
  };
}

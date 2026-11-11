// Chat of the lobby the player is in, shown in the lobby panel: history,
// new messages live (Supabase Realtime), and a field to write. Server side:
// lobbyApi.ts (supabaseLobbyChat) and scratch/supabase_lobby_chat.sql.
import { useEffect, useRef, useState } from "react";
import { Send } from "lucide-react";
import { addLobbyMessage, cleanLobbyMessage, LOBBY_MESSAGE_MAX, type LobbyChatApi, type LobbyMessage } from "./lobbyApi";

interface Props {
  lobbyId: string;
  userId: string;
  /** user_id → username, for the lobby's members. */
  names: Record<string, string>;
  api: LobbyChatApi;
  t: (key: string, params?: Record<string, string | number>) => string;
  onLog: (level: "INFO" | "WARN" | "ERROR", message: string) => void;
}

export function LobbyChat({ lobbyId, userId, names, api, t, onLog }: Props) {
  const [messages, setMessages] = useState<LobbyMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let alive = true;
    setMessages([]);
    setError(null);
    const unsubscribe = api.subscribe(lobbyId, (msg) => {
      if (alive) setMessages((prev) => addLobbyMessage(prev, msg));
    });
    api.load(lobbyId)
      .then((history) => {
        if (!alive) return;
        setMessages((prev) => history.reduce(addLobbyMessage, prev));
        onLog("INFO", `chat du lobby ${lobbyId} ouvert (${history.length} messages)`);
      })
      .catch((e) => {
        if (!alive) return;
        setError(t("lobby.chatUnavailable"));
        onLog("ERROR", `chat du lobby ${lobbyId} indisponible: ${String(e)}`);
      });
    return () => { alive = false; unsubscribe(); };
    // onLog/t change identity on re-render; the subscription follows the lobby only.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lobbyId, api]);

  useEffect(() => {
    const list = listRef.current;
    if (list) list.scrollTop = list.scrollHeight;
  }, [messages]);

  const send = async () => {
    const content = cleanLobbyMessage(draft);
    if (!content || sending) return;
    setSending(true);
    setError(null);
    try {
      const msg = await api.send(lobbyId, userId, content);
      setMessages((prev) => addLobbyMessage(prev, msg));
      setDraft("");
    } catch (e) {
      setError(t("lobby.chatFailed", { error: String(e) }));
      onLog("ERROR", `message non envoyé dans le lobby ${lobbyId}: ${String(e)}`);
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="lobby-chat" role="log" aria-label={t("lobby.chatTitle")}>
      <div className="lobby-chat__messages" ref={listRef}>
        {messages.length === 0 && !error && <p className="lobby-chat__empty">{t("lobby.chatEmpty")}</p>}
        {messages.map((m) => (
          <p key={m.id} className={`lobby-chat__message ${m.user_id === userId ? "lobby-chat__message--mine" : ""}`}>
            <strong>{m.user_id === userId ? t("lobby.you") : names[m.user_id] || t("lobby.player")}</strong> {m.content}
          </p>
        ))}
      </div>
      {error && <p className="lobby-chat__error">{error}</p>}
      <form className="lobby-chat__form" onSubmit={(e) => { e.preventDefault(); send(); }}>
        <input
          className="settings__field-input gamepad-nav-item"
          aria-label={t("lobby.chatPlaceholder")}
          placeholder={t("lobby.chatPlaceholder")}
          value={draft}
          maxLength={LOBBY_MESSAGE_MAX}
          onChange={(e) => setDraft(e.target.value)}
        />
        <button type="submit" className="btn btn--primary btn--sm gamepad-nav-item" disabled={sending || !cleanLobbyMessage(draft)} aria-label={t("lobby.chatSend")}>
          <Send size={12} />
        </button>
      </form>
    </div>
  );
}

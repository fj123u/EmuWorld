// Floating panel of the lobby the player is in: game, members and their
// state, invite, ready, launch (host) / join (friends), leave.
import { motion } from "framer-motion";
import { Download, Play, RefreshCw, Wifi } from "lucide-react";
import { everyoneReady } from "./netplay";

export interface LobbyMemberView {
  user_id: string;
  is_ready: boolean;
  profile?: { username: string | null } | null;
}

export interface LobbyView {
  id: string;
  host_id: string;
  game_name: string;
  game_console: string;
  status: string;
  members?: LobbyMemberView[];
}

interface Props {
  lobby: LobbyView;
  userId: string | null;
  /** Accepted friends who can be invited. */
  friends: { id: string; username: string }[];
  /** The player's copy of the game (possibly another region), if any. */
  myRomName: string | null;
  /** What the panel is waiting for, to disable buttons and show progress. */
  busy: "launching" | "joining" | null;
  t: (key: string, params?: Record<string, string | number>) => string;
  onInvite: (friendId: string) => void;
  onReady: () => void;
  onLaunch: () => void;
  onJoin: () => void;
  onLeave: () => void;
  onGetGame: () => void;
}

export function LobbyPanel({ lobby, userId, friends, myRomName, busy, t, onInvite, onReady, onLaunch, onJoin, onLeave, onGetGame }: Props) {
  const members = lobby.members ?? [];
  const isHost = lobby.host_id === userId;
  const me = members.find((m) => m.user_id === userId);
  const playing = lobby.status === "playing";
  const invitable = friends.filter((f) => !members.some((m) => m.user_id === f.id));

  return (
    <motion.div className="lobby-panel" role="region" aria-label={t("lobby.title")} initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }}>
      <div className="lobby-panel__header">
        <h3>{t("lobby.title")}</h3>
        <button className="btn btn--danger btn--sm gamepad-nav-item" onClick={onLeave}>
          {isHost ? t("lobby.dissolve") : t("lobby.leave")}
        </button>
      </div>
      <div className="lobby-panel__game">
        <strong>{lobby.game_name}</strong>
        <span>{lobby.game_console}</span>
      </div>
      {myRomName && myRomName !== lobby.game_name && (
        <div className="lobby-panel__warning">{t("lobby.otherVersion", { mine: myRomName })}</div>
      )}
      <div className="lobby-panel__members">
        {members.map((m) => (
          <div key={m.user_id} className={`lobby-panel__member ${m.is_ready ? "lobby-panel__member--ready" : ""}`}>
            <span>
              {m.user_id === userId ? t("lobby.you") : m.profile?.username || t("lobby.player")}
              {m.user_id === lobby.host_id ? ` · ${t("lobby.host")}` : ""}
            </span>
            <span className="lobby-panel__status">{m.is_ready ? t("lobby.readyStatus") : t("lobby.waiting")}</span>
          </div>
        ))}
      </div>
      {playing && <div className="lobby-panel__live"><Wifi size={12} /> {t("lobby.playing")}</div>}
      {busy && (
        <div className="lobby-panel__live">
          <RefreshCw size={12} className="animate-spin" /> {t(busy === "launching" ? "lobby.opening" : "lobby.joining")}
        </div>
      )}
      <div className="lobby-panel__actions">
        {isHost && !playing && invitable.length > 0 && (
          <select
            className="lobby-panel__invite-select gamepad-nav-item"
            aria-label={t("lobby.inviteFriend")}
            value=""
            onChange={(e) => { if (e.target.value) onInvite(e.target.value); }}
          >
            <option value="">{t("lobby.inviteFriend")}</option>
            {invitable.map((f) => <option key={f.id} value={f.id}>{f.username}</option>)}
          </select>
        )}
        {!me?.is_ready && !playing && (
          myRomName ? (
            <button className="btn btn--success btn--sm gamepad-nav-item" onClick={onReady}>{t("lobby.ready")}</button>
          ) : (
            <button className="btn btn--primary btn--sm gamepad-nav-item" onClick={onGetGame}>
              <Download size={12} /> {t("lobby.downloadGame")}
            </button>
          )
        )}
        {isHost && !playing && everyoneReady(members) && (
          <button className="btn btn--success gamepad-nav-item" disabled={busy !== null} onClick={onLaunch}>
            <Play size={14} /> {t("lobby.launch")}
          </button>
        )}
        {isHost && !playing && !everyoneReady(members) && members.length < 2 && (
          <span className="lobby-panel__hint">{t("lobby.needFriend")}</span>
        )}
        {!isHost && playing && myRomName && (
          <button className="btn btn--success gamepad-nav-item" disabled={busy !== null} onClick={onJoin}>
            <Play size={14} /> {t("lobby.joinGame")}
          </button>
        )}
      </div>
    </motion.div>
  );
}

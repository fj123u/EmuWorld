// Floating panel of the lobby the player is in: game, members and their
// state, invite, ready, launch (host) / join (friends), leave.
import { motion } from "framer-motion";
import { useState } from "react";
import { Copy, Download, Play, RefreshCw, Send, Wifi } from "lucide-react";
import { everyoneReady, isValidHostCode } from "./netplay";

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
  /** Emulator of the game in progress ("retroarch", "dolphin"…), once known. */
  method: string | null;
  /** Code the host shared (Dolphin / Azahar), shown to friends. */
  hostCode: string | null;
  /** The host launched Dolphin/Azahar and still has to share its code. */
  awaitingHostCode: boolean;
  onPublishCode: (code: string) => void;
  t: (key: string, params?: Record<string, string | number>) => string;
  onInvite: (friendId: string) => void;
  onReady: () => void;
  onLaunch: () => void;
  onJoin: () => void;
  onLeave: () => void;
  onGetGame: () => void;
}

/** Steps to follow inside the emulator, when EmuWorld can't do it all. */
const STEPS: Record<string, { host: string; client: string }> = {
  dolphin: { host: "lobby.stepsDolphinHost", client: "lobby.stepsDolphinClient" },
  azahar: { host: "lobby.stepsAzaharHost", client: "lobby.stepsAzaharClient" },
  ppsspp: { host: "lobby.stepsPpsspp", client: "lobby.stepsPpsspp" },
};

export function LobbyPanel({ lobby, userId, friends, myRomName, busy, method, hostCode, awaitingHostCode, t, onInvite, onReady, onLaunch, onJoin, onLeave, onGetGame, onPublishCode }: Props) {
  const [codeDraft, setCodeDraft] = useState("");
  const [copied, setCopied] = useState(false);
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
      {method && STEPS[method] && (playing || awaitingHostCode) && (
        <p className="lobby-panel__steps">{t(isHost ? STEPS[method].host : STEPS[method].client)}</p>
      )}
      {isHost && awaitingHostCode && method && (
        <form
          className="lobby-panel__code"
          onSubmit={(e) => { e.preventDefault(); if (isValidHostCode(method, codeDraft)) onPublishCode(codeDraft.trim()); }}
        >
          <input
            className="settings__field-input gamepad-nav-item"
            aria-label={t(method === "dolphin" ? "lobby.codeLabelDolphin" : "lobby.codeLabelAzahar")}
            placeholder={t(method === "dolphin" ? "lobby.codeLabelDolphin" : "lobby.codeLabelAzahar")}
            value={codeDraft}
            maxLength={100}
            onChange={(e) => setCodeDraft(e.target.value)}
          />
          <button type="submit" className="btn btn--primary btn--sm gamepad-nav-item" disabled={!isValidHostCode(method, codeDraft)}>
            <Send size={12} /> {t("lobby.shareCode")}
          </button>
        </form>
      )}
      {!isHost && playing && hostCode && (
        <div className="lobby-panel__code">
          <span>{t("lobby.hostCodeTitle")}</span>
          <code className="lobby-panel__code-value">{hostCode}</code>
          <button
            className="btn btn--ghost btn--sm gamepad-nav-item"
            onClick={() => { navigator.clipboard?.writeText(hostCode).then(() => setCopied(true), () => {}); }}
          >
            <Copy size={12} /> {copied ? t("lobby.copied") : t("lobby.copy")}
          </button>
        </div>
      )}
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
        {isHost && !playing && !awaitingHostCode && everyoneReady(members) && (
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

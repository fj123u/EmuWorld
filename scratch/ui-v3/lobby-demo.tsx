// Dev-only: the Lobbies page with an open lobby (app.html?demo=lobby), to look
// at it without a signed-in account. Sample data, chat kept in memory.
import { useMemo } from "react";
import { useTranslation } from "../../src/i18n";
import { LobbyPage } from "../../src/LobbyPage";
import { LobbyPanel } from "../../src/LobbyPanel";
import { LobbyChat } from "../../src/LobbyChat";
import type { LobbyChatApi, LobbyMessage } from "../../src/lobbyApi";

const now = Date.now();
const messages: LobbyMessage[] = [
  { id: "1", lobby_id: "l1", user_id: "lena", content: "on fait la coupe Étoile ?", created_at: new Date(now - 90_000).toISOString() },
  { id: "2", lobby_id: "l1", user_id: "max", content: "go, je prends le petit kart", created_at: new Date(now - 60_000).toISOString() },
  { id: "3", lobby_id: "l1", user_id: "me", content: "j'arrive, je règle ma manette", created_at: new Date(now - 30_000).toISOString() },
];

export function LobbyDemo() {
  const { t } = useTranslation();
  const api = useMemo<LobbyChatApi>(() => ({
    load: async () => messages,
    send: async (lobbyId, userId, content) => ({ id: String(Math.random()), lobby_id: lobbyId, user_id: userId, content, created_at: new Date().toISOString() }),
    subscribe: () => () => {},
  }), []);
  const lobby = {
    id: "l1", host_id: "lena", game_name: "Kart Fiesta", game_console: "GameCube / Wii", status: "waiting",
    members: [
      { user_id: "lena", is_ready: true, profile: { username: "Léna" } },
      { user_id: "me", is_ready: false, profile: { username: "fj123u" } },
      { user_id: "max", is_ready: true, profile: { username: "Max" } },
    ],
  };
  const noop = () => {};
  return (
    <div className="app" style={{ padding: 28, overflow: "auto" }}>
      <LobbyPage
        t={t}
        signedIn
        current={{ id: "l1", game: "Kart Fiesta", console: "GameCube / Wii", host: "Léna", players: 3 }}
        invites={[{ id: "l2", game: "Robo Rumble", console: "Nintendo 64", host: "Max" }]}
        games={[]}
        panel={
          <LobbyPanel
            variant="page" lobby={lobby} userId="me" friends={[{ id: "sam", username: "Sam" }]} myRomName="Kart Fiesta"
            busy={null} method={null} hostCode={null} awaitingHostCode={false} t={t}
            chat={<LobbyChat lobbyId="l1" userId="me" names={{ lena: "Léna", max: "Max" }} api={api} t={t} onLog={noop} />}
            onInvite={noop} onReady={noop} onLaunch={noop} onJoin={noop} onLeave={noop} onGetGame={noop} onPublishCode={noop}
          />
        }
        onAccept={noop} onDecline={noop} onCreate={noop} onSignIn={noop}
      />
    </div>
  );
}

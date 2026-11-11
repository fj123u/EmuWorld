// v3 "Lobbies" page (mockup "Multijoueur", first tab of Social): the
// player's lobby and the invitations on the left, the open lobby on the
// right (LobbyPanel in its page variant, passed as `panel`). Lobbies are
// private (scratch/supabase_lobbies_rls_v2.sql): a player only sees the
// lobby they are in and the ones they are invited to.
import { useState, type CSSProperties, type ReactNode } from "react";
import { consoleHue } from "./consoleTheme";

type T = (key: string, params?: Record<string, string | number>) => string;

export interface LobbyCard {
  id: string;
  game: string;
  console: string;
  host: string;
  /** Unknown for an invitation (its members are not loaded). */
  players?: number;
}

export interface LobbyGame {
  path: string;
  name: string;
  console: string;
}

interface Props<G extends LobbyGame> {
  t: T;
  signedIn: boolean;
  /** The lobby the player is in, if any. */
  current: LobbyCard | null;
  invites: LobbyCard[];
  /** Games of the library that can be played online. */
  games: G[];
  panel: ReactNode;
  onAccept: (id: string) => void;
  onDecline: (id: string) => void;
  onCreate: (game: G) => void;
  onSignIn: () => void;
}

function hueStyle(consoleName: string): CSSProperties | undefined {
  const hue = consoleHue(consoleName);
  return hue === null ? undefined : ({ "--console-hue": hue } as CSSProperties);
}

function Card({ t, lobby, children, mine }: { t: T; lobby: LobbyCard; children?: ReactNode; mine?: boolean }) {
  return (
    <div className={`v3-card lobby-card${mine ? " lobby-card--mine" : ""}`} style={hueStyle(lobby.console)}>
      <div className="v3-cover v3-ring lobby-card__cover" aria-hidden="true" />
      <div className="lobby-card__text">
        <span className="lobby-card__game">{lobby.game}</span>
        <span className="lobby-card__meta">{t("lobbyPage.consoleHost", { console: lobby.console, host: lobby.host })}</span>
        {lobby.players !== undefined && <span className="v3-pill lobby-card__players">{t("lobbyPage.players", { count: lobby.players })}</span>}
      </div>
      {children}
    </div>
  );
}

export function LobbyPage<G extends LobbyGame>({ t, signedIn, current, invites, games, panel, onAccept, onDecline, onCreate, onSignIn }: Props<G>) {
  const [picking, setPicking] = useState(false);
  const [choice, setChoice] = useState("");
  const chosen = games.find((g) => g.path === choice);

  return (
    <div className="lobby-page">
      <section className="lobby-page__list">
        <div className="lobby-page__head">
          <h2>{t("lobbyPage.title")}</h2>
          {signedIn && !current && (
            <button className="btn btn--go gamepad-nav-item" onClick={() => setPicking((v) => !v)} aria-expanded={picking}>
              {t("lobby.createLobby")}
            </button>
          )}
        </div>

        {!signedIn && (
          <div className="v3-card lobby-page__empty">
            <p>{t("lobbyPage.signIn")}</p>
            <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={onSignIn}>{t("sidebar.loginSignUp")}</button>
          </div>
        )}

        {signedIn && picking && !current && (
          <div className="v3-card lobby-page__create">
            {games.length === 0 ? (
              <p className="lobby-page__muted">{t("lobbyPage.noOnlineGames")}</p>
            ) : (
              <>
                <select
                  className="library-sort-select gamepad-nav-item"
                  aria-label={t("lobbyPage.pickGame")}
                  value={choice}
                  onChange={(e) => setChoice(e.target.value)}
                >
                  <option value="">{t("lobbyPage.pickGame")}</option>
                  {games.map((g) => <option key={g.path} value={g.path}>{g.name} ({g.console})</option>)}
                </select>
                <button
                  className="btn btn--primary btn--sm gamepad-nav-item"
                  disabled={!chosen}
                  onClick={() => { if (chosen) { onCreate(chosen); setPicking(false); setChoice(""); } }}
                >
                  {t("lobbyPage.create")}
                </button>
              </>
            )}
          </div>
        )}

        {current && (
          <Card t={t} lobby={current} mine>
            <span className="v3-pill lobby-card__mine">{t("lobbyPage.youreIn")}</span>
          </Card>
        )}

        {invites.map((inv) => (
          <Card key={inv.id} t={t} lobby={inv}>
            <div className="lobby-card__actions">
              <button className="btn btn--primary btn--sm gamepad-nav-item" onClick={() => onAccept(inv.id)}>{t("lobbyPage.join")}</button>
              <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => onDecline(inv.id)}>{t("versus.decline")}</button>
            </div>
          </Card>
        ))}

        {signedIn && !current && invites.length === 0 && !picking && (
          <p className="lobby-page__muted">{t("lobbyPage.none")}</p>
        )}
      </section>

      <div className="lobby-page__open">
        {panel ?? (
          <section className="v3-frame lobby-page__placeholder">
            <h2>{t("lobbyPage.noLobbyTitle")}</h2>
            <p className="lobby-page__muted">{t("lobbyPage.noLobbyText")}</p>
          </section>
        )}
      </div>
    </div>
  );
}

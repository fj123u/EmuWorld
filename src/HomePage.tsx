// v3 home page (mockup "Accueil"): the game to resume in the gradient frame,
// your consoles, recently played games, then friends online, the challenge
// of the week and this month's summary. Data: src/home.ts. What the old
// "Discover" page showed (game of the day, suggestion, top games…) is passed
// as children and stays below.
import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { consoleHue } from "./consoleTheme";
import { formatPlaytime, type HomeRom } from "./home";

type T = (key: string, params?: Record<string, string | number>) => string;

/** Sets --console-hue so .v3-ring / .v3-tile / .v3-cover take the console's color. */
function hueStyle(consoleName: string): CSSProperties | undefined {
  const hue = consoleHue(consoleName);
  return hue === null ? undefined : ({ "--console-hue": hue } as CSSProperties);
}

/** A cover from the cache / cover sources, or the console color with the game's name. */
function Cover({ rom, className = "" }: { rom: HomeRom; className?: string }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    invoke<string | null>("fetch_boxart", { gameName: rom.name, console: rom.console, forceRefresh: false })
      .then((c) => { if (alive && c) setSrc(c); })
      .catch(() => {});
    return () => { alive = false; };
  }, [rom.name, rom.console]);
  return (
    <div className={`v3-cover v3-ring home-cover ${className}`} style={hueStyle(rom.console)}>
      {src ? <img src={src} alt="" /> : <span className="home-cover__name">{rom.name}</span>}
    </div>
  );
}

export interface HomeFriend {
  id: string;
  name: string;
  avatarUrl: string | null;
  playing: string | null;
}

export interface HomeChallenge {
  title: string;
  description: string;
  /** null: the player has not joined it. */
  progress: number | null;
  goal: number;
  /** "launches" counts sessions, the others are seconds. */
  goalType: string;
}

export interface HomePageProps<R extends HomeRom> {
  t: T;
  /** The game to resume (last session), or the game of the day when nothing was played. */
  resume: { rom: R; seconds: number; lastPlayed: string | null; kind: "resume" | "gameOfTheDay" } | null;
  consoles: { name: string; count: number }[];
  recent: { rom: R; seconds: number }[];
  /** null: signed out. */
  friends: HomeFriend[] | null;
  challenge: HomeChallenge | null;
  month: { seconds: number; activeDays: number; achievements: number };
  dateLocale: string;
  canLobby: (rom: R) => boolean;
  onPlay: (rom: R) => void;
  onLobby: (rom: R) => void;
  onOpenConsole: (name: string) => void;
  onSeeAllGames: () => void;
  onOpenStore: () => void;
  onOpenFriend: (friend: HomeFriend) => void;
  onOpenChallenges: () => void;
  onSignIn: () => void;
  children?: ReactNode;
}

function lastPlayedText(t: T, iso: string | null, dateLocale: string): string | null {
  if (!iso) return null;
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return null;
  const days = Math.floor((Date.now() - d.getTime()) / 86_400_000);
  if (days <= 0) return t("home.lastToday");
  if (days === 1) return t("home.lastYesterday");
  return t("home.lastOn", { date: d.toLocaleDateString(dateLocale) });
}

export function HomePage<R extends HomeRom>(p: HomePageProps<R>) {
  const { t } = p;
  const challengeText = (c: HomeChallenge, value: number) =>
    c.goalType === "launches" ? t("home.sessions", { count: value }) : formatPlaytime(value);

  return (
    <div className="home">
      <div className="home__main">
        {p.resume ? (
          <section className="v3-frame home-resume" style={hueStyle(p.resume.rom.console)}>
            <Cover rom={p.resume.rom} className="home-resume__cover" />
            <div className="home-resume__text">
              <span className="v3-pill">{t(p.resume.kind === "resume" ? "home.resume" : "discover.gameOfTheDay")}</span>
              <h1 className="home-resume__title">{p.resume.rom.name}</h1>
              <p className="home-resume__meta">
                {[
                  p.resume.rom.console,
                  p.resume.seconds > 0 ? t("home.played", { time: formatPlaytime(p.resume.seconds) }) : null,
                  lastPlayedText(t, p.resume.lastPlayed, p.dateLocale),
                ].filter(Boolean).join(" · ")}
              </p>
              <div className="home-resume__actions">
                <button className="btn btn--go gamepad-nav-item" onClick={() => p.onPlay(p.resume!.rom)}>
                  <span className="v3-key" aria-hidden="true">A</span>{t("discover.play")}
                </button>
                {p.canLobby(p.resume.rom) && (
                  <button className="btn btn--ghost gamepad-nav-item" onClick={() => p.onLobby(p.resume!.rom)}>{t("lobby.createLobby")}</button>
                )}
              </div>
            </div>
          </section>
        ) : (
          <section className="v3-frame home-resume home-resume--empty">
            <div className="home-resume__text">
              <h1 className="home-resume__title">{t("library.noGames")}</h1>
              <p className="home-resume__meta">{t("library.noGamesDesc")}</p>
              <div className="home-resume__actions">
                <button className="btn btn--go gamepad-nav-item" onClick={p.onOpenStore}>{t("discover.openStore")}</button>
              </div>
            </div>
          </section>
        )}

        {p.consoles.length > 0 && (
          <section className="home-section">
            <div className="home-section__head">
              <h2>{t("home.yourConsoles")}</h2>
              <button className="home-link gamepad-nav-item" onClick={p.onSeeAllGames}>{t("home.seeAll")}</button>
            </div>
            <div className="home-tiles">
              {p.consoles.map((c) => (
                <button key={c.name} className="v3-tile v3-ring home-tile gamepad-nav-item" style={hueStyle(c.name)} onClick={() => p.onOpenConsole(c.name)}>
                  <span className="v3-tile__name">{c.name}</span>
                  <span className="v3-pill home-tile__count">{t(c.count === 1 ? "library.gameCountOne" : "library.gameCountMany", { count: c.count })}</span>
                </button>
              ))}
            </div>
          </section>
        )}

        {p.recent.length > 0 && (
          <section className="home-section">
            <h2>{t("discover.recentlyPlayed")}</h2>
            <div className="home-covers">
              {p.recent.map(({ rom, seconds }) => (
                <button key={rom.path} className="home-game gamepad-nav-item" onClick={() => p.onPlay(rom)}>
                  <Cover rom={rom} />
                  <span className="home-game__name">{rom.name}</span>
                  <span className="home-game__meta">{rom.console} · {formatPlaytime(seconds)}</span>
                </button>
              ))}
            </div>
          </section>
        )}
      </div>

      <aside className="home__side">
        <section className="v3-card home-card">
          <h2>{t("home.friendsOnline")}</h2>
          {p.friends === null ? (
            <>
              <p className="home-card__muted">{t("friendsUi.signInPrompt")}</p>
              <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={p.onSignIn}>{t("sidebar.loginSignUp")}</button>
            </>
          ) : p.friends.length === 0 ? (
            <p className="home-card__muted">{t("home.nobodyOnline")}</p>
          ) : (
            p.friends.map((f) => (
              <button key={f.id} className="home-friend gamepad-nav-item" onClick={() => p.onOpenFriend(f)}>
                <span className="v3-avatar v3-ring">{f.avatarUrl ? <img src={f.avatarUrl} alt="" /> : f.name.slice(0, 2).toUpperCase()}</span>
                <span className="home-friend__text">
                  <span className="home-friend__name">{f.name}</span>
                  <span className="home-card__muted">{f.playing ? t("overlay.playing", { game: f.playing }) : t("overlay.online")}</span>
                </span>
              </button>
            ))
          )}
        </section>

        {p.challenge && (
          <section className="v3-card home-card">
            <span className="v3-pill">{t("home.weeklyChallenge")}</span>
            <h3 className="home-card__title">{p.challenge.title}</h3>
            <p className="home-card__muted">{p.challenge.description}</p>
            {p.challenge.progress !== null ? (
              <>
                <div className="v3-progress" role="progressbar" aria-valuemin={0} aria-valuemax={p.challenge.goal} aria-valuenow={Math.min(p.challenge.progress, p.challenge.goal)}>
                  <span style={{ width: `${Math.min(100, (p.challenge.progress / Math.max(1, p.challenge.goal)) * 100)}%` }} />
                </div>
                <p className="home-card__muted">{t("home.progressOf", { done: challengeText(p.challenge, p.challenge.progress), goal: challengeText(p.challenge, p.challenge.goal) })}</p>
              </>
            ) : (
              <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={p.onOpenChallenges}>{t("statsUi.join")}</button>
            )}
          </section>
        )}

        <section className="v3-card home-card">
          <h2>{t("home.thisMonth")}</h2>
          <div className="home-stats">
            <div><div className="home-stats__value">{formatPlaytime(p.month.seconds)}</div><div className="home-card__muted">{t("home.playtime")}</div></div>
            <div><div className="home-stats__value">{p.month.activeDays}</div><div className="home-card__muted">{t("home.daysPlayed")}</div></div>
            <div><div className="home-stats__value">{p.month.achievements}</div><div className="home-card__muted">{t("home.achievementsUnlocked")}</div></div>
          </div>
        </section>
      </aside>

      {p.children && <div className="home__more">{p.children}</div>}
    </div>
  );
}

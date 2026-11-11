import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { mockBackend } from "./test/tauriMock";
import { HomePage, type HomePageProps } from "./HomePage";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};
type Rom = { name: string; console: string; path: string };
const zelda: Rom = { name: "Zelda", console: "NES", path: "D:\\zelda.nes" };
const kart: Rom = { name: "Kart", console: "Game Boy Advance", path: "D:\\kart.gba" };

function props(over: Partial<HomePageProps<Rom>> = {}): HomePageProps<Rom> {
  return {
    t, dateLocale: "fr-FR",
    resume: { rom: zelda, seconds: 45_600, lastPlayed: new Date(Date.now() - 86_400_000).toISOString(), kind: "resume" },
    consoles: [{ name: "Game Boy Advance", count: 31 }, { name: "NES", count: 1 }],
    recent: [{ rom: kart, seconds: 2880 }],
    friends: [],
    challenge: null,
    month: { seconds: 33_120, activeDays: 6, achievements: 4 },
    canLobby: () => true,
    onPlay: vi.fn(), onLobby: vi.fn(), onOpenConsole: vi.fn(), onSeeAllGames: vi.fn(), onOpenStore: vi.fn(),
    onOpenFriend: vi.fn(), onOpenChallenges: vi.fn(), onSignIn: vi.fn(),
    ...over,
  };
}

function renderHome(over: Partial<HomePageProps<Rom>> = {}) {
  mockBackend();
  const p = props(over);
  const view = render(<HomePage {...p} />);
  return { p, ...view };
}

describe("home page", () => {
  it("offers to resume the last game, and a lobby when it can be played online", () => {
    const { p } = renderHome();
    expect(screen.getByText("Reprendre la partie")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Zelda" })).toBeTruthy();
    expect(screen.getByText("NES · 12 h 40 de jeu · dernière partie hier")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Jouer/ }));
    expect(p.onPlay).toHaveBeenCalledWith(zelda);
    fireEvent.click(screen.getByRole("button", { name: "Créer un lobby" }));
    expect(p.onLobby).toHaveBeenCalledWith(zelda);
  });

  it("hides the lobby button when the game cannot be played online", () => {
    renderHome({ canLobby: () => false });
    expect(screen.queryByRole("button", { name: "Créer un lobby" })).toBeNull();
  });

  it("shows the game of the day when nothing was played, and the store when the library is empty", () => {
    const { p, rerender } = renderHome({ resume: { rom: zelda, seconds: 0, lastPlayed: null, kind: "gameOfTheDay" } });
    expect(screen.getByText("Jeu du jour")).toBeTruthy();
    rerender(<HomePage {...p} resume={null} consoles={[]} recent={[]} />);
    fireEvent.click(screen.getByRole("button", { name: "Ouvrir le Store" }));
    expect(p.onOpenStore).toHaveBeenCalled();
  });

  it("opens a console from its tile, colored like the console", () => {
    const { p } = renderHome();
    const tile = screen.getByRole("button", { name: /Game Boy Advance\s*31 jeux/ });
    expect(tile.getAttribute("style")).toContain("--console-hue");
    fireEvent.click(tile);
    expect(p.onOpenConsole).toHaveBeenCalledWith("Game Boy Advance");
    fireEvent.click(screen.getByRole("button", { name: "Tout voir" }));
    expect(p.onSeeAllGames).toHaveBeenCalled();
  });

  it("shows each console's logo on its tile", () => {
    renderHome({ consoleLogo: (name) => <img className="console-logo" src={`logo/${name}.svg`} alt="" /> });
    const tile = screen.getByRole("button", { name: /Game Boy Advance\s*31 jeux/ });
    expect(tile.querySelector(".home-tile__logo img")?.getAttribute("src")).toBe("logo/Game Boy Advance.svg");
  });

  it("lists recently played games", () => {
    const { p } = renderHome();
    expect(screen.getByText("Game Boy Advance · 48 min")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Kart/ }));
    expect(p.onPlay).toHaveBeenCalledWith(kart);
  });

  it("shows friends online", () => {
    const lena = { id: "u1", name: "Léna", avatarUrl: null, playing: "Kart Fiesta" };
    const { p } = renderHome({ friends: [lena] });
    const button = screen.getByRole("button", { name: /Léna/ });
    expect(button.textContent).toContain("Joue à Kart Fiesta");
    fireEvent.click(button);
    expect(p.onOpenFriend).toHaveBeenCalledWith(lena);
  });

  it("asks to sign in to see friends", () => {
    const { p } = renderHome({ friends: null });
    fireEvent.click(screen.getByRole("button", { name: "Connexion / Inscription" }));
    expect(p.onSignIn).toHaveBeenCalled();
  });

  it("shows the challenge of the week with my progress, or a way to join it", () => {
    const challenge = { title: "Vétéran", description: "Accumule 5 heures", progress: 11_160, goal: 18_000, goalType: "any_playtime" };
    const { p, rerender } = renderHome({ challenge });
    expect(screen.getByText("3 h 06 sur 5 h 00")).toBeTruthy();
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("11160");
    rerender(<HomePage {...p} challenge={{ ...challenge, progress: null }} />);
    fireEvent.click(screen.getByRole("button", { name: "Participer" }));
    expect(p.onOpenChallenges).toHaveBeenCalled();
  });

  it("counts sessions for a launches challenge", () => {
    renderHome({ challenge: { title: "Hyper Actif", description: "Lance 10 sessions", progress: 3, goal: 10, goalType: "launches" } });
    expect(screen.getByText("3 sessions sur 10 sessions")).toBeTruthy();
  });

  it("sums up the month", () => {
    renderHome();
    expect(screen.getByText("Ce mois-ci")).toBeTruthy();
    expect(screen.getByText("9 h 12")).toBeTruthy();
    expect(screen.getByText("6")).toBeTruthy();
  });

  it("can be used with a gamepad", () => {
    const { container } = renderHome({ friends: [{ id: "u1", name: "Léna", avatarUrl: null, playing: null }] });
    for (const b of container.querySelectorAll("button")) expect(b.classList.contains("gamepad-nav-item"), b.textContent ?? "").toBe(true);
  });
});

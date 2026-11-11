import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { LobbyPage } from "./LobbyPage";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

const kart = { path: "D:\\kart.iso", name: "Kart Fiesta", console: "GameCube / Wii" };
const smb = { path: "D:\\smb3.nes", name: "Super Mario Bros. 3", console: "NES" };

function renderPage(over: Partial<Parameters<typeof LobbyPage>[0]> = {}) {
  const props = {
    t, signedIn: true, current: null, invites: [], games: [kart, smb], panel: null,
    onAccept: vi.fn(), onDecline: vi.fn(), onCreate: vi.fn(), onSignIn: vi.fn(),
    ...over,
  };
  render(<LobbyPage {...props} />);
  return props;
}

describe("Lobbies page", () => {
  it("asks to sign in first", () => {
    const p = renderPage({ signedIn: false });
    expect(screen.queryByRole("button", { name: "Créer un lobby" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Connexion / Inscription" }));
    expect(p.onSignIn).toHaveBeenCalled();
  });

  it("creates a lobby for a game that can be played online", () => {
    const p = renderPage();
    expect(screen.getByText(/Pas de lobby en cours/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Créer un lobby" }));
    const create = screen.getByRole("button", { name: "Créer" }) as HTMLButtonElement;
    expect(create.disabled).toBe(true);
    fireEvent.change(screen.getByRole("combobox", { name: "Choisis un jeu jouable en ligne" }), { target: { value: smb.path } });
    fireEvent.click(create);
    expect(p.onCreate).toHaveBeenCalledWith(smb);
  });

  it("says when no game of the library can be played online", () => {
    renderPage({ games: [] });
    fireEvent.click(screen.getByRole("button", { name: "Créer un lobby" }));
    expect(screen.getByText(/Aucun jeu de ta bibliothèque/)).toBeTruthy();
  });

  it("shows the lobby the player is in, and no second lobby to create", () => {
    renderPage({ current: { id: "l1", game: "Kart Fiesta", console: "GameCube / Wii", host: "Léna", players: 3 }, panel: <section>panel</section> });
    expect(screen.getByText("Tu y es")).toBeTruthy();
    expect(screen.getByText("GameCube / Wii · hôte Léna")).toBeTruthy();
    expect(screen.getByText("3 joueur(s)")).toBeTruthy();
    expect(screen.getByText("panel")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Créer un lobby" })).toBeNull();
  });

  it("joins or declines an invitation", () => {
    const p = renderPage({ invites: [{ id: "inv", game: "Robo Rumble", console: "Nintendo 64", host: "Max" }] });
    expect(screen.queryByText(/joueur\(s\)/)).toBeNull(); // members of an invitation are unknown
    fireEvent.click(screen.getByRole("button", { name: "Rejoindre" }));
    expect(p.onAccept).toHaveBeenCalledWith("inv");
    fireEvent.click(screen.getByRole("button", { name: "Refuser" }));
    expect(p.onDecline).toHaveBeenCalledWith("inv");
  });

  it("explains the empty right side when there is no lobby open", () => {
    renderPage();
    expect(screen.getByText("Aucun lobby ouvert")).toBeTruthy();
  });

  it("can be used with a gamepad", () => {
    renderPage({ invites: [{ id: "inv", game: "Robo Rumble", console: "Nintendo 64", host: "Max" }] });
    fireEvent.click(screen.getByRole("button", { name: "Créer un lobby" }));
    for (const el of document.querySelectorAll("button, select")) expect(el.classList.contains("gamepad-nav-item"), el.textContent ?? "").toBe(true);
  });
});

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { LobbyPanel, type LobbyView } from "./LobbyPanel";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

const base: LobbyView = {
  id: "lobby-1", host_id: "host", game_name: "Pokemon - Version Rubis (France)", game_console: "Game Boy Advance", status: "waiting",
  members: [
    { user_id: "host", is_ready: true, profile: { username: "fj123u" } },
    { user_id: "friend", is_ready: false, profile: { username: "Léo" } },
  ],
};

function renderPanel(props: Partial<Parameters<typeof LobbyPanel>[0]> = {}) {
  const handlers = { onInvite: vi.fn(), onReady: vi.fn(), onLaunch: vi.fn(), onJoin: vi.fn(), onLeave: vi.fn(), onGetGame: vi.fn(), onPublishCode: vi.fn() };
  render(
    <LobbyPanel
      lobby={base} userId="host" friends={[]} myRomName={base.game_name} busy={null}
      method={null} hostCode={null} awaitingHostCode={false} t={t}
      {...handlers} {...props}
    />,
  );
  return handlers;
}

describe("lobby panel", () => {
  it("shows everyone by name and status, translated", () => {
    renderPanel({ userId: "friend" });
    const panel = screen.getByRole("region", { name: "Lobby" });
    expect(panel.textContent).toContain("fj123u · hôte");
    expect(panel.textContent).toContain("Toi");
    expect(panel.textContent).toContain("✓ Prêt");
    expect(panel.textContent).toContain("En attente...");
    expect(panel.textContent).not.toMatch(/Joueur|Ami\b/);
  });

  it("names a member whose profile is missing as a player", () => {
    renderPanel({ lobby: { ...base, members: [base.members![0], { user_id: "x", is_ready: false }] } });
    expect(screen.getByText("Joueur")).toBeTruthy();
  });

  it("lets only the host launch, once everyone is ready", () => {
    const ready = { ...base, members: base.members!.map((m) => ({ ...m, is_ready: true })) };
    renderPanel({ lobby: base });
    expect(screen.queryByRole("button", { name: /Lancer le jeu/ })).toBeNull();

    const { onLaunch } = renderPanel({ lobby: ready });
    fireEvent.click(screen.getByRole("button", { name: /Lancer le jeu/ }));
    expect(onLaunch).toHaveBeenCalledTimes(1);
  });

  it("disables launching while the game is being opened", () => {
    const ready = { ...base, members: base.members!.map((m) => ({ ...m, is_ready: true })) };
    renderPanel({ lobby: ready, busy: "launching" });
    expect((screen.getByRole("button", { name: /Lancer le jeu/ }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("Ouverture de la partie en ligne…")).toBeTruthy();
  });

  it("offers a friend to join a game in progress", () => {
    const { onJoin } = renderPanel({ userId: "friend", lobby: { ...base, status: "playing" } });
    expect(screen.getByText("Partie en cours")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Rejoindre la partie/ }));
    expect(onJoin).toHaveBeenCalled();
  });

  it("warns when the player's copy is another version", () => {
    renderPanel({ userId: "friend", myRomName: "Pokemon - Version Rubis (Europe)" });
    expect(screen.getByText(/Ta version : « Pokemon - Version Rubis \(Europe\) »/)).toBeTruthy();
  });

  it("sends a player without the game to get it", () => {
    const { onGetGame } = renderPanel({ userId: "friend", myRomName: null });
    fireEvent.click(screen.getByRole("button", { name: /Télécharger le jeu/ }));
    expect(onGetGame).toHaveBeenCalled();
  });

  it("guides a Dolphin host and shares the code it pastes", () => {
    const ready = { ...base, members: base.members!.map((m) => ({ ...m, is_ready: true })) };
    const { onPublishCode } = renderPanel({ lobby: ready, method: "dolphin", awaitingHostCode: true });
    expect(screen.getByText(/Outils → Démarrer NetPlay → onglet « Héberger »/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Lancer le jeu/ }), "already launched").toBeNull();

    const input = screen.getByRole("textbox", { name: "Code NetPlay Dolphin" });
    const share = screen.getByRole("button", { name: /Partager/ }) as HTMLButtonElement;
    fireEvent.change(input, { target: { value: "ab" } });
    expect(share.disabled).toBe(true);
    fireEvent.change(input, { target: { value: "  Ab12Cd34 " } });
    fireEvent.click(share);
    expect(onPublishCode).toHaveBeenCalledWith("Ab12Cd34");
  });

  it("shows friends the host's code and what to do with it", () => {
    renderPanel({ userId: "friend", lobby: { ...base, status: "playing" }, method: "dolphin", hostCode: "Ab12Cd34" });
    expect(screen.getByText("Ab12Cd34")).toBeTruthy();
    expect(screen.getByText(/Le code de l'hôte est déjà rempli/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /Copier/ })).toBeTruthy();
  });

  it("explains PPSSPP's ad hoc mode once the game runs", () => {
    renderPanel({ userId: "friend", lobby: { ...base, status: "playing" }, method: "ppsspp" });
    expect(screen.getByText(/serveur ad hoc de PPSSPP/)).toBeTruthy();
  });

  it("tells the host and friends how to meet in a Switch game", () => {
    renderPanel({ lobby: { ...base, status: "playing" }, method: "ryubing" });
    expect(screen.getByText(/crée une partie en jeu local \/ sans fil/)).toBeTruthy();
  });

  it("tells a friend to join the host's Switch session", () => {
    renderPanel({ userId: "friend", lobby: { ...base, status: "playing" }, method: "ryubing" });
    expect(screen.getByText(/rejoins la partie de l'hôte/)).toBeTruthy();
  });

  it("only offers friends who are not in the lobby yet", () => {
    const { onInvite } = renderPanel({ friends: [{ id: "friend", username: "Léo" }, { id: "max", username: "Max" }] });
    const select = screen.getByRole("combobox", { name: "Inviter un ami..." });
    expect([...select.querySelectorAll("option")].map((o) => o.textContent)).toEqual(["Inviter un ami...", "Max"]);
    fireEvent.change(select, { target: { value: "max" } });
    expect(onInvite).toHaveBeenCalledWith("max");
  });
});

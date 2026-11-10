import { describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { mockBackend } from "./test/tauriMock";
import { COVER_CHANGED_EVENT, CoverPicker, type CoverGame } from "./CoverPicker";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

const rom = { name: "Tomb Raider (Europe)", console: "PlayStation 1" };
const games: CoverGame[] = [
  { parent_id: 1, name: "Tomb Raider", platform: "PlayStation", year: 1996, covers: [
    { url: "https://assets.iisu.network/eu.png", thumbnail: "https://assets.iisu.network/eu_t.webp", region: "Europe", console_id: "ps1" },
    { url: "https://assets.iisu.network/us.png", thumbnail: null, region: "USA", console_id: "ps1" },
  ] },
  { parent_id: 2, name: "Tomb Raider", platform: "Xbox 360", year: 2013, covers: [
    { url: "https://assets.iisu.network/2013.png", thumbnail: null, region: "World", console_id: "xbox360" },
  ] },
];

describe("cover picker", () => {
  it("shows every game iiDB hesitates between, with their covers", async () => {
    const backend = mockBackend({ iidb_cover_choices: () => games });
    render(<CoverPicker rom={rom} t={t} onClose={() => {}} onNotify={() => {}} />);
    expect(await screen.findByText(/PlayStation, 1996/)).toBeTruthy();
    expect(screen.getByText(/Xbox 360, 2013/)).toBeTruthy();
    expect(screen.getByText(/iiDB hésite entre plusieurs jeux/)).toBeTruthy();
    const imgs = screen.getAllByRole("button", { name: /Utiliser cette cover/ }).map((b) => b.querySelector("img")!.getAttribute("src"));
    expect(imgs).toEqual(["https://assets.iisu.network/eu_t.webp", "https://assets.iisu.network/us.png", "https://assets.iisu.network/2013.png"]);
    expect(backend.calls.find((c) => c.cmd === "iidb_cover_choices")?.args).toEqual({ gameName: rom.name, console: rom.console });
  });

  it("applies the picked cover and tells the library", async () => {
    const backend = mockBackend({ iidb_cover_choices: () => games });
    const onClose = vi.fn();
    const onNotify = vi.fn();
    const changed = vi.fn();
    window.addEventListener(COVER_CHANGED_EVENT, changed);
    render(<CoverPicker rom={rom} t={t} onClose={onClose} onNotify={onNotify} />);
    const usa = await screen.findByRole("button", { name: "Utiliser cette cover (USA)" });
    await act(async () => {
      fireEvent.click(usa);
    });
    // parentId: the picked game is remembered for later searches.
    expect(backend.calls.find((c) => c.cmd === "set_cover_from_url")?.args).toEqual({ gameName: rom.name, console: rom.console, url: "https://assets.iisu.network/us.png", parentId: 1 });
    expect((changed.mock.calls[0][0] as CustomEvent).detail).toEqual({ name: rom.name, console: rom.console, dataUrl: "data:image/webp;base64,NEW" });
    expect(onNotify).toHaveBeenCalledWith("Cover mise à jour", "success");
    expect(onClose).toHaveBeenCalled();
    window.removeEventListener(COVER_CHANGED_EVENT, changed);
  });

  it("marks the game picked earlier and sends the right game when picking another", async () => {
    const backend = mockBackend({ iidb_cover_choices: () => [{ ...games[1], remembered: true }, games[0]] });
    render(<CoverPicker rom={rom} t={t} onClose={() => {}} onNotify={() => {}} />);
    const chosen = await screen.findByText("✓ Ton choix");
    expect(chosen.closest("section")?.textContent).toContain("Xbox 360, 2013");
    expect(screen.getByText(/EmuWorld s'en souviendra/)).toBeTruthy();
    const eu = screen.getByRole("button", { name: "Utiliser cette cover (Europe)" });
    await act(async () => {
      fireEvent.click(eu);
    });
    expect((backend.calls.find((c) => c.cmd === "set_cover_from_url")?.args as { parentId: number }).parentId).toBe(1);
  });

  it("takes a cover from an image on the PC, even without iiDB", async () => {
    const backend = mockBackend({
      iidb_cover_choices: () => { throw "no_key"; },
      "plugin:dialog|open": () => "C:\\Covers\\tomb raider.jpg",
    });
    const onClose = vi.fn();
    const changed = vi.fn();
    window.addEventListener(COVER_CHANGED_EVENT, changed);
    render(<CoverPicker rom={rom} t={t} onClose={onClose} onNotify={() => {}} />);
    await screen.findByText(/Ajoute ta clé API iiDB/);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Depuis un fichier/ }));
    });
    await waitFor(() => expect(backend.calls.find((c) => c.cmd === "set_cover_from_file")?.args).toEqual({ gameName: rom.name, console: rom.console, path: "C:\\Covers\\tomb raider.jpg" }));
    expect((changed.mock.calls[0][0] as CustomEvent).detail.dataUrl).toBe("data:image/webp;base64,FILE");
    expect(onClose).toHaveBeenCalled();
    window.removeEventListener(COVER_CHANGED_EVENT, changed);
  });

  it("does nothing when the file dialog is cancelled", async () => {
    const backend = mockBackend();
    render(<CoverPicker rom={rom} t={t} onClose={() => {}} onNotify={() => {}} />);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Depuis un fichier/ }));
    });
    expect(backend.count("set_cover_from_file")).toBe(0);
  });

  it("explains that the iiDB key is needed", async () => {
    mockBackend({ iidb_cover_choices: () => { throw "no_key"; } });
    render(<CoverPicker rom={rom} t={t} onClose={() => {}} onNotify={() => {}} />);
    expect(await screen.findByText(/Ajoute ta clé API iiDB/)).toBeTruthy();
  });

  it("says when iiDB has nothing, and credits iiDB", async () => {
    const backend = mockBackend();
    render(<CoverPicker rom={rom} t={t} onClose={() => {}} onNotify={() => {}} />);
    expect(await screen.findByText("iiDB n'a pas de cover pour ce jeu.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "iiDB" }));
    await waitFor(() => expect(backend.calls.some((c) => c.cmd === "plugin:opener|open_url" && JSON.stringify(c.args).includes("https://iidb.iisu.network"))).toBe(true));
  });
});

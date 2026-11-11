// End-to-end-ish tests of the whole UI against the fake backend in
// src/test/tauriMock.ts. They catch the regressions a monolithic App.tsx is
// prone to: a page that crashes on render, an effect that loops, duplicate
// React keys, a backend command called with the wrong arguments.
import { Component, type ReactNode } from "react";
import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { emit } from "@tauri-apps/api/event";
import App, { CoverImage } from "./App";
import { AchievementCelebration, CELEBRATION_MS } from "./AchievementCelebration";
import { mockBackend, type Handler } from "./test/tauriMock";

class Boundary extends Component<{ onError: (e: Error) => void; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch(error: Error) {
    this.props.onError(error);
  }
  render() {
    return this.state.failed ? <div data-testid="crashed" /> : this.props.children;
  }
}

/** React problems that are bugs even when nothing visibly crashes. */
const REACT_BUG_PATTERNS = [
  /Encountered two children with the same key/,
  /Maximum update depth exceeded/,
  /Each child in a list should have a unique "key"/,
  /Cannot update a component .* while rendering a different component/,
];

let consoleErrors: string[] = [];
let renderErrors: Error[] = [];

beforeEach(() => {
  consoleErrors = [];
  renderErrors = [];
  localStorage.setItem("emuworld_onboarding_done", "true");
  localStorage.setItem("emuworld-locale", "fr");
  vi.spyOn(console, "error").mockImplementation((...args: unknown[]) => {
    consoleErrors.push(args.map(String).join(" "));
  });
});

afterEach(() => {
  expect(renderErrors.map((e) => e.stack ?? String(e))).toEqual([]);
  const reactBugs = consoleErrors.filter((m) => REACT_BUG_PATTERNS.some((p) => p.test(m)));
  expect(reactBugs).toEqual([]);
});

async function renderApp(overrides: Record<string, Handler> = {}) {
  const backend = mockBackend(overrides);
  const utils = render(
    <Boundary onError={(e) => renderErrors.push(e)}>
      <App />
    </Boundary>,
  );
  // Startup is done once the library has been scanned.
  await waitFor(() => expect(backend.count("scan_roms")).toBeGreaterThan(0));
  return { backend, ...utils };
}

/** Buttons of the v3 navigation: the section bar, then the tabs of the open section. */
function navButtons(container: HTMLElement) {
  const bar = container.querySelector(".v3-topbar");
  if (!bar) throw new Error("navigation bar not found");
  return [...bar.querySelectorAll<HTMLElement>(".v3-dock__item, .v3-tabs__item")];
}

/** Opens a page by its label: the section in the bar, then its tab ("Roms" is a tab of Jeux). */
function navTo(container: HTMLElement, label: string) {
  const find = () => navButtons(container).find((b) => (b.getAttribute("aria-label") ?? "") === label || b.textContent?.trim().startsWith(label));
  let button = find();
  for (const section of [...container.querySelectorAll<HTMLElement>(".v3-dock__item")]) {
    if (button) break;
    fireEvent.click(section);
    button = find();
  }
  if (!button) throw new Error(`no navigation button "${label}"`);
  fireEvent.click(button);
}

describe("App", () => {
  it("starts and loads the library from the backend", async () => {
    const { backend, container } = await renderApp();
    expect(screen.queryByTestId("crashed")).toBeNull();
    // The library scan uses the configured ROMs folder.
    const scan = backend.calls.find((c) => c.cmd === "scan_roms");
    expect(scan?.args).toMatchObject({ directory: "D:\\Roms" });
    expect(navButtons(container).length).toBeGreaterThanOrEqual(6);
  });

  it("shows the app's real version in the titlebar", async () => {
    const { container } = await renderApp();
    // tauriMock answers plugin:app|version with "0.0.0-test"; it used to be a hardcoded "v2.5.3".
    await waitFor(() => expect(container.querySelector(".titlebar__version")?.textContent).toBe("v0.0.0-test"));
  });

  it("opens every page without crashing", async () => {
    const { container, backend } = await renderApp();
    const visited = new Set<string>();
    const open = async (button: HTMLElement) => {
      await act(async () => {
        fireEvent.click(button);
      });
      // Let effects (data loads) of the new page settle.
      await act(async () => {
        await new Promise((r) => setTimeout(r, 50));
      });
      const main = container.querySelector<HTMLElement>("main.main-content")!;
      const page = main.dataset.page!;
      visited.add(page);
      expect(screen.queryByTestId("crashed"), `page "${page}" crashed`).toBeNull();
      expect(main.textContent?.trim().length, `page "${page}" rendered nothing`).toBeGreaterThan(0);
    };
    // Every section of the bar, then every tab of that section.
    const sections = [...container.querySelectorAll<HTMLElement>(".v3-dock__item")];
    expect(sections.length).toBe(6);
    for (const section of sections) {
      await open(section);
      for (const tab of [...container.querySelectorAll<HTMLElement>(".v3-tabs__item[data-nav-page]")]) await open(tab);
    }
    // Same 15 pages as the old sidebar: the test must not visit fewer.
    expect([...visited].sort()).toEqual(["backup", "catalog", "challenges", "changelogs", "controller", "discover", "friends", "installed", "leaderboard", "library", "marketplace", "settings", "stats", "store", "wrap"]);
    // Commands answered with `null` because tauriMock.ts has no handler.
    // Not a failure, but a hint the mock should learn the real return shape.
    if (backend.unknown.size > 0) {
      process.stdout.write(`[tauriMock] unmocked commands: ${[...backend.unknown].join(", ")}\n`);
    }
  });

  it("renders an empty library without crashing", async () => {
    await renderApp({ scan_roms: () => [], get_playtime: () => ({ games: {}, emulators: {}, collections: [] }) });
    expect(screen.queryByTestId("crashed")).toBeNull();
  });

  it("shows hostile ROM names as text, never as HTML", async () => {
    const { container } = await renderApp();
    navTo(container, "Roms");
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50));
    });
    expect(container.querySelector("main b")).toBeNull();
    expect(container.querySelector("script")).toBeNull();
  });

  it("survives a backend that fails every command", async () => {
    const failing = mockBackend({
      get_config: () => { throw new Error("backend down"); },
      scan_roms: () => { throw new Error("backend down"); },
      get_playtime: () => { throw new Error("backend down"); },
    });
    render(
      <Boundary onError={(e) => renderErrors.push(e)}>
        <App />
      </Boundary>,
    );
    await waitFor(() => expect(failing.count("get_config")).toBeGreaterThan(0));
    await act(async () => {
      await new Promise((r) => setTimeout(r, 100));
    });
    expect(screen.queryByTestId("crashed")).toBeNull();
  });
});

describe("Vimm store download", () => {
  const vimmGame = { id: "12345", name: "Zelda Test", console: "NES", size: "1 MB", region: "Europe" };

  async function openVimmResults(overrides: Record<string, Handler>) {
    const r = await renderApp({ search_vimm: () => [vimmGame], ...overrides });
    navTo(r.container, "Store");
    const search = await screen.findByPlaceholderText(/Vimm/i);
    fireEvent.change(search, { target: { value: "zelda" } });
    const row = await screen.findByText("Zelda Test", {}, { timeout: 3000 });
    return { ...r, row: row.closest("div")!.parentElement! };
  }

  it("starts one download even on a fast double click", async () => {
    let finish: (v: string) => void = () => {};
    const { backend } = await openVimmResults({
      download_vimm_rom: () => new Promise<string>((r) => { finish = r; }),
    });
    const button = screen.getByRole("button", { name: /Télécharger/ });
    // Same tick, before any re-render: the old stale-closure guard let all
    // three through.
    fireEvent.click(button);
    fireEvent.click(button);
    fireEvent.click(button);
    await waitFor(() => expect(backend.count("download_vimm_rom")).toBe(1));
    expect(backend.calls.find((c) => c.cmd === "download_vimm_rom")?.args).toMatchObject({
      gameId: "12345",
      gameName: "Zelda Test",
    });
    await act(async () => finish("ok"));
  });

  it("shows extraction instead of freezing at 99%", async () => {
    let finish: (v: string) => void = () => {};
    await openVimmResults({
      download_vimm_rom: () => new Promise<string>((r) => { finish = r; }),
    });
    fireEvent.click(screen.getByRole("button", { name: /Télécharger/ }));

    await act(async () => {
      await emit("vimm-download-progress", { game_id: "12345", status: "downloading", progress: 99, downloaded_bytes: 99, total_bytes: 100, speed_bps: 10, eta: 1 });
    });
    expect(screen.getAllByText(/99%/).length).toBeGreaterThan(0);

    await act(async () => {
      await emit("vimm-download-progress", { game_id: "12345", status: "extracting", progress: 99 });
    });
    expect(screen.getAllByText(/Extraction/).length).toBeGreaterThan(0);

    await act(async () => finish("ok"));
    await waitFor(() => expect(screen.getByRole("button", { name: /Télécharger/ })).toBeTruthy());
  });

  it("reports a failed download and lets the user retry", async () => {
    let attempts = 0;
    const { backend } = await openVimmResults({
      download_vimm_rom: () => {
        attempts++;
        throw "Vimm download HTTP 503";
      },
    });
    fireEvent.click(screen.getByRole("button", { name: /Télécharger/ }));
    await screen.findByText(/HTTP 503/);
    fireEvent.click(await screen.findByRole("button", { name: /Télécharger/ }));
    await waitFor(() => expect(backend.count("download_vimm_rom")).toBe(2));
    expect(attempts).toBe(2);
  });
});

describe("smart playlists", () => {
  async function openLibraryFilter() {
    const r = await renderApp();
    navTo(r.container, "Roms");
    const filter = (await screen.findByDisplayValue("Tous")) as HTMLSelectElement;
    return { ...r, filter };
  }

  it("lists the games matching a smart playlist, with live counts", async () => {
    const { filter, container } = await openLibraryFilter();
    // Super Mario Bros. 3 has 10 h in the sample playtime; nothing else does.
    expect(screen.getByRole("option", { name: "Gros temps de jeu (10h+) (1)" })).toBeTruthy();
    await act(async () => {
      fireEvent.change(filter, { target: { value: "long" } });
    });
    const grid = container.querySelector(".game-grid")!;
    expect(grid.textContent).toContain("Super Mario Bros. 3");
    expect(grid.textContent).not.toContain("Tetris");
  });

  it("explains an empty playlist instead of showing a blank search", async () => {
    const { filter } = await openLibraryFilter();
    await act(async () => {
      fireEvent.change(filter, { target: { value: "stale" } });
    });
    expect(screen.getByText("Aucun jeu ne correspond à ce filtre")).toBeTruthy();
  });
});

describe("RetroArch rewind setting", () => {
  it("saves the choice in the config, and can hand control back to RetroArch", async () => {
    const { container, backend } = await renderApp();
    navTo(container, "Réglages");
    const field = (await screen.findByText("Rembobinage (rewind)")).closest(".settings__field") as HTMLElement;
    const button = (name: string) => [...field.querySelectorAll("button")].find((b) => b.textContent === name)!;

    await act(async () => {
      fireEvent.click(button("Activé"));
    });
    const saved = () => backend.calls.filter((c) => c.cmd === "save_config").pop()?.args?.config;
    expect(saved()).toMatchObject({ roms_directory: "D:\\Roms", retroarch_rewind: true });
    expect(button("Activé").className).toContain("btn--primary");

    await act(async () => {
      fireEvent.click(button("Laisser RetroArch décider"));
    });
    expect(saved()).toMatchObject({ retroarch_rewind: null });
  });
});

describe("achievement celebration", () => {
  const unlocked = [
    { id: "first_launch", name: "Premier pas", description: "Lancer un premier jeu", icon: "🎮", unlocked: true, unlocked_at: "2026-11-05T10:00:00Z", hidden: false },
    { id: "night_owl", name: "Oiseau de nuit", description: "Jouer après minuit", icon: "🦉", unlocked: true, unlocked_at: "2026-11-05T10:00:00Z", hidden: true },
  ];
  /** check_achievements reports the unlocks once, like the real backend. */
  const unlockOnce = () => {
    let sent = false;
    return () => (sent ? [] : ((sent = true), unlocked));
  };

  it("celebrates each new achievement in turn instead of a toast", async () => {
    await renderApp({ check_achievements: unlockOnce() });
    const dialog = await screen.findByRole("alertdialog");
    expect(dialog.textContent).toContain("Premier pas");
    expect(screen.queryByText(/Succès débloqué : Premier pas/)).toBeNull();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Suivant" }));
    });
    expect(screen.getByRole("alertdialog").textContent).toContain("Oiseau de nuit");
    expect(screen.getByText("Succès secret débloqué !")).toBeTruthy();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Continuer" }));
    });
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });

  it("falls back to toasts when the animation is turned off", async () => {
    localStorage.setItem("emuworld_achievement_animation", "off");
    await renderApp({ check_achievements: unlockOnce() });
    expect(await screen.findByText(/Premier pas/)).toBeTruthy();
    expect(screen.queryByRole("alertdialog")).toBeNull();
    localStorage.removeItem("emuworld_achievement_animation");
  });

  it("waits for the player to come back before counting down", () => {
    vi.useFakeTimers();
    const hidden = vi.spyOn(document, "hidden", "get").mockReturnValue(true);
    const onDone = vi.fn();
    render(<AchievementCelebration queue={unlocked.slice(0, 1)} onDone={onDone} t={(k) => k} />);
    act(() => { vi.advanceTimersByTime(CELEBRATION_MS * 3); });
    expect(onDone).not.toHaveBeenCalled();

    hidden.mockReturnValue(false);
    act(() => { document.dispatchEvent(new Event("visibilitychange")); });
    act(() => { vi.advanceTimersByTime(CELEBRATION_MS); });
    expect(onDone).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });
});

describe("console colours", () => {
  const root = document.documentElement;

  async function openNesInLibrary() {
    const r = await renderApp();
    navTo(r.container, "Roms");
    fireEvent.click(await screen.findByText("Nintendo", { selector: ".rgs-console-card__name" }));
    fireEvent.click(await screen.findByText("NES", { selector: ".rgs-console-card__name" }));
    return r;
  }

  it("tints the app while browsing a console, and restores it after", async () => {
    const { container } = await openNesInLibrary();
    await waitFor(() => expect(root.hasAttribute("data-console-accent")).toBe(true));
    expect(root.style.getPropertyValue("--console-hue")).toBe("0");

    navTo(container, "Réglages");
    await waitFor(() => expect(root.hasAttribute("data-console-accent")).toBe(false));
  });

  it("can be turned off in the settings", async () => {
    localStorage.setItem("emuworld_console_colors", "off");
    await openNesInLibrary();
    await act(async () => {
      await new Promise((r) => setTimeout(r, 20));
    });
    expect(root.hasAttribute("data-console-accent")).toBe(false);
    localStorage.removeItem("emuworld_console_colors");
  });
});

describe("save states manager", () => {
  const states = [
    { path: "C:\\RA\\states\\Mesen\\smb3.state2", file_name: "smb3.state2", slot: 2, auto: false, folder: "Mesen", modified: "2026-11-04T20:00:00Z", size: 2_500_000, thumbnail: "data:image/png;base64,AA", label: "Avant Bowser" },
    { path: "C:\\RA\\states\\Mesen\\smb3.state.auto", file_name: "smb3.state.auto", slot: null, auto: true, folder: "Mesen", modified: "2026-11-03T20:00:00Z", size: 40_000, thumbnail: null, label: null },
  ];

  async function openSaveStates(overrides: Record<string, Handler> = {}) {
    const r = await renderApp({ list_save_states: () => states, ...overrides });
    navTo(r.container, "Roms");
    fireEvent.change(await screen.findByDisplayValue("Tous"), { target: { value: "long" } });
    await waitFor(() => expect(r.container.querySelector(".game-grid .game-card")).not.toBeNull());
    fireEvent.contextMenu(r.container.querySelector(".game-grid .game-card")!);
    fireEvent.click(await screen.findByRole("button", { name: /Save states/ }));
    const dialog = await screen.findByRole("dialog", { name: "Save states" });
    await screen.findByText("Avant Bowser");
    return { ...r, dialog };
  }

  it("lists the game's states, named or by slot", async () => {
    const { backend, dialog } = await openSaveStates();
    expect(backend.calls.find((c) => c.cmd === "list_save_states")?.args).toEqual({ romPath: "D:\\Roms\\NES\\smb3.nes" });
    expect(dialog.textContent).toContain("Avant Bowser");
    expect(dialog.textContent).toContain("Slot 2");
    expect(dialog.textContent).toContain("Sauvegarde auto");
    expect(dialog.querySelectorAll("img").length).toBe(1);
  });

  it("resumes the game from a slot through the launcher", async () => {
    const { backend } = await openSaveStates();
    fireEvent.click(screen.getByRole("button", { name: /Reprendre/ }));
    await waitFor(() => expect(backend.count("launch_emulator")).toBe(1));
    expect(backend.calls.find((c) => c.cmd === "launch_emulator")?.args).toMatchObject({ emulatorId: "mesen", entrySlot: 2 });
    expect(screen.queryByRole("dialog", { name: "Save states" })).toBeNull();
  });

  it("names, duplicates and deletes (after confirmation) a state", async () => {
    const { backend } = await openSaveStates();
    fireEvent.click(screen.getAllByRole("button", { name: "Nommer" })[1]);
    const input = screen.getByPlaceholderText("Nom (ex. : avant le boss)");
    fireEvent.change(input, { target: { value: "Monde 8" } });
    await act(async () => {
      fireEvent.keyDown(input, { key: "Enter" });
    });
    await waitFor(() => expect(backend.calls.find((c) => c.cmd === "rename_save_state")?.args).toEqual({ path: states[1].path, label: "Monde 8" }));

    await act(async () => {
      fireEvent.click(screen.getAllByRole("button", { name: "Dupliquer dans un slot libre" })[0]);
    });
    await waitFor(() => expect(backend.count("duplicate_save_state")).toBe(1));

    fireEvent.click(screen.getAllByRole("button", { name: "Mettre à la corbeille" })[0]);
    expect(backend.count("delete_save_state")).toBe(0);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Corbeille ?" }));
    });
    await waitFor(() => expect(backend.calls.find((c) => c.cmd === "delete_save_state")?.args).toEqual({ path: states[0].path }));
    // Every change reloads the list from disk.
    expect(backend.count("list_save_states")).toBeGreaterThanOrEqual(4);
  });

  it("explains how to make one when the game has none", async () => {
    const r = await renderApp({ list_save_states: () => [] });
    navTo(r.container, "Roms");
    fireEvent.change(await screen.findByDisplayValue("Tous"), { target: { value: "long" } });
    const card = r.container.querySelector(".game-grid .game-card")!;
    fireEvent.contextMenu(card);
    fireEvent.click(await screen.findByRole("button", { name: /Save states/ }));
    expect(await screen.findByText(/F2 sauvegarde dans le slot actuel/)).toBeTruthy();
  });
});

describe("game menu", () => {
  it("speaks the app's language", async () => {
    localStorage.setItem("emuworld-locale", "en");
    const r = await renderApp();
    navTo(r.container, "Roms");
    fireEvent.change(await screen.findByDisplayValue("All"), { target: { value: "long" } });
    await waitFor(() => expect(r.container.querySelector(".game-grid .game-card")).not.toBeNull());
    fireEvent.contextMenu(r.container.querySelector(".game-grid .game-card")!);
    const menu = (await screen.findByRole("button", { name: /Similar games/ })).closest(".rom-context-menu")!;
    for (const label of ["Play", "Notes", "New collection...", "Screenshots", "Guide", "Community reviews", "Delete"]) {
      expect(menu.textContent).toContain(label);
    }
    expect(menu.textContent).toMatch(/favorites/);
    for (const french of ["Jouer", "Avis communauté", "Jeux similaires", "Supprimer", "favoris", "Mettre en favori"]) {
      expect(menu.textContent).not.toContain(french);
    }
  });
});

describe("cover picker in the library", () => {
  it("opens from a game's menu and updates its card with the chosen cover", async () => {
    const r = await renderApp({
      iidb_cover_choices: () => [{ parent_id: 9, name: "Super Mario Bros. 3", platform: "NES", year: 1988, covers: [
        { url: "https://assets.iisu.network/smb3.png", thumbnail: null, region: "Europe", console_id: "nes" },
      ] }],
    });
    navTo(r.container, "Roms");
    fireEvent.change(await screen.findByDisplayValue("Tous"), { target: { value: "long" } });
    await waitFor(() => expect(r.container.querySelector(".game-grid .game-card")).not.toBeNull());
    const card = r.container.querySelector(".game-grid .game-card") as HTMLElement;
    fireEvent.contextMenu(card);
    fireEvent.click(await screen.findByRole("button", { name: /Choisir la cover/ }));
    const pick = await screen.findByRole("button", { name: "Utiliser cette cover (Europe)" });
    await act(async () => {
      fireEvent.click(pick);
    });
    expect(r.backend.calls.find((c) => c.cmd === "set_cover_from_url")?.args).toMatchObject({ url: "https://assets.iisu.network/smb3.png" });
    await waitFor(() => expect(card.querySelector("img[src='data:image/webp;base64,NEW']")).not.toBeNull());
    expect(screen.queryByRole("dialog", { name: "Choisir la cover" })).toBeNull();
  });
});

describe("cover picker in Big Picture", () => {
  // One gamepad frame from the Rust side (gilrs); buttons are edge-triggered.
  async function pad(pressed: number | null) {
    const buttons = Array.from({ length: 16 }, (_, i) => i === pressed);
    await act(async () => {
      await emit("gamepad-state", { connected: true, name: "Pad", buttons, axes: [0, 0, 0, 0] });
    });
  }
  async function press(button: number) {
    await pad(button);
    await pad(null);
  }

  async function openBigPictureOnNes() {
    const r = await renderApp();
    await act(async () => {
      fireEvent.keyDown(window, { key: "F11" });
    });
    fireEvent.click(await screen.findByRole("button", { name: /NES/ }));
    await waitFor(() => expect(r.container.querySelector(".big-picture .bp-card")).not.toBeNull());
    return r;
  }

  it("X opens it for the focused game, inside Big Picture, and the pad drives it", async () => {
    const r = await openBigPictureOnNes();
    await press(15); // right: from the back button to the first game
    await press(2); // X
    const dialog = await screen.findByRole("dialog", { name: "Choisir la cover" });
    expect(dialog.closest(".big-picture"), "rendered above Big Picture").not.toBeNull();
    expect(r.backend.calls.find((c) => c.cmd === "iidb_cover_choices")?.args).toEqual({ gameName: "Super Mario Bros. 3 (Europe)", console: "NES" });
    expect(screen.getByText(/X — Choisir la cover/)).toBeTruthy();
    await press(13); // down: first item of the dialog
    await press(0); // A
    await waitFor(() => expect(r.backend.count("plugin:dialog|open")).toBe(1));
    await press(1); // B closes the dialog, Big Picture stays
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Choisir la cover" })).toBeNull());
    expect(r.container.querySelector(".big-picture")).not.toBeNull();
  });

  it("Y asks for a lobby for the focused game, and says why not when signed out", async () => {
    const r = await openBigPictureOnNes();
    expect(screen.getByText(/Y — Créer un lobby/)).toBeTruthy();
    await press(15); // right: from the back button to the first game
    await press(3); // Y
    const toast = await screen.findByText("Connecte-toi pour créer un lobby.");
    // Toasts show in Big Picture too (it is drawn above the app).
    expect(toast.closest(".big-picture")).not.toBeNull();
    expect(r.container.querySelector(".big-picture")).not.toBeNull();
  });

  it("opens with a right-click on a game card", async () => {
    const r = await openBigPictureOnNes();
    fireEvent.contextMenu(r.container.querySelector(".big-picture .bp-card")!);
    expect(await screen.findByRole("dialog", { name: "Choisir la cover" })).toBeTruthy();
  });
});

describe("gamepad in the v3 navigation", () => {
  async function pad(pressed: number | null) {
    const buttons = Array.from({ length: 16 }, (_, i) => i === pressed);
    await act(async () => {
      await emit("gamepad-state", { connected: true, name: "Pad", buttons, axes: [0, 0, 0, 0] });
    });
  }
  async function press(button: number) {
    await pad(button);
    await pad(null);
  }
  const focused = (c: HTMLElement) => c.querySelector<HTMLElement>(".gamepad-focused");

  it("moves along the section bar, opens a section with A, goes down into the page and back up", async () => {
    const r = await renderApp();
    await pad(null); // pad connected
    await press(15); // right: Accueil -> Jeux
    await waitFor(() => expect(focused(r.container)?.getAttribute("data-section")).toBe("games"));
    await press(0); // A
    await waitFor(() => expect(r.container.querySelector<HTMLElement>("main.main-content")!.dataset.page).toBe("library"));
    // The pad stays on the open section of the bar.
    await waitFor(() => expect(focused(r.container)?.getAttribute("data-section")).toBe("games"));
    await press(13); // down: into the page
    await waitFor(() => expect(focused(r.container)?.closest("main.main-content")).not.toBeNull());
    await press(12); // up from the first row: back to the bar, on the open tab
    await waitFor(() => expect(focused(r.container)?.getAttribute("data-nav-page")).toBe("library"));
  });
});

describe("gamepad in the library", () => {
  it("Y toggles the favorite of the focused game", async () => {
    const r = await renderApp();
    navTo(r.container, "Roms");
    fireEvent.change(await screen.findByDisplayValue("Tous"), { target: { value: "long" } });
    await waitFor(() => expect(r.container.querySelector(".game-grid .game-card")).not.toBeNull());
    const card = r.container.querySelector(".game-grid .game-card") as HTMLElement;
    card.classList.add("gamepad-focused");
    const frame = (pressed: number | null) => emit("gamepad-state", { connected: true, name: "Pad", buttons: Array.from({ length: 16 }, (_, i) => i === pressed), axes: [0, 0, 0, 0] });
    await act(async () => {
      await frame(3); // Y
      await frame(null);
    });
    // The listener is registered once: it used to see an empty library.
    await waitFor(() => expect(r.backend.count("toggle_favorite")).toBe(1));
    const args = r.backend.calls.find((c) => c.cmd === "toggle_favorite")?.args as { console: string; name: string };
    expect(args.name.length).toBeGreaterThan(0);
    expect(args.console.length).toBeGreaterThan(0);
  });
});

describe("CoverImage", () => {
  function loadWith(img: HTMLImageElement, width: number, height: number) {
    Object.defineProperty(img, "naturalWidth", { value: width, configurable: true });
    Object.defineProperty(img, "naturalHeight", { value: height, configurable: true });
    fireEvent.load(img);
  }

  it("shows square art (iiDB) whole over a blurred backdrop", () => {
    const { container } = render(<CoverImage src="data:image/webp;base64,AA" alt="Super Mario Bros. 3" />);
    loadWith(screen.getByAltText("Super Mario Bros. 3") as HTMLImageElement, 1024, 1024);
    expect(screen.getByAltText("Super Mario Bros. 3").className).toContain("cover--contain");
    expect(container.querySelector("img.cover-backdrop")).not.toBeNull();
  });

  it("leaves portrait box art as it was", () => {
    const { container } = render(<CoverImage src="data:image/webp;base64,AA" alt="Metroid Prime" />);
    loadWith(screen.getByAltText("Metroid Prime") as HTMLImageElement, 600, 800);
    expect(screen.getByAltText("Metroid Prime").className).not.toContain("cover--contain");
    expect(container.querySelector("img.cover-backdrop")).toBeNull();
  });
});

describe("Google / Discord sign-in return", () => {
  it("shows why the provider refused and logs it", async () => {
    const { backend } = await renderApp();
    await act(async () => {
      await emit("oauth-callback", "emuworld://auth-callback?error=access_denied&error_description=User+cancelled+login");
    });
    expect(screen.getByText("Erreur d'authentification : User cancelled login")).toBeTruthy();
    expect(backend.calls.filter((c) => c.cmd === "log_event").map((c) => c.args)).toContainEqual({
      level: "WARN",
      message: "OAuth: connexion refusée: User cancelled login",
    });
  });

  it("never accepts raw tokens from a link", async () => {
    const { backend } = await renderApp();
    await act(async () => {
      await emit("oauth-callback", "emuworld://auth-callback#access_token=evil&refresh_token=evil");
    });
    expect(screen.queryByText(/Connecté/)).toBeNull();
    expect(backend.calls.filter((c) => c.cmd === "log_event").map((c) => c.args)).toContainEqual({
      level: "WARN",
      message: "OAuth: retour sans code ni erreur, ignoré",
    });
  });
});

describe("toasts", () => {
  it("keeps two toasts fired in the same millisecond", async () => {
    vi.spyOn(Date, "now").mockReturnValue(1_800_000_000_000);
    await renderApp();
    await act(async () => {
      await emit("roms-detected", ["Game A.nes"]);
      await emit("roms-detected", ["Game B.nes"]);
    });
    expect(screen.getByText(/Game A\.nes/)).toBeTruthy();
    expect(screen.getByText(/Game B\.nes/)).toBeTruthy();
  });
});

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { emit } from "@tauri-apps/api/event";
import { mockBackend } from "./test/tauriMock";
import { SpeedrunModal, SpeedrunWindow } from "./SpeedrunTimer";
import type { SpeedrunData } from "./speedrun";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

const stored: SpeedrunData = {
  splits: ["Monde 1", "Monde 2", "Bowser"],
  pb: [60_000, 150_000, 300_000],
  best_segments: [60_000, 90_000, 150_000],
  attempts: 5,
  completed: 2,
};

afterEach(() => {
  vi.restoreAllMocks();
  window.history.pushState({}, "", "/");
});

describe("speedrun timer window", () => {
  async function openWindow() {
    window.history.pushState({}, "", "/?speedrun=1&game=NES%3A%3AMario&name=Super%20Mario%20Bros.%203");
    let clock = 0;
    vi.spyOn(performance, "now").mockImplementation(() => clock);
    const backend = mockBackend({ get_speedrun: () => stored });
    render(<SpeedrunWindow t={t} />);
    await screen.findByText("Monde 2");
    const press = async (key: string, at: number) => {
      clock = at;
      await act(async () => { await emit("speedrun-hotkey", key); });
    };
    return { backend, press };
  }

  const saves = (backend: ReturnType<typeof mockBackend>) =>
    backend.calls.filter((c) => c.cmd === "save_speedrun").map((c) => (c.args as { data: SpeedrunData }).data);

  const lastSave = (backend: ReturnType<typeof mockBackend>) => {
    const all = saves(backend);
    return all[all.length - 1];
  };

  it("counts an attempt when a run starts, and saves a faster run as the record", async () => {
    const { backend, press } = await openWindow();
    expect(backend.calls.find((c) => c.cmd === "get_speedrun")?.args).toEqual({ gameKey: "NES::Mario" });

    await press("split", 1_000);
    await waitFor(() => expect(saves(backend)[0]?.attempts).toBe(6));

    await press("split", 56_000); // Monde 1 in 55 s (PB 60 s)
    expect(screen.getByText("-5.0")).toBeTruthy();
    await press("split", 141_000);
    await press("split", 281_000); // 4:40 < 5:00

    await waitFor(() => expect(screen.getByText("🏆 Nouveau record !")).toBeTruthy());
    const last = lastSave(backend)!;
    expect(last.pb).toEqual([55_000, 140_000, 280_000]);
    expect(last.completed).toBe(3);
  });

  it("keeps the best segments of a reset run, not its time", async () => {
    const { backend, press } = await openWindow();
    await press("split", 0);
    await press("split", 50_000); // gold Monde 1
    await press("reset", 70_000);
    await waitFor(() => expect(lastSave(backend)?.best_segments[0]).toBe(50_000));
    expect(lastSave(backend)?.pb).toEqual(stored.pb);
    expect(screen.getByText("0.00")).toBeTruthy();
  });

  it("counts every hotkey, even pressed in the same instant", async () => {
    const { backend, press } = await openWindow();
    await act(async () => {
      // No render between the presses.
      await Promise.all([press("split", 0), press("split", 55_000), press("split", 140_000), press("split", 280_000)]);
    });
    await waitFor(() => expect(lastSave(backend)?.pb).toEqual([55_000, 140_000, 280_000]));
  });

  it("only reacts to its own hotkeys", async () => {
    const { backend, press } = await openWindow();
    await press("pause", 1_000); // nothing running yet
    await press("undo", 2_000);
    expect(saves(backend)).toEqual([]);
  });
});

describe("speedrun setup", () => {
  function renderModal(data: SpeedrunData = stored) {
    const backend = mockBackend({ get_speedrun: () => data });
    const onNotify = vi.fn();
    render(<SpeedrunModal rom={{ name: "Super Mario Bros. 3", console: "NES" }} t={t} onClose={() => {}} onNotify={onNotify} />);
    return { backend, onNotify };
  }

  it("shows the record and opens the timer for this game", async () => {
    const { backend } = renderModal();
    const dialog = await screen.findByRole("dialog", { name: "Speedrun" });
    await waitFor(() => expect(dialog.textContent).toContain("5:00.00Record"));
    expect(dialog.textContent).toContain("5:00.00Meilleur possible");
    expect(dialog.textContent).toContain("2/5Runs finies");
    fireEvent.click(screen.getByRole("button", { name: /Afficher le chrono/ }));
    await waitFor(() => expect(backend.calls.find((c) => c.cmd === "open_speedrun_window")?.args).toEqual({
      gameKey: "NES::Super Mario Bros. 3",
      gameName: "Super Mario Bros. 3",
    }));
  });

  it("warns that changing the number of splits clears the record", async () => {
    const { backend } = renderModal();
    const box = await screen.findByLabelText("Splits (un par ligne)");
    fireEvent.change(box, { target: { value: "Monde 1\nMonde 2\nMonde 3\nBowser" } });
    expect(screen.getByText(/ton record et tes meilleurs segments seront effacés/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(backend.count("save_speedrun")).toBe(1));
    const saved = (backend.calls.find((c) => c.cmd === "save_speedrun")!.args as { data: SpeedrunData }).data;
    expect(saved.splits).toEqual(["Monde 1", "Monde 2", "Monde 3", "Bowser"]);
    expect(saved.pb).toBeNull();
  });

  it("renames splits without losing the record", async () => {
    const { backend } = renderModal();
    const box = await screen.findByLabelText("Splits (un par ligne)");
    fireEvent.change(box, { target: { value: "W1\nW2\nBowser" } });
    expect(screen.queryByText(/seront effacés/)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(backend.count("save_speedrun")).toBe(1));
    expect((backend.calls.find((c) => c.cmd === "save_speedrun")!.args as { data: SpeedrunData }).data.pb).toEqual(stored.pb);
  });
});

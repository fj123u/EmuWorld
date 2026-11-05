import { describe, expect, it, vi, beforeEach } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { mockBackend } from "./test/tauriMock";
import { SaveStatesModal, type SaveState } from "./SaveStatesModal";
import type { SharedSaveState } from "./savestateSharing";
import fr from "./i18n/fr";

const sharing = vi.hoisted(() => ({
  listReceivedStates: vi.fn(),
  shareSaveState: vi.fn(),
  importReceivedState: vi.fn(),
  dismissReceivedState: vi.fn(),
}));
vi.mock("./savestateSharing", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./savestateSharing")>()),
  ...sharing,
}));

/** The real French strings, so the test reads like the UI. */
const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

const rom = { name: "Zelda (USA)", path: "D:\\Roms\\SNES\\Zelda (USA).sfc", console: "Super Nintendo" };
const mine: SaveState = {
  path: "C:\\RA\\states\\Snes9x\\Zelda (USA).state1", file_name: "Zelda (USA).state1", slot: 1, auto: false,
  folder: "Snes9x", core: "Snes9x", modified: "2026-11-04T20:00:00Z", size: 2048, thumbnail: null, label: "Donjon 3",
};
const received = (over: Partial<SharedSaveState>): SharedSaveState => ({
  id: "r1", sender_id: "friend-1", recipient_id: "me", console: "Super Nintendo", game_name: "Zelda (USA)",
  core: "Snes9x", slot: 4, label: "Avant Ganon", storage_path: "friend-1/a.state", size_bytes: 4096,
  thumbnail: null, created_at: "2026-11-05T09:00:00Z", sender_name: "Léo", ...over,
});
const friends = [{ id: "friend-1", username: "Léo" }, { id: "friend-2", username: "Max" }];

function renderModal(props: Partial<Parameters<typeof SaveStatesModal>[0]> = {}) {
  const onNotify = vi.fn();
  const onShared = vi.fn();
  render(
    <SaveStatesModal
      rom={rom} t={t} locale="fr" userId="me" friends={friends}
      onClose={() => {}} onResume={() => {}} onNotify={onNotify} onShared={onShared}
      {...props}
    />,
  );
  return { onNotify, onShared };
}

beforeEach(() => {
  vi.clearAllMocks();
  sharing.listReceivedStates.mockResolvedValue([]);
  sharing.shareSaveState.mockResolvedValue(undefined);
  sharing.importReceivedState.mockResolvedValue(mine);
  sharing.dismissReceivedState.mockResolvedValue(undefined);
});

describe("sharing save states with friends", () => {
  it("sends a state to the chosen friend and tells them in the chat", async () => {
    mockBackend({ list_save_states: () => [mine] });
    const { onNotify, onShared } = renderModal();
    fireEvent.click(await screen.findByRole("button", { name: "Partager avec un ami" }));
    const send = screen.getByRole("button", { name: /Envoyer/ }) as HTMLButtonElement;
    expect(send.disabled).toBe(true); // no friend picked yet

    fireEvent.change(screen.getByRole("combobox", { name: "Choisir un ami" }), { target: { value: "friend-2" } });
    await act(async () => {
      fireEvent.click(send);
    });
    expect(sharing.shareSaveState).toHaveBeenCalledWith({ senderId: "me", recipientId: "friend-2", state: mine, rom });
    expect(onShared).toHaveBeenCalledWith(friends[1]);
    expect(onNotify).toHaveBeenCalledWith("Save state envoyé à Max !", "success");
  });

  it("reports a failed send without closing the picker", async () => {
    mockBackend({ list_save_states: () => [mine] });
    sharing.shareSaveState.mockRejectedValue(new Error("Bucket not found"));
    const { onNotify, onShared } = renderModal();
    fireEvent.click(await screen.findByRole("button", { name: "Partager avec un ami" }));
    fireEvent.change(screen.getByRole("combobox", { name: "Choisir un ami" }), { target: { value: "friend-1" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Envoyer/ }));
    });
    expect(onNotify).toHaveBeenCalledWith("Envoi impossible : Bucket not found", "error");
    expect(onShared).not.toHaveBeenCalled();
    expect(screen.getByRole("combobox", { name: "Choisir un ami" })).toBeTruthy();
  });

  it("does not rewrite a name that was not changed", async () => {
    const backend = mockBackend({ list_save_states: () => [mine] });
    renderModal();
    fireEvent.click(await screen.findByRole("button", { name: "Nommer" }));
    const input = screen.getByDisplayValue("Donjon 3");
    await act(async () => {
      fireEvent.blur(input);
    });
    expect(backend.count("rename_save_state")).toBe(0);
    expect(screen.getByText("Donjon 3")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Nommer" }));
    fireEvent.change(screen.getByDisplayValue("Donjon 3"), { target: { value: "Donjon 4" } });
    await act(async () => {
      fireEvent.blur(screen.getByDisplayValue("Donjon 4"));
    });
    expect(backend.calls.find((c) => c.cmd === "rename_save_state")?.args).toEqual({ path: mine.path, label: "Donjon 4" });
  });

  it("hides sharing when signed out", async () => {
    mockBackend({ list_save_states: () => [mine] });
    renderModal({ userId: null });
    await screen.findByText("Donjon 3");
    expect(screen.queryByRole("button", { name: "Partager avec un ami" })).toBeNull();
    expect(sharing.listReceivedStates).not.toHaveBeenCalled();
  });

  it("shows only the states received for this game, and adds one to a new slot", async () => {
    const backend = mockBackend({ list_save_states: () => [mine] });
    sharing.listReceivedStates.mockResolvedValue([
      received({}),
      received({ id: "r2", game_name: "Metroid", label: "Pas ce jeu" }),
    ]);
    const { onNotify } = renderModal();
    expect(await screen.findByText("Reçus de tes amis")).toBeTruthy();
    expect(screen.getByText("Avant Ganon")).toBeTruthy();
    expect(screen.queryByText("Pas ce jeu")).toBeNull();

    sharing.listReceivedStates.mockResolvedValue([]);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Ajouter/ }));
    });
    expect(sharing.importReceivedState).toHaveBeenCalledWith(expect.objectContaining({ id: "r1" }), rom.path, "Avant Ganon · Léo");
    expect(onNotify).toHaveBeenCalledWith("Save state de Léo ajouté dans un nouveau slot !", "success");
    await waitFor(() => expect(screen.queryByText("Reçus de tes amis")).toBeNull());
    // The local list is re-read so the new slot shows up.
    expect(backend.count("list_save_states")).toBe(2);
  });

  it("warns when the state comes from another version of the game, and can decline it", async () => {
    mockBackend({ list_save_states: () => [] });
    sharing.listReceivedStates.mockResolvedValue([received({ game_name: "Zelda (Europe)" })]);
    renderModal();
    expect(await screen.findByText(/Créé avec « Zelda \(Europe\) »/)).toBeTruthy();

    sharing.listReceivedStates.mockResolvedValue([]);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Refuser" }));
    });
    expect(sharing.dismissReceivedState).toHaveBeenCalledWith(expect.objectContaining({ id: "r1" }));
    expect(sharing.importReceivedState).not.toHaveBeenCalled();
  });

  it("keeps the local manager working when sharing is not set up on Supabase", async () => {
    mockBackend({ list_save_states: () => [mine] });
    sharing.listReceivedStates.mockRejectedValue(new Error('relation "shared_savestates" does not exist'));
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    renderModal();
    expect(await screen.findByText("Donjon 3")).toBeTruthy();
    await waitFor(() => expect(warn).toHaveBeenCalled());
    expect(screen.queryByText("Reçus de tes amis")).toBeNull();
  });
});

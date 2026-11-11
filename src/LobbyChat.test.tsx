import { describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { LobbyChat } from "./LobbyChat";
import type { LobbyChatApi, LobbyMessage } from "./lobbyApi";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

const msg = (id: string, user_id: string, content: string, minute: number): LobbyMessage => ({
  id, lobby_id: "lobby-1", user_id, content, created_at: `2026-11-11T20:${String(minute).padStart(2, "0")}:00Z`,
});

/** Server double: history, sent messages, and a way to push a live message. */
function fakeApi(history: LobbyMessage[] = [], sendError?: string) {
  let live: ((m: LobbyMessage) => void) | null = null;
  const sent: { lobbyId: string; userId: string; content: string }[] = [];
  const unsubscribe = vi.fn();
  const api: LobbyChatApi = {
    load: async () => history,
    send: async (lobbyId, userId, content) => {
      if (sendError) throw new Error(sendError);
      sent.push({ lobbyId, userId, content });
      return msg(`sent-${sent.length}`, userId, content, 50);
    },
    subscribe: (_id, onMessage) => { live = onMessage; return unsubscribe; },
  };
  return { api, sent, unsubscribe, push: (m: LobbyMessage) => live?.(m) };
}

const names = { host: "fj123u", friend: "Léo" };

function renderChat(api: LobbyChatApi, onLog = vi.fn()) {
  return render(<LobbyChat lobbyId="lobby-1" userId="host" names={names} api={api} t={t} onLog={onLog} />);
}

describe("lobby chat", () => {
  it("shows the history with each author's name", async () => {
    const { api } = fakeApi([msg("1", "friend", "On lance ?", 1), msg("2", "host", "Go", 2)]);
    renderChat(api);
    const log = await screen.findByRole("log", { name: "Discussion du lobby" });
    await waitFor(() => expect(log.textContent).toContain("Léo On lance ?"));
    expect(log.textContent).toContain("Toi Go");
  });

  it("says when there is no message yet", async () => {
    renderChat(fakeApi().api);
    expect(await screen.findByText("Pas encore de message. Dis bonjour !")).toBeTruthy();
  });

  it("shows friends' messages as they arrive, once each", async () => {
    const { api, push } = fakeApi();
    renderChat(api);
    await screen.findByText(/Pas encore de message/);
    const live = msg("3", "friend", "Prêt !", 3);
    act(() => { push(live); push(live); });
    expect(screen.getAllByText("Prêt !")).toHaveLength(1);
  });

  it("sends the trimmed message, clears the field and shows it only once", async () => {
    const { api, sent, push } = fakeApi();
    renderChat(api);
    const input = await screen.findByRole("textbox", { name: "Écris un message…" });
    fireEvent.change(input, { target: { value: "  Salut  " } });
    await act(async () => { fireEvent.click(screen.getByRole("button", { name: "Envoyer" })); });
    expect(sent).toEqual([{ lobbyId: "lobby-1", userId: "host", content: "Salut" }]);
    expect((input as HTMLInputElement).value).toBe("");
    act(() => push(msg("sent-1", "host", "Salut", 50))); // realtime echo of our own message
    expect(screen.getAllByText("Salut")).toHaveLength(1);
  });

  it("does not send a blank message", async () => {
    const { api, sent } = fakeApi();
    renderChat(api);
    const input = await screen.findByRole("textbox", { name: "Écris un message…" });
    fireEvent.change(input, { target: { value: "   " } });
    expect((screen.getByRole("button", { name: "Envoyer" }) as HTMLButtonElement).disabled).toBe(true);
    await act(async () => { fireEvent.submit(input.closest("form")!); });
    expect(sent).toEqual([]);
  });

  it("keeps the draft and logs when sending fails", async () => {
    const onLog = vi.fn();
    renderChat(fakeApi([], "row-level security").api, onLog);
    const input = await screen.findByRole("textbox", { name: "Écris un message…" });
    fireEvent.change(input, { target: { value: "Coucou" } });
    await act(async () => { fireEvent.click(screen.getByRole("button", { name: "Envoyer" })); });
    expect(screen.getByText(/Message non envoyé/)).toBeTruthy();
    expect((input as HTMLInputElement).value).toBe("Coucou");
    expect(onLog).toHaveBeenCalledWith("ERROR", expect.stringContaining("message non envoyé"));
  });

  it("stops listening when the lobby is left", async () => {
    const { api, unsubscribe } = fakeApi();
    const view = renderChat(api);
    await screen.findByText(/Pas encore de message/);
    view.unmount();
    expect(unsubscribe).toHaveBeenCalled();
  });
});

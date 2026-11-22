import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { VersusCard, type VersusView } from "./VersusCard";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

const now = new Date("2026-11-20T12:00:00Z");
const versus = (over: Partial<VersusView> = {}): VersusView => ({
  id: "v1", challenger_id: "me", opponent_id: "bob", challenge_type: "playtime", game_name: null, game_console: null,
  status: "active", created_at: "2026-11-16T10:00:00Z", started_at: "2026-11-16T12:00:00Z", ends_at: "2026-11-23T12:00:00Z",
  duration_days: 7, challenger_progress: 5400, opponent_progress: 3600, winner_id: null, goal_description: "Qui joue le plus en 7 jours ?",
  opponent_profile: { username: "Bob", avatar_url: "" }, ...over,
});
const handlers = () => ({ onAccept: vi.fn(), onDecline: vi.fn(), onRematch: vi.fn() });

describe("versus card", () => {
  it("shows both play times and the time left while it runs", () => {
    render(<VersusCard v={versus()} me="me" t={t} now={now} {...handlers()} />);
    expect(screen.getByText("1 h 30")).toBeTruthy();
    expect(screen.getByText("1 h 00")).toBeTruthy();
    expect(screen.getByText("encore 3 j")).toBeTruthy();
    expect(screen.getByText("Temps de jeu")).toBeTruthy();
  });

  it("lets the opponent accept or decline, and tells the challenger to wait", () => {
    const h = handlers();
    const { rerender } = render(<VersusCard v={versus({ status: "pending", challenger_id: "bob", opponent_id: "me", challenger_profile: { username: "Bob", avatar_url: "" } })} me="me" t={t} now={now} {...h} />);
    fireEvent.click(screen.getByRole("button", { name: "Accepter" }));
    expect(h.onAccept).toHaveBeenCalledWith("v1");
    rerender(<VersusCard v={versus({ status: "pending" })} me="me" t={t} now={now} {...h} />);
    expect(screen.getByText("En attente de Bob")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Accepter" })).toBeNull();
  });

  it("names the winner once closed, and offers a rematch", () => {
    const h = handlers();
    const { rerender } = render(<VersusCard v={versus({ status: "completed", winner_id: "me" })} me="me" t={t} now={now} {...h} />);
    expect(screen.getByText("Tu as gagné ! 🏆")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Revanche/ }));
    expect(h.onRematch).toHaveBeenCalled();
    rerender(<VersusCard v={versus({ status: "completed", winner_id: "bob" })} me="me" t={t} now={now} {...h} />);
    expect(screen.getByText("Bob a gagné")).toBeTruthy();
    rerender(<VersusCard v={versus({ status: "completed", winner_id: null, challenge_type: "launches", challenger_progress: 4, opponent_progress: 4 })} me="me" t={t} now={now} {...h} />);
    expect(screen.getByText("Égalité !")).toBeTruthy();
    expect(screen.getAllByText("4")).toHaveLength(2);
  });
});

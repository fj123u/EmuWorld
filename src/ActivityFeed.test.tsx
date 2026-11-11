import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { ACTIVITY_PAGE, ActivityFeed, type ActivityItem } from "./ActivityFeed";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

const events = (n: number): ActivityItem[] =>
  Array.from({ length: n }, (_, i) => ({
    id: String(i), event_type: "game_started", game_name: `Jeu ${i}`, console: "NES", details: null,
    created_at: new Date().toISOString(), username: "fj123u",
  }));
const rows = (c: HTMLElement) => c.querySelectorAll(".activity-feed__item").length;

describe("recent activity (Social)", () => {
  it("shows a page of events, then more on demand, never the whole feed at once", () => {
    const { container } = render(<ActivityFeed items={events(20)} t={t} />);
    expect(rows(container)).toBe(ACTIVITY_PAGE);
    fireEvent.click(screen.getByRole("button", { name: "Voir 8 de plus" }));
    expect(rows(container)).toBe(16);
    fireEvent.click(screen.getByRole("button", { name: "Voir 4 de plus" }));
    expect(rows(container)).toBe(20);
    expect(screen.queryByRole("button", { name: /de plus/ })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Voir moins" }));
    expect(rows(container)).toBe(ACTIVITY_PAGE);
  });

  it("has no buttons when everything fits, and speaks the app's language", () => {
    const { container } = render(<ActivityFeed items={[{ ...events(1)[0], event_type: "achievement_unlocked", details: "Speedrunner" }]} t={t} />);
    expect(rows(container)).toBe(1);
    expect(screen.queryByRole("button")).toBeNull();
    expect(container.textContent).toContain("fj123u a débloqué Speedrunner");
  });
});

import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { ChallengesPage, timeLeft, type ChallengeItem } from "./ChallengesPage";
import { periodOf, rewards, type Challenge } from "./challenges";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

const now = new Date(2026, 10, 18, 12, 0); // Wednesday 18 November 2026, noon
const day = periodOf("day", now);
const week = periodOf("week", now);
const month = periodOf("month", now);
const ch = (over: Partial<Challenge> & Pick<Challenge, "id" | "rhythm">): Challenge =>
  ({ key: `${over.rhythm}:${over.id}`, kind: "time", family: "common", goal: 3600, points: 30, ...over });

const items: ChallengeItem[] = [
  { challenge: ch({ id: "d", rhythm: "day", kind: "discovery", family: "personal", goal: 1200, points: 10 }), period: day, value: 1200 },
  { challenge: ch({ id: "w1", rhythm: "week", goal: 3 * 3600 }), period: week, value: 4800 },
  { challenge: ch({ id: "w2", rhythm: "week", kind: "distinct", goal: 4 }), period: week, value: 1 },
  { challenge: ch({ id: "w3", rhythm: "week", console: "NES", family: "personal", goal: 7200 }), period: week, value: 0 },
  { challenge: ch({ id: "m", rhythm: "month", kind: "days", goal: 15, minSeconds: 600, points: 100 }), period: month, value: 6 },
];

describe("challenges page", () => {
  it("lists the challenges by rhythm with their progress, nothing to join", () => {
    render(<ChallengesPage items={items} rewards={rewards({}, now)} now={now} t={t} />);
    for (const heading of ["Aujourd'hui", "Cette semaine", "Ce mois-ci"]) expect(screen.getByRole("heading", { name: new RegExp(heading) })).toBeTruthy();
    expect(screen.getByText("Joue 20 min à un jeu que tu n'as jamais lancé")).toBeTruthy();
    expect(screen.getByText("Réussi !")).toBeTruthy();
    expect(screen.getByText("1 h 20 / 3 h 00")).toBeTruthy();
    expect(screen.getByText("1 / 4")).toBeTruthy();
    expect(screen.getByText("Joue 2 h 00 sur NES")).toBeTruthy();
    expect(screen.getByText("6 / 15")).toBeTruthy();
    expect(screen.getAllByText("Perso")).toHaveLength(2);
    expect(screen.queryByRole("button", { name: /Participer|Rejoindre/ })).toBeNull();
    expect(screen.getByText(/Joue : tes défis avancent tout seuls/)).toBeTruthy();
  });

  it("shows points, the streak, badges and the challenges done", () => {
    const history = {
      "S-2026-W46:week_time_3h": { key: "S-2026-W46:week_time_3h", id: "week_time_3h", kind: "time" as const, rhythm: "week" as const, period: "S-2026-W46", points: 30, at: "2026-11-12T10:00:00Z" },
    };
    render(<ChallengesPage items={items} rewards={rewards(history, now)} now={now} t={t} />);
    expect(screen.getByText("30 points")).toBeTruthy();
    expect(screen.getByText("Série : 1 semaine(s)")).toBeTruthy();
    expect(screen.getAllByText("Temps de jeu").length).toBeGreaterThan(0);
    expect(screen.getByText(/Bronze · 1 réussi/)).toBeTruthy();
    expect(screen.getByText("+30")).toBeTruthy();
  });

  it("says how long is left, in days or hours", () => {
    expect(timeLeft(week, now, t)).toBe("encore 5 j");
    expect(timeLeft(day, now, t)).toBe("encore 12 h");
  });
});

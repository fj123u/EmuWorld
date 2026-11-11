import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Dock, PAGE_LABEL_KEYS, SECTIONS, SectionTabs, TAB_PAGES, adjacentTabPage, sectionOf, type Page } from "./navigation";
import fr from "./i18n/fr";

const t = (key: string) => key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr) as string ?? key;

describe("v3 navigation", () => {
  it("puts every page in exactly one section", () => {
    const all = SECTIONS.flatMap((s) => s.pages);
    expect(new Set(all).size).toBe(all.length);
    expect(all).toHaveLength(17);
    // The 15 pages of the old sidebar and the Lobbies page have a tab (account opens as a dialog).
    expect(TAB_PAGES).toHaveLength(16);
    for (const p of TAB_PAGES) expect(fr.nav[PAGE_LABEL_KEYS[p as Exclude<Page, "account">].split(".")[1] as keyof typeof fr.nav], p).toBeTruthy();
    expect(sectionOf("installed")).toBe("games");
    expect(sectionOf("account")).toBe("settings");
  });

  it("shows the 6 sections, names only the open one, and opens a section's first page", () => {
    const navigate = vi.fn();
    render(<Dock page="installed" navigate={navigate} t={t} badges={{ social: 2 }} />);
    const items = screen.getAllByRole("button");
    expect(items.map((b) => b.getAttribute("aria-label"))).toEqual(["Accueil", "Jeux", "Store", "Social", "Succès", "Réglages"]);
    const games = screen.getByRole("button", { name: "Jeux" });
    expect(games.getAttribute("aria-current")).toBe("page");
    expect(items.every((b) => b.classList.contains("gamepad-nav-item"))).toBe(true);
    expect(screen.getByRole("button", { name: "Social" }).textContent).toContain("2");
    fireEvent.click(screen.getByRole("button", { name: "Social" }));
    expect(navigate).toHaveBeenCalledWith("lobby");
  });

  it("shows the tabs of the open section with their counts, none for Home", () => {
    const navigate = vi.fn();
    const { rerender } = render(<SectionTabs page="catalog" navigate={navigate} t={t} counts={{ library: 4 }} />);
    expect(screen.getAllByRole("tab").map((b) => b.textContent)).toEqual(["Roms4", "Console", "Installé"]);
    expect(screen.getByRole("tab", { name: "Console" }).getAttribute("aria-selected")).toBe("true");
    fireEvent.click(screen.getByRole("tab", { name: "Installé" }));
    expect(navigate).toHaveBeenCalledWith("installed");
    rerender(<SectionTabs page="discover" navigate={navigate} t={t} />);
    expect(screen.queryAllByRole("tab")).toHaveLength(0);
  });
});

describe("LB / RB", () => {
  it("follow the tabs of the bar, across sections", () => {
    expect(adjacentTabPage("library", 1)).toBe("catalog"); // Roms → Console
    expect(adjacentTabPage("catalog", 1)).toBe("installed");
    expect(adjacentTabPage("store", 1)).toBe("marketplace"); // not Settings
    expect(adjacentTabPage("marketplace", 1)).toBe("lobby");
    expect(adjacentTabPage("catalog", -1)).toBe("library");
    expect(adjacentTabPage("discover", -1)).toBe("discover");
    expect(adjacentTabPage("changelogs", 1)).toBe("changelogs");
    expect(adjacentTabPage("account", 1)).toBe("controller"); // the account dialog is in Settings
  });
});

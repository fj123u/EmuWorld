// v3 navigation: a pill bar with 6 sections (double-outlined icons) and, in
// each section, tabs for its pages. Started from scratch/ui-v3/SectionIcon.tsx.
// `Page` and setPage keep working as before: only the way to reach a page
// changes. Every button keeps `gamepad-nav-item` and a `data-tour` target.
import type { CSSProperties } from "react";

/** Every page of the app (main content). */
export type Page =
  | "discover" | "catalog" | "library" | "installed" | "settings" | "changelogs"
  | "account" | "store" | "controller" | "backup" | "leaderboard" | "stats"
  | "friends" | "challenges" | "wrap" | "marketplace" | "lobby";

export type Section = "home" | "games" | "store" | "social" | "trophy" | "settings";

/**
 * The 6 sections and their pages. A click on a section opens its first page;
 * the pages become tabs at the top of the section. "account" has no page of
 * its own (the account opens as a dialog): it belongs to Settings, no tab.
 */
export const SECTIONS: { id: Section; labelKey: string; pages: Page[] }[] = [
  { id: "home", labelKey: "nav.home", pages: ["discover"] },
  { id: "games", labelKey: "nav.games", pages: ["library", "catalog", "installed"] },
  { id: "store", labelKey: "nav.store", pages: ["store", "marketplace"] },
  { id: "social", labelKey: "nav.social", pages: ["lobby", "friends", "leaderboard", "challenges"] },
  { id: "trophy", labelKey: "nav.trophies", pages: ["stats", "wrap"] },
  { id: "settings", labelKey: "nav.settings", pages: ["settings", "controller", "backup", "changelogs", "account"] },
];

/** Pages that have a tab (all but "account"). */
export const TAB_PAGES: Page[] = SECTIONS.flatMap((s) => s.pages).filter((p) => p !== "account");

/**
 * LB / RB: the previous / next of the 6 sections (its first page); stays on
 * the first and last. The account dialog belongs to Settings.
 */
export function adjacentSectionPage(page: Page, step: -1 | 1): Page {
  const idx = SECTIONS.findIndex((s) => s.id === sectionOf(page));
  const next = SECTIONS[idx + step];
  return next ? next.pages[0] : page;
}

/**
 * LB / RB with the pad on the tabs: the previous / next tab of the open
 * section (Roms → Console → Installé), never leaving it.
 */
export function adjacentTabInSection(page: Page, step: -1 | 1): Page {
  const pages = SECTIONS.find((s) => s.id === sectionOf(page))!.pages.filter((p) => p !== "account");
  const idx = pages.indexOf(page === "account" ? "settings" : page);
  return pages[idx + step] ?? page;
}

/** Label of each page's tab. */
export const PAGE_LABEL_KEYS: Record<Exclude<Page, "account">, string> = {
  discover: "nav.discover",
  library: "nav.roms",
  catalog: "nav.console",
  installed: "nav.installed",
  store: "nav.store",
  marketplace: "nav.marketplace",
  lobby: "nav.lobbies",
  friends: "nav.friends",
  challenges: "nav.challenges",
  leaderboard: "nav.leaderboard",
  stats: "nav.stats",
  wrap: "nav.monthlyWrap",
  settings: "nav.settings",
  controller: "nav.controller",
  backup: "nav.backup",
  changelogs: "nav.changelogs",
};

/** Section highlighted in the bar for a page. */
export function sectionOf(page: Page): Section {
  return SECTIONS.find((s) => s.pages.includes(page))?.id ?? "home";
}

/** Outline (--k) and pastel fill (--f) of each section: tokens of App.css. */
const COLORS: Record<Section, { k: string; f: string }> = {
  home: { k: "var(--sec-home)", f: "var(--sec-home-fill)" },
  games: { k: "var(--sec-games)", f: "var(--sec-games-fill)" },
  store: { k: "var(--sec-store)", f: "var(--sec-store-fill)" },
  social: { k: "var(--sec-social)", f: "var(--sec-social-fill)" },
  trophy: { k: "var(--sec-trophy)", f: "var(--sec-trophy-fill)" },
  settings: { k: "var(--sec-settings)", f: "var(--sec-settings-fill)" },
};

/** Shape of each icon: `shape` is drawn twice (outline, then fill). */
const SHAPES: Record<Section, { shape: string; details?: string; under?: string }> = {
  home: { shape: "M6 15L16 6l10 9v10a1 1 0 0 1-1 1h-6v-7h-6v7H7a1 1 0 0 1-1-1z" },
  games: {
    shape: "M10 11h12a6 6 0 0 1 0 12H10a6 6 0 0 1 0-12z",
    details: "M11 15v4M9 17h4M21 16h.01M23 18.5h.01",
  },
  store: { shape: "M8 12h16l-1.5 14h-13z", under: "M12.5 12v-2a3.5 3.5 0 0 1 7 0v2" },
  social: { shape: "M16 6a4.5 4.5 0 1 1 0 9 4.5 4.5 0 0 1 0-9zM7 26a9 9 0 0 1 18 0z" },
  trophy: { shape: "M10 6h12v7a6 6 0 0 1-4 5.6V22h3v4H11v-4h3v-3.4A6 6 0 0 1 10 13z" },
  settings: {
    shape: "M16 6a10 10 0 1 1 0 20 10 10 0 0 1 0-20z",
    details: "M16 12.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7z",
  },
};

/** Section icon with a double outline: pastel fill, white edge, colored outline. */
export function SectionIcon({ section, size = 34 }: { section: Section; size?: number }) {
  const { shape, details, under } = SHAPES[section];
  const { k, f } = COLORS[section];
  return (
    <svg
      className="v3-icon"
      aria-hidden="true"
      viewBox="0 0 32 32"
      style={{ "--k": k, "--f": f, width: size, height: size } as CSSProperties}
    >
      {under && <path className="d" d={under} style={{ strokeWidth: 5 }} />}
      <path className="o" d={shape} />
      <path className="i" d={shape} />
      {details && <path className="d" d={details} />}
    </svg>
  );
}

type T = (key: string, params?: Record<string, string | number>) => string;

/** Where the guided tour points for each section (data-tour). */
const SECTION_TOUR: Partial<Record<Section, string>> = { games: "library", store: "store", social: "friends" };

/** The bar of 6 sections. The name only shows on the open section. */
export function Dock({ page, navigate, t, badges = {} }: {
  page: Page;
  navigate: (p: Page) => void;
  t: T;
  badges?: Partial<Record<Section, number>>;
}) {
  const current = sectionOf(page);
  return (
    <nav className="v3-dock" aria-label={t("nav.navigation")}>
      {SECTIONS.map((s) => {
        const active = s.id === current;
        const badge = badges[s.id] ?? 0;
        return (
          <button
            key={s.id}
            className={`v3-dock__item gamepad-nav-item${active ? " active" : ""}`}
            data-section={s.id}
            data-tour={SECTION_TOUR[s.id]}
            aria-label={t(s.labelKey)}
            aria-current={active ? "page" : undefined}
            onClick={() => navigate(s.pages[0])}
          >
            <SectionIcon section={s.id} />
            <span className="v3-dock__label">{t(s.labelKey)}</span>
            {badge > 0 && <span className="v3-dock__badge">{badge}</span>}
          </button>
        );
      })}
    </nav>
  );
}

/** Tabs of the open section (nothing for a one-page section). */
export function SectionTabs({ page, navigate, t, counts = {} }: {
  page: Page;
  navigate: (p: Page) => void;
  t: T;
  counts?: Partial<Record<Page, number>>;
}) {
  const section = SECTIONS.find((s) => s.id === sectionOf(page))!;
  const pages = section.pages.filter((p): p is Exclude<Page, "account"> => p !== "account");
  if (pages.length < 2) return null;
  return (
    <div className="v3-tabs" role="tablist" aria-label={t(section.labelKey)}>
      {pages.map((p) => (
        <button
          key={p}
          role="tab"
          aria-selected={p === page}
          className={`v3-tabs__item gamepad-nav-item${p === page ? " active" : ""}`}
          data-nav-page={p}
          data-tour={p === "catalog" ? "emulators" : undefined}
          onClick={() => navigate(p)}
        >
          {t(PAGE_LABEL_KEYS[p])}
          {counts[p] !== undefined && <span className="v3-tabs__count">{counts[p]}</span>}
        </button>
      ))}
    </div>
  );
}

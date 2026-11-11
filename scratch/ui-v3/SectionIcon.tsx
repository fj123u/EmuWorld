// EmuWorld v3 — navigation à 6 destinations (étape 3 de la refonte).
// À déplacer dans src/ au moment de coder la navigation. Rien ici n'est
// encore branché : c'est un point de départ, à tester dans App.test.tsx.

import type { CSSProperties } from "react";

/** Pages actuelles de l'app (copie du type `Page` de App.tsx, ligne 457). */
export type Page =
  | "discover" | "catalog" | "library" | "installed" | "settings" | "changelogs"
  | "account" | "store" | "controller" | "backup" | "leaderboard" | "stats"
  | "friends" | "challenges" | "wrap" | "marketplace";

export type Section = "home" | "games" | "store" | "social" | "trophy" | "settings";

/**
 * Les 6 destinations de la barre en pilule, et les pages que chacune regroupe.
 * La première page de la liste est celle qu'ouvre un clic sur l'icône ; les
 * autres deviennent des onglets (`.v3-tabs`) en haut de la section.
 * Le type `Page` et `setPage` ne changent pas : seul l'habillage change.
 */
export const SECTIONS: { id: Section; labelKey: string; pages: Page[] }[] = [
  { id: "home", labelKey: "nav.home", pages: ["discover"] },
  { id: "games", labelKey: "nav.games", pages: ["library", "catalog", "installed"] },
  { id: "store", labelKey: "nav.store", pages: ["store", "marketplace"] },
  { id: "social", labelKey: "nav.social", pages: ["friends", "challenges", "leaderboard"] },
  { id: "trophy", labelKey: "nav.trophies", pages: ["stats", "wrap"] },
  { id: "settings", labelKey: "nav.settings", pages: ["settings", "controller", "backup", "changelogs", "account"] },
];

/** Section à surligner dans la barre pour une page donnée. */
export function sectionOf(page: Page): Section {
  return SECTIONS.find((s) => s.pages.includes(page))?.id ?? "home";
}

/** Contour (--k) et remplissage pastel (--f) de chaque section : jetons de tokens.css. */
const COLORS: Record<Section, { k: string; f: string }> = {
  home: { k: "var(--sec-home)", f: "var(--sec-home-fill)" },
  games: { k: "var(--sec-games)", f: "var(--sec-games-fill)" },
  store: { k: "var(--sec-store)", f: "var(--sec-store-fill)" },
  social: { k: "var(--sec-social)", f: "var(--sec-social-fill)" },
  trophy: { k: "var(--sec-trophy)", f: "var(--sec-trophy-fill)" },
  settings: { k: "var(--sec-settings)", f: "var(--sec-settings-fill)" },
};

/** Forme de chaque icône : `shape` est dessinée deux fois (contour puis remplissage). */
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

/** Icône de section à double contour : remplissage pastel, liseré blanc, contour coloré. */
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

/**
 * Barre de navigation. Exemple d'usage dans App.tsx :
 *   <Dock page={page} setPage={setPage} t={t} badges={{ social: pendingRequests }} />
 * Chaque bouton garde `gamepad-nav-item` (navigation manette) et un
 * `data-tour` pour le tutoriel.
 */
export function Dock({
  page,
  setPage,
  t,
  badges = {},
}: {
  page: Page;
  setPage: (p: Page) => void;
  t: (key: string) => string;
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
            data-tour={`section-${s.id}`}
            aria-label={t(s.labelKey)}
            aria-current={active ? "page" : undefined}
            onClick={() => setPage(s.pages[0])}
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

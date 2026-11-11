// Themes since v3: light ("default", no data-theme) and dark ("dark") are the
// two base themes offered in Settings. The five themes from before v3 are
// variants of the dark one, offered by the marketplace; their ids stay valid
// because community themes saved them as `base_theme` in Supabase.
// Colors live in App.css ([data-theme] and [data-theme="<variant>"]).

export const BASE_THEMES = ["default", "dark"] as const;

export interface ThemeVariant {
  id: string;
  /** Accent and panel, for the swatches of the marketplace cards. */
  accent: string;
  panel: string;
}

export const DARK_VARIANTS: ThemeVariant[] = [
  { id: "midnight", accent: "#5b9bf6", panel: "#1a2332" },
  { id: "oled", accent: "#8E9BF5", panel: "#0a0a0a" },
  { id: "retro", accent: "#4ade80", panel: "#0f240f" },
  { id: "sakura", accent: "#f472b6", panel: "#251a22" },
  { id: "sunset", accent: "#fbbf24", panel: "#261a0e" },
];

/** Light unless a data-theme is set: every other theme is dark. */
export function isDarkTheme(id: string): boolean {
  return id !== "default";
}

/** i18n key of a theme's name; an unknown id (newer app, typo) reads as "dark". */
export function themeNameKey(id: string): string {
  if (id === "default") return "themes.light";
  if (id === "dark" || DARK_VARIANTS.some((v) => v.id === id)) return `themes.${id}`;
  return "themes.dark";
}

/** Accent swatch of a theme (marketplace cards). */
export function themeSwatch(id: string): string {
  if (id === "default") return "#6573E6";
  return DARK_VARIANTS.find((v) => v.id === id)?.accent ?? "#8E9BF5";
}

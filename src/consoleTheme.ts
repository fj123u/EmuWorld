// Per-console accent colours: while browsing one console, the app's accent
// takes a hue inspired by that console's branding (Switch red, PlayStation
// blue, Xbox green…). Only the hue changes; saturation and lightness stay
// those of the custom-accent palette, so every theme stays readable.

export const CONSOLE_COLORS_KEY = "emuworld_console_colors";

/** Exact catalog console names (src-tauri/src/emulators.rs) → hue in degrees. */
const CONSOLE_HUES: Record<string, number> = {
  // Nintendo
  "NES": 0,
  "Super Nintendo": 265,
  "Nintendo 64": 140,
  "GameCube / Wii": 255,
  "Wii U": 195,
  "Nintendo Switch": 355,
  "Game Boy Advance": 245,
  "Nintendo DS": 210,
  "Nintendo 3DS": 0,
  "Virtual Boy": 350,
  // Sony
  "PlayStation 1": 220,
  "PlayStation 2": 228,
  "PlayStation 3": 212,
  "PlayStation 4": 222,
  "PlayStation 5": 200,
  "PlayStation Portable": 205,
  // Microsoft
  "Xbox": 120,
  "Xbox 360": 95,
  // Sega
  "Master System": 0,
  "Mega Drive": 215,
  "Sega CD": 215,
  "Sega 32X": 10,
  "Saturn": 232,
  "Dreamcast": 25,
  "Game Gear": 220,
  // Others
  "Atari 2600": 20,
  "Atari 5200": 20,
  "Atari 7800": 20,
  "Jaguar": 0,
  "Lynx": 30,
  "Neo-Geo": 45,
  "Arcade": 285,
  "PC Engine": 28,
  "TurboGrafx-16": 28,
  "TurboGrafx-CD": 28,
  "CD-i": 205,
  "WonderSwan": 195,
};

/** Hue for a console, or `null` when it has no colour of its own (e.g. "Multi-System"). */
export function consoleHue(consoleName: string | null | undefined): number | null {
  if (!consoleName) return null;
  return CONSOLE_HUES[consoleName] ?? null;
}

export function consoleColorsEnabled(): boolean {
  try {
    return localStorage.getItem(CONSOLE_COLORS_KEY) !== "off";
  } catch {
    return true;
  }
}

/**
 * Applies (or clears) the console accent on the document root. It sits on
 * top of the theme and the custom accent, which come back as soon as it is
 * cleared.
 */
export function applyConsoleAccent(root: HTMLElement, hue: number | null) {
  if (hue === null) {
    root.removeAttribute("data-console-accent");
    root.style.removeProperty("--console-hue");
  } else {
    root.setAttribute("data-console-accent", "");
    root.style.setProperty("--console-hue", String(hue));
  }
}

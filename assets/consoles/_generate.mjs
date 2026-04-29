// Generate console icon SVGs for Discord Rich Presence.
// Each icon = circular gradient background tinted to the constructor's
// brand colour + a short glyph (letters or wordmark).
// Keys match the Rust mapping in src-tauri/src/discord_rpc.rs.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));

// bg1 = top/highlight colour, bg2 = bottom colour
const CONSOLES = [
  // Nintendo
  { key: "console_switch",    bg1: "#ff6b6b", bg2: "#c13a3a", glyph: "NS",  sub: "Switch" },
  { key: "console_wiiu",      bg1: "#4a8ac6", bg2: "#1e4d7c", glyph: "U",   sub: "Wii U",    big: true },
  { key: "console_wii",       bg1: "#e8e8e8", bg2: "#a8a8a8", glyph: "Wii", wordmark: true },
  { key: "console_gamecube",  bg1: "#7e5fa8", bg2: "#3f2d5e", glyph: "GC",  sub: "GameCube" },
  { key: "console_3ds",       bg1: "#f57a7a", bg2: "#a02929", glyph: "3DS", wordmark: true },
  { key: "console_ds",        bg1: "#9ba7b6", bg2: "#556374", glyph: "DS",  sub: "Nintendo" },
  { key: "console_n64",       bg1: "#6fc47a", bg2: "#2d7a38", glyph: "64",  sub: "Nintendo" },
  { key: "console_snes",      bg1: "#b89bf0", bg2: "#6a4cb8", glyph: "SN",  sub: "Super NES" },
  { key: "console_nes",       bg1: "#d45a5a", bg2: "#7e2626", glyph: "NES", wordmark: true },
  { key: "console_gba",       bg1: "#7fbdff", bg2: "#3770b8", glyph: "GBA", wordmark: true },
  { key: "console_gbc",       bg1: "#ffd866", bg2: "#b8893a", glyph: "GBC", wordmark: true },
  { key: "console_gb",        bg1: "#c6d0b6", bg2: "#7a8466", glyph: "GB",  sub: "Game Boy" },
  // Sony
  { key: "console_ps5",       bg1: "#ffffff", bg2: "#a0a7b2", glyph: "PS5", wordmark: true, darkGlyph: true },
  { key: "console_ps4",       bg1: "#4e8fdc", bg2: "#1f3f78", glyph: "PS4", wordmark: true },
  { key: "console_ps3",       bg1: "#5a5a5a", bg2: "#141414", glyph: "PS3", wordmark: true },
  { key: "console_ps2",       bg1: "#3e5bcc", bg2: "#0f1a5a", glyph: "PS2", wordmark: true },
  { key: "console_ps1",       bg1: "#b5b9c0", bg2: "#55595f", glyph: "PS",  sub: "PlayStation", darkGlyph: true },
  { key: "console_psp",       bg1: "#6d6d6d", bg2: "#1a1a1a", glyph: "PSP", wordmark: true },
  // Microsoft
  { key: "console_xbox",      bg1: "#8ad34a", bg2: "#2d6613", glyph: "X",   sub: "Xbox",    big: true },
  // Sega
  { key: "console_dreamcast", bg1: "#ff9560", bg2: "#c24420", glyph: "DC",  sub: "Dreamcast" },
  { key: "console_saturn",    bg1: "#6a7898", bg2: "#2a3448", glyph: "Sat", wordmark: true },
  { key: "console_megadrive", bg1: "#4a7ac6", bg2: "#1a3a7a", glyph: "MD",  sub: "Mega Drive" },
];

const svgFor = ({ key, bg1, bg2, glyph, sub, wordmark, big, darkGlyph }) => {
  const textColor = darkGlyph ? "#1e1f22" : "#ffffff";
  const glyphFs = big ? 260 : (wordmark ? 170 : 220);
  const subColor = darkGlyph ? "rgba(30,31,34,0.65)" : "rgba(255,255,255,0.7)";
  const gradId = "g_" + key;
  const hiId = "h_" + key;
  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <defs>
    <radialGradient id="${gradId}" cx="0.35" cy="0.30" r="0.85">
      <stop offset="0" stop-color="${bg1}"/>
      <stop offset="1" stop-color="${bg2}"/>
    </radialGradient>
    <linearGradient id="${hiId}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.25"/>
      <stop offset="0.45" stop-color="#ffffff" stop-opacity="0"/>
    </linearGradient>
  </defs>
  <circle cx="256" cy="256" r="248" fill="url(#${gradId})"/>
  <circle cx="256" cy="256" r="248" fill="url(#${hiId})"/>
  <circle cx="256" cy="256" r="246" fill="none" stroke="rgba(0,0,0,0.25)" stroke-width="3"/>
  <circle cx="256" cy="256" r="244" fill="none" stroke="rgba(255,255,255,0.10)" stroke-width="2"/>
  <text x="256" y="${sub ? 260 : 290}" text-anchor="middle"
        font-family="Nunito, Arial Black, sans-serif"
        font-size="${glyphFs}" font-weight="900"
        fill="${textColor}"
        style="paint-order: stroke; stroke: rgba(0,0,0,0.20); stroke-width: ${darkGlyph ? 0 : 3}">${glyph}</text>
  ${sub ? `<text x="256" y="380" text-anchor="middle"
        font-family="Nunito, Arial, sans-serif"
        font-size="52" font-weight="800" letter-spacing="2"
        fill="${subColor}">${sub.toUpperCase()}</text>` : ""}
</svg>
`;
};

for (const c of CONSOLES) {
  fs.writeFileSync(path.join(here, `${c.key}.svg`), svgFor(c));
}
console.log(`Generated ${CONSOLES.length} SVGs in ${here}`);

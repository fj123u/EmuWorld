# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

EmuWorld is a Tauri v2 desktop application (Windows-first) that serves as an all-in-one emulator launcher. It manages emulator installation, ROM library scanning, box art fetching, playtime tracking, Discord Rich Presence, and in-app auto-updates. Authentication and profiles are backed by Supabase.

## Development Commands

```bash
# Frontend dev server (Vite on port 1420)
npm run dev

# Full Tauri app in dev mode (frontend + Rust backend)
npm run tauri dev

# Production build (frontend + Rust → NSIS installer)
npm run tauri build

# TypeScript type-check only
npx tsc --noEmit

# Clean build caches (cargo + vite)
npm run clean
```

## Architecture

**Frontend** (`src/`): Single-page React 19 app in one large `App.tsx` file. Uses Framer Motion for animations, Lucide for icons, and Supabase JS client for auth/profiles. No router — the UI uses view state (`library`, `store`, `emulators`, `settings`, `profile`). Custom titlebar (decorations: false).

**Backend** (`src-tauri/src/`):
- `lib.rs` — All Tauri commands (30+), config management, ROM scanning, box art fetching (libretro thumbnails, tinfoil.media, web scraping), emulator install/launch, ROM store scraping (Myrient, Vimm's Lair, RGS). This is a ~2800-line monolith.
- `emulators.rs` — Static catalog of supported emulators with download URLs, executable names, supported extensions, and archive types.
- `playtime.rs` — JSON-file-based playtime tracking (per-game seconds, launches, favorites).
- `discord_rpc.rs` — Discord IPC integration for rich presence status.

**Data storage**: All user data lives in `%LOCALAPPDATA%/EmuWorld/` — `config.json`, `playtime.json`, covers cache, emulator installs, ROMs directory.

**IPC pattern**: Frontend calls `invoke("command_name", { args })` → Rust `#[tauri::command]` functions. Progress/events flow back via `app_handle.emit()` + frontend `listen()`.

**Plugins used**: dialog, fs, shell, deep-link (emuworld:// scheme for OAuth callback), single-instance, updater, process, opener.

## CI/CD

GitHub Actions workflow (`.github/workflows/release.yml`) triggers on `v*` tags. Builds Windows NSIS installer, signs with `TAURI_SIGNING_PRIVATE_KEY` secret, uploads to GitHub Release with `latest.json` for the in-app updater.

## Key Conventions

- The app locale is French (date formatting, UI labels).
- Emulator IDs are always lowercase and match folder names on disk.
- ROM detection uses file extensions matched against the emulator catalog; updates/DLCs are filtered by title ID patterns (Switch: non-000 suffix, Wii U: 0005000E/C prefix).
- Box art fetching cascades: local cache → tinfoil.media (Switch) → libretro thumbnails (raw then stripped name) → web scraping fallbacks.
- The Rust backend uses `reqwest` for HTTP, `zip`/`sevenz-rust` for archive extraction, `scraper` for HTML parsing, `walkdir` for recursive FS scans.

## Workflow After Each Feature/Fix

1. **Commit + push** immediately after implementing
2. **Check off** the task in `scratch/plan.md`
3. **Propose next ideas** — suggest 3-4 options for what to work on next (from plan.md or new ideas)
4. Always include the `Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>` trailer

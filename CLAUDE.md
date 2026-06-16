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

## Project Structure

```
EmuWorld/
├── src/                          # Frontend React (single-page app)
│   ├── App.tsx                   # Composant monolithique (~8000 lignes) — toute l'UI, state, logique
│   ├── App.css                   # Design system complet (thèmes, glassmorphism, responsive, animations)
│   ├── main.tsx                  # Point d'entrée React (monte <App/> dans #root)
│   ├── supabase.ts               # Client Supabase initialisé (auth, DB, storage)
│   ├── vite-env.d.ts             # Types Vite
│   └── i18n/                     # Internationalisation
│       ├── index.ts              # Hook useTranslation + détection locale
│       ├── fr.ts                 # Traductions françaises
│       └── en.ts                 # Traductions anglaises
├── src-tauri/                    # Backend Rust (Tauri v2)
│   ├── src/
│   │   ├── lib.rs                # Monolithe principal (~4500 lignes) — toutes les commandes Tauri :
│   │   │                         #   scan ROMs, fetch covers (libretro/tinfoil/Wikipedia/GameTDB),
│   │   │                         #   install/launch émulateurs, store (Vimm/RGS), config, overlay,
│   │   │                         #   screenshots, bandwidth limiter, guides Wikipedia/RA
│   │   ├── emulators.rs          # Catalogue statique des 25+ émulateurs (URLs, exécutables, extensions)
│   │   ├── playtime.rs           # Tracking JSON local (secondes/jeu, launches, favoris, notes, étoiles)
│   │   ├── discord_rpc.rs        # Discord IPC — Rich Presence (jeu en cours, idle, logo)
│   │   ├── achievements.rs       # 33 achievements internes (milestones + cachés) + sync Supabase
│   │   ├── cloud_backup.rs       # Scan saves émulateurs + upload/download Supabase Storage
│   │   ├── retroachievements.rs  # Wrapper API retroachievements.org + injection token RetroArch
│   │   ├── gamepad.rs            # Config manette persistante (remapping, deadzone)
│   │   └── main.rs               # Entry point Rust (juste le bootstrap Tauri)
│   ├── Cargo.toml                # Dépendances Rust (reqwest, scraper, image, webp, gilrs, zip, etc.)
│   └── tauri.conf.json           # Config fenêtre, plugins activés, updater endpoint, bundle NSIS
├── web-panel/                    # Pages web statiques hébergées sur alwaysdata
│   ├── index.html                # Profil public (stats, achievements, bibliothèque)
│   └── auth-callback.html        # Callback OAuth Google/Discord → ferme l'onglet
├── .github/workflows/
│   └── release.yml               # CI: build Windows NSIS + sign + upload GitHub Release sur tag v*
├── scratch/                      # Notes de dev, SQL schemas Supabase, plans
│   ├── plan.md                   # Roadmap features (checklist)
│   ├── release_guide.md          # Guide pour créer une release
│   ├── supabase_*.sql            # Schemas SQL pour les tables Supabase
│   └── ...
├── EmuWorld.bat                  # Script batch qui lance `npm run tauri dev`
├── index.html                    # HTML racine Vite (monte /src/main.tsx)
├── package.json                  # Scripts npm, dépendances frontend
├── vite.config.ts                # Config Vite (port 1420, watch src-tauri)
├── tsconfig.json                 # Config TypeScript
└── CLAUDE.md                     # Ce fichier
```

## Architecture

**Frontend** (`src/`): Single-page React 19 app dans un seul fichier `App.tsx`. Utilise Framer Motion pour les animations, Lucide pour les icônes, et le client Supabase JS pour auth/profiles. Pas de routeur — l'UI utilise un state `page` (`library`, `store`, `emulators`, `settings`, `friends`, `stats`, `challenges`, etc.). Custom titlebar (decorations: false).

**Backend** (`src-tauri/src/`):
- `lib.rs` — Toutes les commandes Tauri (50+), config management, ROM scanning, box art fetching (libretro thumbnails, tinfoil.media, Wikipedia, GameTDB), emulator install/launch, ROM store (Vimm's Lair, RGS), overlay, guides, bandwidth limiter. C'est un monolithe ~4500 lignes.
- `emulators.rs` — Catalogue statique des émulateurs supportés avec URLs de download, noms d'exécutables, extensions supportées, types d'archives, et setup_files (firmware, cores).
- `playtime.rs` — Tracking JSON-file-based (secondes par jeu, launches, favoris, notes texte, étoiles 1-5).
- `discord_rpc.rs` — Discord IPC pour Rich Presence (jeu en cours avec nom + console, ou idle).
- `achievements.rs` — 33 succès internes avec détection temps réel + sync cloud.
- `cloud_backup.rs` — Scan des dossiers saves de chaque émulateur, zip, upload/download Supabase Storage.
- `retroachievements.rs` — API retroachievements.org, injection credentials dans RetroArch config.
- `gamepad.rs` — Persistance de la config manette (remapping boutons, deadzone).

**Data storage**: Toutes les données utilisateur dans `%LOCALAPPDATA%/EmuWorld/` — `config.json`, `playtime.json`, cache covers (WebP), émulateurs installés, dossier ROMs.

**IPC pattern**: Frontend appelle `invoke("command_name", { args })` → fonctions Rust `#[tauri::command]`. Les événements de progress reviennent via `app_handle.emit()` + `listen()` frontend.

**Plugins Tauri**: dialog, fs, shell, deep-link (emuworld:// pour OAuth), single-instance, updater, process, opener, notification, global-shortcut.

## CI/CD

GitHub Actions workflow (`.github/workflows/release.yml`) se déclenche sur les tags `v*`. Build l'installeur Windows NSIS, signe avec `TAURI_SIGNING_PRIVATE_KEY`, upload sur GitHub Release avec `latest.json` pour l'auto-updater in-app.

## Key Conventions

- L'app est en français par défaut (dates, labels UI), avec support EN.
- Les IDs d'émulateurs sont toujours en lowercase et correspondent aux noms de dossiers sur disque.
- La détection de ROMs utilise les extensions fichier matchées contre le catalogue + le nom de dossier parent (avec alias : "PSP" → "PlayStation Portable", "3DS" → "Nintendo 3DS", etc.).
- Les updates/DLCs sont filtrés par pattern de Title ID (Switch: suffixe non-000, Wii U: préfixe 0005000E/C).
- Le fetch de covers cascade: cache local (WebP) → tinfoil.media (Switch) → libretro thumbnails → Wikipedia → GameTDB.
- Les covers sont sauvegardées en WebP (quality 85) pour réduire l'espace disque (~80% vs PNG).
- Le backend Rust utilise `reqwest` pour HTTP, `zip`/`sevenz-rust` pour l'extraction, `scraper` pour le parsing HTML, `walkdir` pour les scans FS récursifs, `image`/`webp` pour la conversion d'images.

## Mandatory Rules

- **i18n** : Every visible text string MUST use `t("section.key")` — never hardcode French or English. Add keys to both `src/i18n/fr.ts` and `src/i18n/en.ts`.
- **Gamepad** : Every new interactive element (button, input, select, card) MUST have the CSS class `gamepad-nav-item` or be a `.btn` for controller navigation.
- **Logs** : Every new feature or command MUST include `push_log("INFO"|"WARN"|"ERROR", ...)` calls at entry, on errors, and on completion. This enables debugging via the daily log files in `%LOCALAPPDATA%/EmuWorld/logs/`.

## Workflow After Each Feature/Fix

1. **Commit + push** immediately after implementing
2. **Check off** the task in `scratch/plan.md`
3. **Propose next ideas** — suggest 3-4 options for what to work on next (from plan.md or new ideas)
4. Always include the `Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>` trailer

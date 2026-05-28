# EmuWorld

Lanceur d'émulateurs tout-en-un pour Windows. Télécharge, installe et lance tes émulateurs et tes jeux rétro depuis une interface moderne.

![Tauri](https://img.shields.io/badge/Tauri_v2-24C8D8?logo=tauri&logoColor=white)
![React](https://img.shields.io/badge/React_19-61DAFB?logo=react&logoColor=black)
![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)
![License](https://img.shields.io/badge/License-MIT-green)

## Fonctionnalités

**Emulation**
- 25+ émulateurs supportés (NES, SNES, N64, GameCube, Wii, Wii U, Switch, PS1, PS2, PS3, PSP, 3DS, DS, GBA, Mega Drive, Dreamcast, Xbox, Virtual Boy...)
- Installation en un clic depuis les sources officielles
- Lancement automatique en plein écran
- Détection auto des ROMs par dossier + extension

**Bibliothèque**
- Covers auto-téléchargées (libretro, tinfoil, Wikipedia, GameTDB) en WebP
- Vue grille / liste avec tri et filtres avancés
- Notes, étoiles, favoris, collections custom
- Fond d'écran dynamique (cover du jeu survolé en blur)

**Social & Stats**
- Système d'amis (chat, activity feed, comparaison)
- Playtime tracking par jeu avec streaks et heatmap
- Leaderboard hebdo + challenges rotatifs
- RetroAchievements intégré (progression par jeu)
- Wrap mensuel animé (top jeux, consoles, tendances)
- Avis communauté et guides intégrés par jeu
- Profil public partageable

**Overlay in-game**
- Fenêtre transparente par-dessus le jeu (Shift+Tab)
- RetroAchievements, chat amis, notes persistantes

**Technique**
- Discord Rich Presence
- Backup cloud des saves (Supabase Storage)
- Auto-update intégré (NSIS + signing)
- Mode Big Picture (navigation 100% manette)
- Mode portable (USB friendly)
- Import/Export config
- Multi-langue (FR/EN)
- Bandwidth limiter dynamique
- Notifications Windows natives

## Quick Start

**Double-clic sur `EmuWorld.bat`** ou :

```bash
npm install
npm run tauri dev
```

### Prérequis

- [Node.js](https://nodejs.org/) v18+
- [Rust](https://www.rust-lang.org/) stable
- [7-Zip](https://www.7-zip.org/) (optionnel, pour les archives .7z)

## Build de production

```bash
npm run tauri build
```

Produit un installeur NSIS dans `src-tauri/target/release/bundle/nsis/`.

## Tech Stack

| Couche | Technologie |
|--------|-------------|
| Frontend | React 19 + TypeScript + Vite |
| Backend | Tauri 2 (Rust) |
| UI | CSS custom (glassmorphism) + Framer Motion + Lucide Icons |
| Auth & DB | Supabase (PostgreSQL + Auth + Storage) |
| CI/CD | GitHub Actions → NSIS installer signé |

## Structure du projet

```
EmuWorld/
├── src/                          # Frontend React
│   ├── App.tsx                   # Composant principal (UI complète)
│   ├── App.css                   # Design system (thèmes, glassmorphism)
│   ├── main.tsx                  # Point d'entrée React
│   ├── supabase.ts               # Client Supabase (auth, DB)
│   └── i18n/                     # Traductions FR/EN
├── src-tauri/                    # Backend Rust
│   ├── src/
│   │   ├── lib.rs                # Commandes Tauri (scan, covers, store, launch)
│   │   ├── emulators.rs          # Catalogue des émulateurs
│   │   ├── playtime.rs           # Tracking temps de jeu
│   │   ├── discord_rpc.rs        # Discord Rich Presence
│   │   ├── achievements.rs       # Système d'achievements
│   │   ├── cloud_backup.rs       # Backup saves vers Supabase
│   │   ├── retroachievements.rs  # API RetroAchievements
│   │   ├── gamepad.rs            # Config manette (gilrs)
│   │   └── main.rs               # Entry point Rust
│   ├── Cargo.toml                # Dépendances Rust
│   └── tauri.conf.json           # Config Tauri (fenêtre, plugins, updater)
├── web-panel/                    # Profil web public
│   ├── index.html                # Page profil partageable
│   └── auth-callback.html        # Callback OAuth
├── .github/workflows/
│   └── release.yml               # CI: build + sign + release sur tag v*
├── scratch/                      # Notes, SQL schemas, plans
├── EmuWorld.bat                  # Lanceur rapide (npm run tauri dev)
├── package.json                  # Config npm + scripts
└── CLAUDE.md                     # Instructions pour Claude Code
```

## Licence

MIT

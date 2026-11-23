# Portage Linux d'EmuWorld (.deb) — recherche du 2026-11-23

## Verdict

Faisable sans réécriture : Tauri v2, React, Supabase, reqwest, zip/7z, gilrs, notify, discord-rich-presence
sont multiplateformes. Le travail se concentre sur ~12 points Windows du backend + un catalogue d'émulateurs
par OS + un job CI Ubuntu. Aujourd'hui **le crate ne compile pas sous Linux** (`dpapi.rs` utilise `winapi` sans `#[cfg(windows)]`).

Cible : `.deb` x86_64 construit sur Ubuntu 22.04 (webkit2gtk-4.1 dispo), donc compatible Ubuntu 22.04+ / Debian 12+ / Mint 21+ / Pop!_OS.
Bonus quasi gratuit : Tauri produit aussi `.AppImage` et `.rpm` (`targets: ["deb","appimage"]`).

## 1. Code Rust spécifique Windows à adapter

| Endroit | Aujourd'hui | Linux |
|---|---|---|
| `dpapi.rs` (clés iiDB, SteamGridDB, RA, cloud) | `CryptProtectData` | `#[cfg]` : crate `keyring` (Secret Service / GNOME Keyring / KWallet), repli : fichier chiffré AES-GCM avec clé en `0600`. Garder le même API `encrypt_and_write` / `read_and_decrypt` |
| `Cargo.toml` | `winapi` toujours | `[target.'cfg(windows)'.dependencies] winapi = …` |
| `lib.rs:18` `emuworld_base_dir` | `dirs::data_local_dir()` | déjà OK → `~/.local/share/EmuWorld`. Mode portable : à garder |
| `lib.rs:261` hook clavier global (overlay Shift+Tab, raccourcis speedrun) | `SetWindowsHookEx` | X11 : `rdev` ou `tauri-plugin-global-shortcut` ; **Wayland interdit l'écoute globale** → repli sur global-shortcut (portail GlobalShortcuts KDE/GNOME 48+) ou bouton manette (gilrs marche) |
| `lib.rs:1642` suivi du processus relancé (RPCS3) | `tasklist` | lire `/proc/*/comm` (ou crate `sysinfo`) |
| `lib.rs:6733` capture d'écran | PowerShell + System.Drawing | crate `xcap` (X11 + Wayland via portail) ou `grim`/`gnome-screenshot` |
| `lib.rs:7034` `open_path` | `explorer` + garde-fous sous `cfg(windows)` (**sous Linux la fonction ne fait rien**) | sortir les garde-fous du `cfg`, ouvrir via `xdg-open` (ou `tauri-plugin-opener`, déjà présent) |
| `lib.rs:7005` `system_description` | `cmd /C ver` | lire `/etc/os-release` (`PRETTY_NAME`) |
| `savestates.rs:261` corbeille | `SHFileOperationW` | crate `trash` (spéc. freedesktop) — le repli `not(windows)` existe déjà, vérifier ce qu'il fait |
| `lib.rs:975` 7-Zip embarqué `7za.exe` | binaire dans `resources` | dépendance deb `7zip` (ou `p7zip-full` sur 22.04) + chercher `7zz`/`7z` dans le PATH |
| `lib.rs:1418` `chdman.exe` | binaire embarqué | dépendance deb `mame-tools` (fournit `chdman`) |
| `creation_flags(0x08000000)` | déjà sous `cfg(windows)` | rien |
| Noms en dur `retroarch.exe`, `Dolphin.exe`, `PPSSPPWindows64.exe`, `rpcs3.exe`… (`lib.rs`, `netplay.rs`, `savestates.rs`, `retroachievements.rs`, `cloud_backup.rs`) | | passer par `executable_name` du catalogue (une seule source de vérité), ou helper `exe_name("retroarch")` |
| `lib.rs:1765` repli « premier .exe » | | Linux : premier `.AppImage` / fichier exécutable (`mode & 0o111`) |
| Fichiers téléchargés | | `chmod +x` après extraction (AppImage, binaires des tar) — le zip perd souvent le bit exécutable |
| `cloud_backup.rs` dossiers de saves | chemins `%APPDATA%`/`Documents` | équivalents `~/.config/<emu>`, `~/.local/share/<emu>` ; les AppImage en mode portable si on crée `<nom>.AppImage.home`/`portable.txt` |

Rien à faire : Discord RPC (le crate utilise le socket `$XDG_RUNTIME_DIR/discord-ipc-0`), deep-link `emuworld://` (Tauri l'inscrit dans le `.desktop` du `.deb`), single-instance (D-Bus), notifications, gilrs (a besoin de `libudev` à la compilation).

Frontend : aucun chemin Windows en dur trouvé dans `App.tsx`. À vérifier : textes « Explorateur », titlebar custom (`decorations:false` marche, mais le redimensionnement par les bords sous Wayland peut demander `startResizeDragging`).

## 2. Émulateurs : version Linux de chacun (vérifié sur les releases le 2026-11-23)

| ID | Linux officiel | Format |
|---|---|---|
| mesen | `Mesen_2.1.1_Linux_x64.zip` | zip (nécessite .NET runtime ? — à tester, la build est self-contained d'après la doc) |
| mgba | `mGBA-0.10.5-appimage-x64.appimage` | AppImage |
| snes9x | `Snes9x-1.63-x86_64.AppImage` | AppImage |
| project64 | **aucun** (Windows only) | remplacer par **RMG** (`RMG-Portable-Linux64-v0.9.1.AppImage`, Mupen64Plus) ou RetroArch N64 |
| dolphin | AppImage maintenant publiés sur dl.dolphin-emu.org (dev builds `dolphin-master-…-x64.AppImage`) + flatpak | AppImage ; adapter `UpdateSource::DolphinEmu` |
| melonds | `melonDS-1.1-appimage-x86_64.zip` | zip contenant AppImage |
| azahar | `azahar.AppImage` (2126.2) | AppImage |
| cemu | `Cemu-2.6-x86_64.AppImage` | AppImage |
| ryubing | `ryujinx-1.3.3-linux_x64.tar.gz` (Forgejo) | tar.gz → **nouvel extracteur tar.gz** (crates `flate2` + `tar`) |
| lichii | `lichii-linux-x86_64-pgo.AppImage` (build-386) | AppImage |
| retroarch-* | `buildbot.libretro.com/stable/1.20.0/linux/x86_64/RetroArch.7z` (contient un AppImage) | cores : `nightly/linux/x86_64/latest/<core>_libretro.so.zip` (vérifié 200) → `core_name` `.dll` → `.so` |
| duckstation | `DuckStation-x64.AppImage` | AppImage |
| pcsx2 | `pcsx2-v2.8.2-linux-appimage-x64-Qt.AppImage` | AppImage |
| rpcs3 | repo `RPCS3/rpcs3-binaries-linux` → `rpcs3-…_linux64.AppImage` | AppImage, update source différente du Windows |
| ppsspp | `PPSSPP-v1.20.4-anylinux-x86_64.AppImage` | AppImage (exe ≠ `PPSSPPWindows64.exe`) |
| flycast | `flycast-x86_64-2.7.AppImage` | AppImage |
| xemu | `xemu-0.8.139-x86_64.AppImage` | AppImage |
| shadps4 | `shadps4-linux-sdl-0.21.0.zip` | zip (contient un AppImage) |
| kytyps5 | `KytyPS5-…-Linux-x86_64.tar.gz` | tar.gz |
| xenia | `xenia_canary_linux.AppImage` | AppImage — **xenia Linux est expérimental**, marquer « bêta » ou masquer |

→ 100 % du catalogue couvert sauf Project64 (remplaçable). Les AppImage simplifient : pas d'extraction, un `chmod +x` et on lance.
Prérequis système des AppImage : `libfuse2` (Ubuntu 22.04 : `libfuse2`, 24.04 : `libfuse2t64`) → à mettre en `Recommends` du .deb, ou lancer avec `APPIMAGE_EXTRACT_AND_RUN=1` pour s'en passer.

### Modèle de données proposé

Ne pas dupliquer tout le catalogue : ajouter à `EmulatorInfo` un bloc optionnel
`linux: Option<PlatformBuild { download_url, executable_name, archive_type: "appimage"|"zip"|"7z"|"tar.gz", core_name }>`
et une fonction `get_catalog()` qui applique la variante de l'OS courant (`cfg!(target_os = "linux")`).
Les émulateurs sans build Linux (`linux: None`) disparaissent de la liste sous Linux.
`update_info()` : sélectionner l'asset Linux (le sélecteur d'assets existant `pick(...)` a déjà un test avec « linux » — l'inverser selon l'OS).
Tests : un test « chaque émulateur Linux a une URL https, un archive_type connu, un exe sans .exe », un test « les cores Linux finissent par .so ».

## 3. Packaging .deb

- `tauri-plugin-updater` est en **2.14.1** : la mise à jour in-app des `.deb` est supportée (depuis 2.1.0 ; deb/rpm/AppImage tous gérés). Le `latest.json` doit contenir une entrée `linux-x86_64` (+ `linux-x86_64-deb`) — `tauri-action` la génère.
- `tauri.conf.json` :
  ```json
  "bundle": {
    "targets": ["nsis", "deb", "appimage"],
    "resources": { … par OS via tauri.linux.conf.json … },
    "linux": { "deb": {
      "depends": ["7zip | p7zip-full", "mame-tools"],
      "recommends": ["libfuse2t64 | libfuse2"],
      "section": "games"
    } }
  }
  ```
  Mettre ce qui est propre à Linux dans `src-tauri/tauri.linux.conf.json` (Tauri fusionne automatiquement) pour ne pas embarquer `7za.exe` dans le .deb.
- Le .deb dépend déjà automatiquement de `libwebkit2gtk-4.1-0` et `libgtk-3-0`.
- CSP / `assetProtocol.scope` : `$APPDATA` → sous Linux ça pointe vers `~/.config/com.emuworld.app` alors que les covers sont dans `~/.local/share/EmuWorld` → ajouter `$LOCALDATA/EmuWorld/**` (à vérifier aussi sous Windows, c'est peut-être déjà le cas qui marche par hasard).

## 4. CI

- `release.yml` : matrice `windows-latest` + `ubuntu-22.04`. Sur Ubuntu :
  `sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libudev-dev libssl-dev`
- Job de tests sur chaque push : ajouter `ubuntu-22.04` pour que `cargo test` attrape tout code Windows non protégé.
- Les deux jobs uploadent dans la même release ; `latest.json` fusionné (tauri-action le fait si `releaseId` partagé).

## 5. Ordre de travail proposé

1. **Compiler sous Linux** : `cfg` sur `winapi`/`dpapi`, stubs ; CI Ubuntu `cargo check` + `cargo test`. (petit, débloque tout)
2. Stockage des secrets Linux (`keyring`) + `open_path`, `system_description`, `tasklist`, corbeille.
3. Catalogue Linux + extraction tar.gz + AppImage (`chmod +x`, lancement) + cores `.so`.
4. Lancement / suivi de jeu / playtime / saves cloud / RetroAchievements (chemins de config RetroArch Linux).
5. Overlay & raccourcis (X11 d'abord, Wayland en repli) + capture d'écran.
6. Packaging `.deb` + AppImage + updater + job release.
7. Test réel dans une VM Ubuntu 24.04 (ou WSL2 + WSLg pour un premier essai rapide depuis ce PC).

## Points ouverts

- Tester sur une vraie machine : WSL2/WSLg suffit pour lancer l'app et un émulateur 2D ; pour Vulkan/manette, VM ou dual-boot.
- Wayland : overlay « par-dessus le jeu » impossible à garantir (pas de fenêtre toujours-au-dessus imposée) ; prévoir que l'overlay soit optionnel.
- Flatpak/Flathub plus tard ? Le bac à sable complique le lancement d'émulateurs téléchargés → .deb + AppImage d'abord.
- BIOS / clés téléchargés : identiques, aucun changement.

# EmuWorld — Plan de route

## 🚀 Ajouts

- [x] **Date / heure dans l'app** *(quick win ~10 min)* — horloge discrète dans la titlebar ou sidebar footer
- [x] **Discord Rich Presence** *(moyen)* — "Jouant à Mario Kart 8 via EmuWorld" avec le logo EmuWorld en grand (brand persistant) et l'icône de la console en petit. Idle = "Browsing the library · In the launcher"

  > ⚠️ Configuration requise côté Discord Developer Portal :
  > 1. App actuelle : ID par défaut `1334862011723284510` dans `src-tauri/src/discord_rpc.rs` — remplace par le tien si tu veux être propriétaire
  > 2. Uploader les assets avec ces keys exactes : `emuworld_logo` (grand), `playing_generic` (petit fallback), et `console_switch` / `console_wiiu` / `console_ps2` etc. (voir `console_icon_key` dans le module Rust pour la liste)
  > 3. Sans assets, les icônes ne s'affichent pas mais le texte lui fonctionne quand même.
- [x] **Manette (navigation UI au pad)** *(gros)* — Gilrs natif Rust, highlight focus, D-pad/stick → navigate, A → click/context menu, B → back, clavier virtuel, remap touches
- [x] **Achievements in-app** *(gros)* — palier par installation, nb de ROMs, heures jouées, jeu favori, 10 consoles, etc. Notif in-app + badges sur le profil web
- [x] **RetroAchievements par jeu** *(gros)* — wrapper de l'API [retroachievements.org](https://retroachievements.org), rattachement du compte, progression affichée sur la fiche jeu, auto-configuration des émulateurs (token injection + cores RetroArch), redirection automatique vers RetroArch pour les standalone sans support RA
- [x] **Executable partageable** — workflow GitHub Actions qui build + signe le `.exe`/`.msi` à chaque tag `v*`, release en prerelease, guide dans [scratch/release_guide.md](scratch/release_guide.md)
- [ ] **Système d'amis** *(hard)*
  - [ ] liste d'amis (table Supabase `friendships` avec statut pending/accepted)
  - [ ] présence : "x est en train de jouer à Y"
  - [ ] inviter dans un lobby
  - [ ] voir le profil
  - [ ] chat 1:1
- [ ] **Multijoueur / lobby** *(très hard, dépend des émulateurs)* — netplay natif supporté uniquement par certains émus (RetroArch, Dolphin, Citra). On pourrait wrapper ces features.
- [x] **Update Automatique** — plugin Tauri updater + signing key + workflow GitHub Actions. L'app check `latest.json` au boot, affiche un bouton "✨ Mise à jour" dans la titlebar, download + relance avec progress. Guide dans [scratch/release_guide.md](scratch/release_guide.md).
- [ ] **leaderboard in app**
- [x] **Backup cloud des saves** — *Techniquement faisable sans souci* : scanner le dossier saves de chaque émulateur connu, zipper par jeu, uploader sur Supabase Storage (~quelques MB par save). Versioning simple avec timestamp. **Limite** : le plan gratuit Supabase c'est 1 GB de Storage donc ça tient facilement tant que tu restes seul. Si tu veux pousser à des amis c'est peut-être 5 GB (~50 jeux × 100 MB) qui suffisent aussi. Pas lourd, juste chiadé côté UI.

## 🐛 Bugs

- [x] **Covers qui restent manquantes** sur certains jeux (lesquels déjà ? faudrait lister les noms) et bugs sur covers wii u
- [x] **OAuth Google / Discord** — `register_all()` au boot, page bounce sur alwaysdata pour que l'onglet se ferme proprement, playtime réinitialisé au switch de compte, et fix du listener `game-closed` qui tournait dans le vide.
- [x] **Lancement jeux NES** ne fonctionne pas — à investiguer : quel émulateur est mappé au NES, quelle erreur remonte ?
- [ ] **Chargement Infini** page de connexion — page bounce `/auth-callback.html` affiche un message "tu peux fermer cette fenêtre".
- [x] **Probleme lancement Cemu** lors du lancement d'une rom, cemu se lance bien masi pas la rom → fix: ajout flag `-g`
- [x] **Probleme installation roms wii/gamecube** lors du finalize, la rom n'est pas dezip et dolphin n'arrive pas a lire le .ciso → fix: extraction 7z séparée du ZIP + ajout extensions ciso/gcz/nkit.iso
- [x] **Probleme covers sites web**
- [ ] **Rendre tout controlable par manette**

## 💡 Idées en vrac (à rajouter au fil de l'eau)

- [ ] Session recap toast à la fermeture de l'émulateur ("Session de 2h34 · +3 launches · total 47h")
- [ ] Discover page avec jeu du jour, top friends, covers en parallax
- [ ] Mode "Versus" / challenge 7 jours entre amis
